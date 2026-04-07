//! Read Claude Code session state from .jsonl transcript files.
//! Extracts the FULL conversation context — user messages, assistant reasoning,
//! tool calls with results, errors, and decisions.

use std::path::Path;
use std::sync::atomic::AtomicUsize;

/// Controls how many conversation turns to keep. Set before calling read_latest_session.
pub static MAX_CONVERSATION_TURNS: AtomicUsize = AtomicUsize::new(25);

/// Extracted session info from Claude's transcript.
pub struct SessionInfo {
    pub current_task: String,
    pub decisions: Vec<String>,
    pub last_error: Option<String>,
    pub last_output: Option<String>,
    /// Full conversation turns (compressed) — the real context
    pub conversation: Vec<ConversationTurn>,
}

/// A single turn in the conversation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationTurn {
    pub role: String, // "user", "assistant", "tool_result"
    pub content: String,
}

/// Read the latest Claude session transcript and extract full context.
pub fn read_latest_session(project_dir: &Path) -> SessionInfo {
    let claude_dir = find_claude_project_dir(project_dir);

    if let Some(dir) = claude_dir {
        if let Some(latest) = find_latest_jsonl(&dir) {
            return parse_session_transcript(&latest);
        }
    }

    // Fallback: infer from git state
    SessionInfo {
        current_task: infer_task_from_git(project_dir),
        decisions: Vec::new(),
        last_error: None,
        last_output: None,
        conversation: Vec::new(),
    }
}

pub fn find_claude_project_dir(project_dir: &Path) -> Option<std::path::PathBuf> {
    let home_dir = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))?;
    let claude_projects = std::path::PathBuf::from(home_dir).join(".claude/projects");
    if !claude_projects.exists() {
        return None;
    }

    // Claude encodes paths with / and \ as dashes: /Users/user/myproject -> -Users-user-myproject
    let proj_str = project_dir
        .to_string_lossy()
        .replace('/', "-")
        .replace('\\', "-");
    let candidate = claude_projects.join(&proj_str);
    if candidate.exists() {
        return Some(candidate);
    }

    // Try matching by suffix
    if let Ok(entries) = std::fs::read_dir(&claude_projects) {
        let dir_name = project_dir.file_name()?.to_string_lossy().to_string();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(&dir_name) && entry.path().is_dir() {
                return Some(entry.path());
            }
        }
    }

    None
}

fn find_latest_jsonl(dir: &std::path::Path) -> Option<std::path::PathBuf> {
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

fn parse_session_transcript(path: &std::path::Path) -> SessionInfo {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return default_session_info(),
    };

    let lines: Vec<&str> = content.lines().collect();

    let mut current_task = String::new();
    let mut decisions = Vec::new();
    let mut last_error = None;
    let mut last_output = None;
    let mut conversation: Vec<ConversationTurn> = Vec::new();

    for line in &lines {
        let Ok(val) = serde_json::from_str::<serde_json::Value>(line) else { continue };
        let msg_type = val.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let message = val.get("message").cloned().unwrap_or_default();
        let msg_content = message.get("content");

        match msg_type {
            // ── User messages ──────────────────────────────────────────
            "user" => {
                // Check if this is a tool result
                if let Some(tool_result) = val.get("toolUseResult") {
                    // Tool result content is in message.content[].content
                    if let Some(items) = msg_content.and_then(|c| c.as_array()) {
                        for item in items {
                            if item.get("type").and_then(|t| t.as_str()) == Some("tool_result") {
                                let result_text = item.get("content")
                                    .and_then(|c| {
                                        if let Some(s) = c.as_str() {
                                            Some(s.to_string())
                                        } else if let Some(arr) = c.as_array() {
                                            Some(arr.iter()
                                                .filter_map(|i| i.get("text").and_then(|t| t.as_str()))
                                                .collect::<Vec<_>>()
                                                .join("\n"))
                                        } else {
                                            None
                                        }
                                    })
                                    .unwrap_or_default();

                                if !result_text.is_empty() {
                                    // Check for errors
                                    let lower = result_text.to_lowercase();
                                    if lower.contains("error") || lower.contains("failed") ||
                                       lower.contains("panic") || lower.contains("exit code") {
                                        last_error = Some(truncate(&result_text, 800));
                                    }
                                    last_output = Some(truncate(&result_text, 800));

                                    conversation.push(ConversationTurn {
                                        role: "tool_result".into(),
                                        content: truncate(&result_text, 200),
                                    });
                                }
                            }
                        }
                    }
                    // Also check stdout/stderr from toolUseResult
                    let stdout = tool_result.get("stdout").and_then(|s| s.as_str()).unwrap_or("");
                    let stderr = tool_result.get("stderr").and_then(|s| s.as_str()).unwrap_or("");
                    if !stdout.is_empty() {
                        last_output = Some(truncate(stdout, 800));
                    }
                    if !stderr.is_empty() {
                        last_error = Some(truncate(stderr, 800));
                    }
                } else {
                    // Regular user message
                    let user_text = extract_text_content(msg_content);
                    if !user_text.is_empty() && user_text.len() > 3 {
                        // Update current task from the last substantive user message
                        if user_text.len() > 10 && !user_text.starts_with('/') {
                            current_task = truncate(&user_text, 500);
                        }
                        conversation.push(ConversationTurn {
                            role: "user".into(),
                            content: truncate(&user_text, 300),
                        });
                    }
                }
            }

            // ── Assistant messages ──────────────────────────────────────
            "assistant" => {
                if let Some(items) = msg_content.and_then(|c| c.as_array()) {
                    for item in items {
                        let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");
                        match item_type {
                            "text" => {
                                let text = item.get("text").and_then(|t| t.as_str()).unwrap_or("");
                                if !text.is_empty() {
                                    conversation.push(ConversationTurn {
                                        role: "assistant".into(),
                                        content: truncate(text, 200),
                                    });
                                    // Extract decisions
                                    for line in text.lines() {
                                        let trimmed = line.trim();
                                        if (trimmed.starts_with("I'll ") ||
                                            trimmed.starts_with("I chose") ||
                                            trimmed.starts_with("Using ") ||
                                            trimmed.starts_with("Decision:") ||
                                            trimmed.starts_with("Approach:") ||
                                            trimmed.starts_with("The fix is") ||
                                            trimmed.starts_with("The issue"))
                                            && trimmed.len() > 20
                                        {
                                            decisions.push(truncate(trimmed, 200));
                                        }
                                    }
                                }
                            }
                            "tool_use" => {
                                let name = item.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                                let input = item.get("input").cloned().unwrap_or_default();
                                let summary = summarize_tool_call(name, &input);
                                conversation.push(ConversationTurn {
                                    role: "assistant_tool".into(),
                                    content: summary,
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {} // skip system, queue-operation, etc.
        }
    }

    if current_task.is_empty() {
        current_task = "Could not determine current task from session transcript".into();
    }

    // Deduplicate decisions
    decisions.dedup();
    decisions.truncate(15);

    // Keep last N conversation turns — caller can override via max_conversation_turns
    let max_turns = MAX_CONVERSATION_TURNS.load(std::sync::atomic::Ordering::Relaxed);
    if conversation.len() > max_turns {
        let skip = conversation.len() - max_turns;
        conversation = conversation.into_iter().skip(skip).collect();
    }

    SessionInfo {
        current_task,
        decisions,
        last_error,
        last_output,
        conversation,
    }
}

/// Summarize a tool call into a human-readable line.
fn summarize_tool_call(name: &str, input: &serde_json::Value) -> String {
    match name {
        "Write" => {
            let path = input.get("file_path").and_then(|p| p.as_str()).unwrap_or("?");
            let content_len = input.get("content").and_then(|c| c.as_str()).map(|s| s.len()).unwrap_or(0);
            format!("[Write] {} ({} chars)", path, content_len)
        }
        "Edit" => {
            let path = input.get("file_path").and_then(|p| p.as_str()).unwrap_or("?");
            let old = input.get("old_string").and_then(|o| o.as_str()).unwrap_or("");
            format!("[Edit] {} (replacing {} chars)", path, old.len())
        }
        "Read" => {
            let path = input.get("file_path").and_then(|p| p.as_str()).unwrap_or("?");
            format!("[Read] {}", path)
        }
        "Bash" => {
            let cmd = input.get("command").and_then(|c| c.as_str()).unwrap_or("?");
            format!("[Bash] {}", truncate(cmd, 120))
        }
        "Glob" => {
            let pat = input.get("pattern").and_then(|p| p.as_str()).unwrap_or("?");
            format!("[Glob] {}", pat)
        }
        "Grep" => {
            let pat = input.get("pattern").and_then(|p| p.as_str()).unwrap_or("?");
            format!("[Grep] {}", pat)
        }
        "TodoWrite" => {
            let todos = input.get("todos").and_then(|t| t.as_array());
            let count = todos.map(|t| t.len()).unwrap_or(0);
            format!("[TodoWrite] {} items", count)
        }
        "Agent" => {
            let desc = input.get("description").and_then(|d| d.as_str()).unwrap_or("?");
            format!("[Agent] {}", desc)
        }
        _ => format!("[{}]", name),
    }
}

/// Extract text content from a message content field.
fn extract_text_content(content: Option<&serde_json::Value>) -> String {
    let Some(c) = content else { return String::new() };
    if let Some(s) = c.as_str() {
        return s.to_string();
    }
    if let Some(arr) = c.as_array() {
        let texts: Vec<&str> = arr.iter()
            .filter_map(|item| {
                if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                    item.get("text").and_then(|t| t.as_str())
                } else {
                    None
                }
            })
            .collect();
        return texts.join("\n");
    }
    String::new()
}

fn infer_task_from_git(project_dir: &Path) -> String {
    let output = std::process::Command::new("git")
        .current_dir(project_dir)
        .args(["log", "--oneline", "-1", "--no-decorate"])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let msg = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !msg.is_empty() {
                return format!("Recent work: {msg}");
            }
        }
    }

    "Unknown — no session transcript or git history found".into()
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    // Find a valid char boundary at or before max
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &s[..end])
}

fn default_session_info() -> SessionInfo {
    SessionInfo {
        current_task: "Could not read session transcript".into(),
        decisions: Vec::new(),
        last_error: None,
        last_output: None,
        conversation: Vec::new(),
    }
}
