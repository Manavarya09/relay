use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

use relay::{agents, capture, handoff, tui, Config};

#[derive(Parser)]
#[command(
    name = "relay",
    about = "Relay — When Claude's rate limit hits, another agent picks up where you left off.",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output as JSON (no TUI)
    #[arg(long, global = true)]
    json: bool,

    /// Verbose logging
    #[arg(long, short, global = true)]
    verbose: bool,

    /// Project directory
    #[arg(long, global = true)]
    project: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Hand off current session to a fallback agent
    Handoff {
        /// Target agent (codex, claude, aider, gemini, copilot, opencode, ollama, openai)
        #[arg(long)]
        to: Option<String>,

        /// Set deadline urgency (e.g. "7pm", "30min")
        #[arg(long)]
        deadline: Option<String>,

        /// Just print the handoff — don't launch agent
        #[arg(long)]
        dry_run: bool,

        /// How many conversation turns to include (default: 25)
        #[arg(long, default_value = "25")]
        turns: usize,

        /// What to include: all, conversation, git, todos (comma-separated)
        #[arg(long, default_value = "all")]
        include: String,

        /// Copy handoff to clipboard instead of launching agent
        #[arg(long)]
        clipboard: bool,

        /// Handoff template: full (default), minimal, raw
        #[arg(long, default_value = "full")]
        template: String,
    },

    /// Show current session snapshot
    Status,

    /// List configured agents and availability
    Agents,

    /// Resume after rate limit resets — show what happened during handoff
    Resume,

    /// List past handoffs
    History {
        /// Number of entries to show
        #[arg(long, default_value = "10")]
        limit: usize,
    },

    /// Show what changed since the last handoff
    Diff,

    /// Generate default config at ~/.relay/config.toml
    Init,

    /// Validate config and test agent connectivity
    Validate,

    /// PostToolUse hook (auto-detect rate limits)
    Hook {
        #[arg(long, default_value = "unknown")]
        session: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let filter = if cli.verbose { "debug" } else { "warn" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter)),
        )
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();

    let project_dir = cli.project
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let config = Config::load().unwrap_or_else(|_| Config {
        general: Default::default(),
        agents: Default::default(),
    });

    match cli.command {
        // ═══════════════════════════════════════════════════════════════
        // HANDOFF
        // ═══════════════════════════════════════════════════════════════
        Commands::Handoff { to, deadline, dry_run, turns, include, clipboard, template } => {
            if !cli.json {
                tui::print_banner();
            }

            // Step 1: Capture
            let sp = if !cli.json { Some(tui::step(1, 3, "Capturing session state...")) } else { None };

            relay::capture::session::MAX_CONVERSATION_TURNS
                .store(turns, std::sync::atomic::Ordering::Relaxed);

            let mut snapshot = capture::capture_snapshot(&project_dir, deadline.as_deref())?;

            // Apply include filter
            let includes: Vec<&str> = include.split(',').map(|s| s.trim()).collect();
            if !includes.contains(&"all") {
                if !includes.contains(&"conversation") { snapshot.conversation.clear(); }
                if !includes.contains(&"git") { snapshot.git_state = None; snapshot.recent_files.clear(); }
                if !includes.contains(&"todos") { snapshot.todos.clear(); }
            }

            if let Some(sp) = sp { sp.finish_with_message("Session captured"); }

            // Step 2: Build handoff
            let sp = if !cli.json { Some(tui::step(2, 3, "Building handoff package...")) } else { None };

            // Resolve target agent
            let target_name = if let Some(ref name) = to {
                name.clone()
            } else if !cli.json && !dry_run {
                // Interactive agent selection
                if let Some(sp) = sp.as_ref() { sp.finish_with_message("Handoff built"); }

                let statuses = agents::check_all_agents(&config);
                let agent_list: Vec<(String, bool, String)> = statuses
                    .iter()
                    .map(|s| (s.name.clone(), s.available, s.reason.clone()))
                    .collect();

                match tui::select_agent(&agent_list) {
                    Some(name) => name,
                    None => {
                        eprintln!("  No agent selected.");
                        return Ok(());
                    }
                }
            } else {
                "auto".into()
            };

            // Build handoff using selected template
            let handoff_text = match handoff::templates::Template::parse(&template) {
                handoff::templates::Template::Minimal => {
                    handoff::templates::build_minimal(&snapshot, &target_name)
                }
                handoff::templates::Template::Raw => {
                    handoff::templates::build_raw(&snapshot)
                }
                handoff::templates::Template::Full => {
                    handoff::build_handoff(&snapshot, &target_name, config.general.max_context_tokens)?
                }
            };
            let handoff_path = handoff::save_handoff(&handoff_text, &project_dir)?;

            if let Some(sp) = sp { sp.finish_with_message("Handoff built"); }

            // JSON / dry-run output
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                    "snapshot": snapshot,
                    "handoff_text": handoff_text,
                    "handoff_file": handoff_path.to_string_lossy(),
                    "target_agent": target_name,
                }))?);
                return Ok(());
            }
            // Clipboard mode
            if clipboard {
                #[cfg(target_os = "macos")]
                {
                    use std::process::{Command, Stdio};
                    let mut child = Command::new("pbcopy")
                        .stdin(Stdio::piped())
                        .spawn()?;
                    if let Some(mut stdin) = child.stdin.take() {
                        use std::io::Write;
                        stdin.write_all(handoff_text.as_bytes())?;
                    }
                    child.wait()?;
                    eprintln!("  📋 Handoff copied to clipboard!");
                    eprintln!("  📄 Also saved: {}", handoff_path.display());
                }
                #[cfg(target_os = "windows")]
                {
                    use std::process::{Command, Stdio};
                    let mut child = Command::new("clip")
                        .stdin(Stdio::piped())
                        .spawn()?;
                    if let Some(mut stdin) = child.stdin.take() {
                        use std::io::Write;
                        stdin.write_all(handoff_text.as_bytes())?;
                    }
                    child.wait()?;
                    eprintln!("  📋 Handoff copied to clipboard!");
                    eprintln!("  📄 Also saved: {}", handoff_path.display());
                }
                #[cfg(all(unix, not(target_os = "macos")))]
                {
                    use std::process::{Command, Stdio};
                    // Try xclip first (X11), then wl-copy (Wayland)
                    let clipboard_result = if let Ok(mut child) = Command::new("xclip")
                        .arg("-selection")
                        .arg("clipboard")
                        .stdin(Stdio::piped())
                        .spawn()
                    {
                        if let Some(mut stdin) = child.stdin.take() {
                            use std::io::Write;
                            let _ = stdin.write_all(handoff_text.as_bytes());
                        }
                        child.wait().ok()
                    } else if let Ok(mut child) = Command::new("wl-copy")
                        .stdin(Stdio::piped())
                        .spawn()
                    {
                        if let Some(mut stdin) = child.stdin.take() {
                            use std::io::Write;
                            let _ = stdin.write_all(handoff_text.as_bytes());
                        }
                        child.wait().ok()
                    } else {
                        None
                    };

                    if clipboard_result.is_some() {
                        eprintln!("  📋 Handoff copied to clipboard!");
                        eprintln!("  📄 Also saved: {}", handoff_path.display());
                    } else {
                        eprintln!("  Clipboard tools not available (xclip or wl-copy required)");
                        eprintln!("  📄 Saved to: {}", handoff_path.display());
                    }
                }
                return Ok(());
            }
            if dry_run {
                println!("{handoff_text}");
                eprintln!();
                eprintln!("  📄 Saved: {}", handoff_path.display());
                return Ok(());
            }

            // Step 3: Launch agent
            let sp = tui::step(3, 3, &format!("Launching {}...", target_name));

            let result = if to.is_some() {
                agents::handoff_to_named(&config, &target_name, &handoff_text, &project_dir.to_string_lossy())
            } else {
                agents::handoff_to_first_available(&config, &handoff_text, &project_dir.to_string_lossy())
            }?;

            sp.finish_with_message(if result.success {
                format!("{} launched", target_name)
            } else {
                "Failed".into()
            });

            if result.success {
                tui::print_handoff_success(&result.agent, &handoff_path.to_string_lossy());
            } else {
                tui::print_handoff_fail(&result.message, &handoff_path.to_string_lossy());
            }
        }

        // ═══════════════════════════════════════════════════════════════
        // STATUS
        // ═══════════════════════════════════════════════════════════════
        Commands::Status => {
            let sp = if !cli.json { Some(tui::spinner("Reading session state...")) } else { None };
            let snapshot = capture::capture_snapshot(&project_dir, None)?;
            if let Some(sp) = sp { sp.finish_and_clear(); }

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&snapshot)?);
            } else {
                tui::print_snapshot(&snapshot);
            }
        }

        // ═══════════════════════════════════════════════════════════════
        // AGENTS
        // ═══════════════════════════════════════════════════════════════
        Commands::Agents => {
            let sp = if !cli.json { Some(tui::spinner("Checking agents...")) } else { None };
            let statuses = agents::check_all_agents(&config);
            if let Some(sp) = sp { sp.finish_and_clear(); }

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&statuses)?);
            } else {
                tui::print_agents(&config.general.priority, &statuses);
            }
        }

        // ═══════════════════════════════════════════════════════════════
        // RESUME
        // ═══════════════════════════════════════════════════════════════
        Commands::Resume => {
            let report = relay::resume::build_resume(&project_dir)?;

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                tui::print_section("🔄", "Resume — What Happened During Handoff");
                eprintln!("  Handoff at: {}", report.handoff_time);
                eprintln!("  Task: {}", report.original_task);
                eprintln!();

                if !report.new_commits.is_empty() {
                    tui::print_section("📝", &format!("New Commits ({})", report.new_commits.len()));
                    for c in &report.new_commits {
                        eprintln!("  {c}");
                    }
                }

                if !report.changes_since.is_empty() {
                    tui::print_section("📄", &format!("Changed Files ({})", report.changes_since.len()));
                    for f in &report.changes_since {
                        eprintln!("  {f}");
                    }
                }

                if !report.diff_stat.is_empty() {
                    tui::print_section("📊", "Diff Summary");
                    for line in report.diff_stat.lines() {
                        eprintln!("  {line}");
                    }
                }

                eprintln!();
                eprintln!("  📋 Resume prompt ready. Use --json to get the full prompt.");
            }
        }

        // ═══════════════════════════════════════════════════════════════
        // HISTORY
        // ═══════════════════════════════════════════════════════════════
        Commands::History { limit } => {
            let entries = relay::history::list_handoffs(&project_dir, limit)?;

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&entries)?);
                return Ok(());
            }

            if entries.is_empty() {
                eprintln!("  No handoffs recorded yet.");
                return Ok(());
            }

            tui::print_section("📜", &format!("Handoff History ({} entries)", entries.len()));
            eprintln!();
            for e in &entries {
                eprintln!(
                    "  {}  → {:<10}  {}",
                    e.timestamp, e.agent, e.task
                );
            }
            eprintln!();
        }

        // ═══════════════════════════════════════════════════════════════
        // DIFF
        // ═══════════════════════════════════════════════════════════════
        Commands::Diff => {
            let report = relay::diff::diff_since_handoff(&project_dir)?;

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
                return Ok(());
            }

            tui::print_section("📊", "Changes Since Last Handoff");
            eprintln!("  Handoff at: {}", report.handoff_time);
            eprintln!(
                "  {} added, {} modified, {} deleted",
                report.files_added, report.files_modified, report.files_deleted
            );

            if !report.new_commits.is_empty() {
                eprintln!();
                eprintln!("  Commits:");
                for c in &report.new_commits {
                    eprintln!("    {c}");
                }
            }

            if !report.diff_stat.is_empty() {
                eprintln!();
                for line in report.diff_stat.lines() {
                    eprintln!("  {line}");
                }
            }
            eprintln!();
        }

        // ═══════════════════════════════════════════════════════════════
        // INIT
        // ═══════════════════════════════════════════════════════════════
        Commands::Init => {
            let path = relay::config_path();
            if path.exists() {
                eprintln!("  Config exists: {}", path.display());
                eprintln!("  Edit to add API keys and customize priority.");
            } else {
                Config::save_default(&path)?;
                eprintln!("  ✅ Config created: {}", path.display());
                eprintln!();
                eprintln!("  Add API keys:");
                eprintln!("    [agents.gemini]");
                eprintln!("    api_key = \"your-key\"");
                eprintln!();
                eprintln!("    [agents.openai]");
                eprintln!("    api_key = \"your-key\"");
            }
        }

        // ═══════════════════════════════════════════════════════════════
        // VALIDATE
        // ═══════════════════════════════════════════════════════════════
        Commands::Validate => {
            if !cli.json {
                eprintln!();
                eprintln!("  {}  {}", "🔍", "Validating Relay Configuration".bold());
                eprintln!("  {}", "─".repeat(50).dimmed());
                eprintln!();
            }

            let results = relay::validate::validate_config(&config);

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&results)?);
                return Ok(());
            }

            let mut all_ok = true;
            for r in &results {
                let (icon, status_str) = match r.status.as_str() {
                    "ok" => ("✅", r.message.green().to_string()),
                    "warn" => { all_ok = false; ("⚠️ ", r.message.yellow().to_string()) },
                    _ => { all_ok = false; ("❌", r.message.red().to_string()) },
                };
                eprintln!("  {} {:<12} {}", icon, r.agent.bold(), status_str);
            }

            eprintln!();
            if all_ok {
                eprintln!("  {} All agents validated successfully!", "🚀".to_string());
            } else {
                eprintln!("  {} Some agents need attention. Run 'relay init' to configure.", "💡".to_string());
            }
            eprintln!();
        }

        // ═══════════════════════════════════════════════════════════════
        // HOOK
        // ═══════════════════════════════════════════════════════════════
        Commands::Hook { session: _ } => {
            use std::io::Read;
            let mut raw = String::new();
            std::io::stdin().read_to_string(&mut raw)?;

            if let Some(detection) = relay::detect::check_hook_output(&raw) {
                eprintln!(
                    "  🚨 Rate limit detected in {} (signal: {})",
                    detection.tool_name, detection.signal
                );
                if config.general.auto_handoff {
                    let snapshot = capture::capture_snapshot(&project_dir, None)?;
                    let handoff_text = handoff::build_handoff(&snapshot, "auto", config.general.max_context_tokens)?;
                    let handoff_path = handoff::save_handoff(&handoff_text, &project_dir)?;
                    let result = agents::handoff_to_first_available(
                        &config, &handoff_text, &project_dir.to_string_lossy(),
                    )?;
                    if result.success {
                        eprintln!("  ✅ Auto-handed off to {}", result.agent);
                    } else {
                        eprintln!("  📄 Saved: {}", handoff_path.display());
                    }
                }
            }
            print!("{raw}");
        }
    }

    Ok(())
}
