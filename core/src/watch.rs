//! `relay watch` — Daemon mode that monitors Claude's session for rate limits.
//! Polls the JSONL transcript and auto-hands off when a rate limit is detected.

use anyhow::Result;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct WatchConfig {
    pub poll_interval: Duration,
    pub cooldown: Duration,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(5),
            cooldown: Duration::from_secs(120), // 2 min between auto-handoffs
        }
    }
}

/// Run the watch loop. Blocks until Ctrl-C.
pub fn run_watch(
    project_dir: &Path,
    config: &crate::Config,
    watch_config: &WatchConfig,
) -> Result<()> {
    eprintln!("  \u{1f440} Watching for rate limits...");
    eprintln!("  Poll: {}s | Cooldown: {}s",
        watch_config.poll_interval.as_secs(),
        watch_config.cooldown.as_secs()
    );
    eprintln!("  Press Ctrl-C to stop.\n");

    // Graceful shutdown via signal handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).ok();

    let watch_start = Instant::now();
    let mut handoff_count: u32 = 0;
    let mut last_handoff: Option<Instant> = None;
    let mut last_size: u64 = 0;

    while running.load(Ordering::SeqCst) {
        std::thread::sleep(watch_config.poll_interval);

        // Find latest JSONL
        let session_dir = crate::capture::session::find_claude_project_dir(project_dir);
        let Some(dir) = session_dir else { continue };

        let jsonl_path = find_latest_jsonl(&dir);
        let Some(path) = jsonl_path else { continue };

        // Only check new content (incremental)
        let meta = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        let current_size = meta.len();
        if current_size <= last_size {
            continue; // No new content
        }

        // Read only the new bytes
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Check last few lines for rate limit signals
        let tail: String = content.lines().rev().take(5).collect::<Vec<_>>().join("\n");
        if !crate::detect::is_rate_limited(&tail) {
            last_size = current_size;
            continue;
        }

        // Cooldown check
        if let Some(last) = last_handoff {
            if last.elapsed() < watch_config.cooldown {
                eprintln!("  \u{23f3} Rate limit detected but cooldown active ({:.0}s remaining)",
                    (watch_config.cooldown - last.elapsed()).as_secs_f64());
                last_size = current_size;
                continue;
            }
        }

        // RATE LIMIT DETECTED — auto handoff!
        eprintln!("\n  \u{1f6a8} Rate limit detected! Auto-handing off...\n");

        let snapshot = match crate::capture::capture_snapshot(project_dir, None) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  \u{274c} Capture failed: {e}");
                last_size = current_size;
                continue;
            }
        };

        let handoff_text = match crate::handoff::build_handoff(&snapshot, "auto", config.general.max_context_tokens) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("  \u{274c} Handoff build failed: {e}");
                last_size = current_size;
                continue;
            }
        };

        let handoff_path = crate::handoff::save_handoff(&handoff_text, project_dir)
            .unwrap_or_default();

        // Try chain handoff
        let result = handoff_with_chain(config, &handoff_text, &project_dir.to_string_lossy());

        if result.success {
            handoff_count += 1;
            eprintln!("  \u{2705} Auto-handed off to {}", result.agent);
            if !handoff_path.as_os_str().is_empty() {
                eprintln!("  \u{1f4c4} Saved: {}", handoff_path.display());
            }

            // Record in analytics
            if let Ok(db) = crate::analytics::open_db() {
                let _ = crate::analytics::record_handoff(
                    &db, &result.agent, true, 0,
                    handoff_text.len(), handoff_text.len() / 4,
                    "full", &project_dir.to_string_lossy(),
                    &snapshot.current_task, None, result.chain_depth,
                );
            }
        } else {
            eprintln!("  \u{274c} All agents failed: {}", result.message);
            eprintln!("  \u{1f4c4} Context saved: {}", handoff_path.display());
        }

        last_handoff = Some(Instant::now());
        last_size = current_size;
    }

    // Graceful shutdown summary
    let elapsed = watch_start.elapsed();
    eprintln!("\n  \u{1f6d1} Watch stopped.");
    eprintln!("  Uptime: {}m {}s | Handoffs: {}",
        elapsed.as_secs() / 60, elapsed.as_secs() % 60, handoff_count);

    Ok(())
}

/// Handoff with chain — try each agent in priority order.
pub fn handoff_with_chain(
    config: &crate::Config,
    handoff_text: &str,
    project_dir: &str,
) -> ChainResult {
    let agents = crate::agents::get_agents(config);
    let mut chain_depth = 0u32;

    for agent in &agents {
        let status = agent.check_available();
        if !status.available {
            continue;
        }

        chain_depth += 1;
        eprintln!("  [{chain_depth}] Trying {}...", agent.name());

        match agent.execute(handoff_text, project_dir) {
            Ok(result) if result.success => {
                return ChainResult {
                    agent: result.agent,
                    success: true,
                    message: result.message,
                    chain_depth,
                };
            }
            Ok(result) => {
                eprintln!("  [{chain_depth}] {} failed: {}", agent.name(), result.message);
            }
            Err(e) => {
                eprintln!("  [{chain_depth}] {} error: {e}", agent.name());
            }
        }
    }

    ChainResult {
        agent: "none".into(),
        success: false,
        message: "All agents in chain exhausted".into(),
        chain_depth,
    }
}

pub struct ChainResult {
    pub agent: String,
    pub success: bool,
    pub message: String,
    pub chain_depth: u32,
}

fn find_latest_jsonl(dir: &Path) -> Option<std::path::PathBuf> {
    let mut newest: Option<(std::path::PathBuf, std::time::SystemTime)> = None;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                if let Ok(meta) = path.metadata() {
                    if let Ok(modified) = meta.modified() {
                        if newest.as_ref().map_or(true, |(_, t)| modified > *t) {
                            newest = Some((path, modified));
                        }
                    }
                }
            }
        }
    }
    newest.map(|(p, _)| p)
}
