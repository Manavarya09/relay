//! Read TodoWrite state from Claude Code session.

use crate::TodoItem;
use std::path::Path;

/// Attempt to read the current todo list from the latest session transcript.
pub fn read_todos(project_dir: &Path) -> Vec<TodoItem> {
    // Reuse session module's path finding instead of duplicating it
    let claude_dir = super::session::find_claude_project_dir(project_dir);
    let Some(dir) = claude_dir else { return Vec::new() };
    let Some(jsonl_path) = find_latest_jsonl(&dir) else { return Vec::new() };

    parse_todos_from_jsonl(&jsonl_path)
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

fn parse_todos_from_jsonl(path: &Path) -> Vec<TodoItem> {
    let content = std::fs::read_to_string(path).unwrap_or_default();

    // Scan for the last TodoWrite tool_use call (reverse for efficiency)
    for line in content.lines().rev() {
        let Ok(val) = serde_json::from_str::<serde_json::Value>(line) else { continue };

        if let Some(content) = val.get("message").and_then(|m| m.get("content")).and_then(|c| c.as_array()) {
            for item in content {
                if item.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                    if item.get("name").and_then(|n| n.as_str()) == Some("TodoWrite") {
                        if let Some(input) = item.get("input") {
                            if let Some(todos) = input.get("todos").and_then(|t| t.as_array()) {
                                let last_todos: Vec<TodoItem> = todos.iter().filter_map(|t| {
                                    let content = t.get("content")?.as_str()?.to_string();
                                    let status = t.get("status")?.as_str()?.to_string();
                                    Some(TodoItem { content, status })
                                }).collect();
                                if !last_todos.is_empty() {
                                    return last_todos;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Vec::new()
}
