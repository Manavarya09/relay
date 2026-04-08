//! Handoff package builder — compresses session state into a prompt
//! that any agent can pick up and continue from.

pub mod templates;

use crate::SessionSnapshot;
use anyhow::Result;

/// Build a handoff prompt from a session snapshot.
/// This is the formatted text that gets sent to the fallback agent.
pub fn build_handoff(
    snapshot: &SessionSnapshot,
    target_agent: &str,
    max_tokens: usize,
) -> Result<String> {
    let mut sections: Vec<String> = Vec::new();

    // ── Header ─────────────────────────────────────────────────
    let urgency = if let Some(ref deadline) = snapshot.deadline {
        format!("\n  DEADLINE     : {deadline}")
    } else {
        String::new()
    };

    sections.push(format!(
        "══ RELAY HANDOFF ══════════════════════════════
  Original agent : Claude Code
  Handed off at  : {}
  Target agent   : {}
  Project        : {}{}
══════════════════════════════════════════════",
        snapshot.timestamp,
        target_agent,
        snapshot.project_dir,
        urgency
    ));

    // ── Current Task ───────────────────────────────────────────
    sections.push(format!(
        "## CURRENT TASK\n\n{}",
        snapshot.current_task
    ));

    // ── Todos ──────────────────────────────────────────────────
    if !snapshot.todos.is_empty() {
        let mut todo_text = String::from("## PROGRESS\n\n");
        for t in &snapshot.todos {
            let icon = match t.status.as_str() {
                "completed"   => "done",
                "in_progress" => "IN PROGRESS",
                _             => "pending",
            };
            todo_text.push_str(&format!("  [{}] {}\n", icon, t.content));
        }
        sections.push(todo_text);
    }

    // ── Last Error ─────────────────────────────────────────────
    if let Some(ref err) = snapshot.last_error {
        sections.push(format!(
            "## LAST ERROR\n\n```\n{}\n```",
            truncate_smart(err, 500)
        ));
    }

    // ── Decisions ──────────────────────────────────────────────
    if !snapshot.decisions.is_empty() {
        let mut dec = String::from("## KEY DECISIONS\n\n");
        for d in &snapshot.decisions {
            dec.push_str(&format!("  - {d}\n"));
        }
        sections.push(dec);
    }

    // ── Git State ──────────────────────────────────────────────
    if let Some(ref git) = snapshot.git_state {
        let mut git_text = format!(
            "## GIT STATE\n\n  Branch: {}\n  Status: {}",
            git.branch, git.status_summary
        );
        if !git.recent_commits.is_empty() {
            git_text.push_str("\n\n  Recent commits:\n");
            for c in git.recent_commits.iter().take(5) {
                git_text.push_str(&format!("    {c}\n"));
            }
        }
        if !git.diff_summary.is_empty() {
            git_text.push_str(&format!(
                "\n  Diff summary:\n    {}",
                truncate_smart(&git.diff_summary, 500)
            ));
        }
        sections.push(git_text);
    }

    // ── Recent Files ───────────────────────────────────────────
    if !snapshot.recent_files.is_empty() {
        let mut files = String::from("## RECENTLY CHANGED FILES\n\n");
        for f in snapshot.recent_files.iter().take(20) {
            files.push_str(&format!("  {f}\n"));
        }
        sections.push(files);
    }

    // ── Full Conversation Context ──────────────────────────────
    if !snapshot.conversation.is_empty() {
        let mut convo = String::from("## CONVERSATION CONTEXT\n\nBelow is the full conversation from the Claude session (most recent turns).\nThis is the actual context that was in Claude's window when it was interrupted.\n\n");
        for turn in &snapshot.conversation {
            let prefix = match turn.role.as_str() {
                "user"           => "USER",
                "assistant"      => "CLAUDE",
                "assistant_tool" => "CLAUDE_TOOL",
                "tool_result"    => "TOOL_OUTPUT",
                _                => &turn.role,
            };
            convo.push_str(&format!("[{prefix}] {}\n\n", turn.content));
        }
        sections.push(convo);
    }

    // ── Instructions for agent ─────────────────────────────────
    let deadline_note = if snapshot.deadline.is_some() {
        "\nIMPORTANT: There is a DEADLINE. If the user asks you to continue, prioritise the current task."
    } else { "" };

    let instructions = format!(
        "## INSTRUCTIONS\n\n\
        You have been given the full context from a Claude Code session.\n\
        The context above shows what was being worked on, what decisions were made,\n\
        what files were changed, and what the last state was.\n\
        \n\
        DO NOT immediately start working on anything.\n\
        Instead, briefly confirm you have the context by saying something like:\n\
        \"Context restored from your Claude session. I can see you were working on [brief summary]. What would you like me to do?\"\n\
        \n\
        Then WAIT for the user to tell you what to do next.\n\
        \n\
        Working directory: {}\
        {}",
        snapshot.project_dir,
        deadline_note
    );
    sections.push(instructions);

    let full = sections.join("\n\n");

    // Smart compression: prioritize recent context over old
    let max_chars = (max_tokens as f64 * 3.5) as usize;
    if full.len() <= max_chars {
        return Ok(full);
    }

    // Priority-based compression: rebuild with budget awareness
    // Priority 1 (always keep): header, current task, last error, instructions
    // Priority 2 (high): recent conversation (last 5 turns), decisions, git state
    // Priority 3 (medium): older conversation, todos, recent files
    let mut budget = max_chars;
    let mut compressed = Vec::new();
    let mut dropped_sections = Vec::new();

    // Priority 1: header + task + error + instructions (always included)
    let p1_indices: Vec<usize> = vec![0, 1]; // header, task always first two
    let error_idx = sections.iter().position(|s| s.starts_with("## LAST ERROR"));
    let instr_idx = sections.iter().position(|s| s.starts_with("## INSTRUCTIONS"));

    for &idx in &p1_indices {
        if idx < sections.len() && sections[idx].len() < budget {
            budget -= sections[idx].len() + 2;
            compressed.push(sections[idx].clone());
        }
    }
    if let Some(idx) = error_idx {
        if sections[idx].len() < budget {
            budget -= sections[idx].len() + 2;
            compressed.push(sections[idx].clone());
        }
    }

    // Priority 2: git state, decisions
    let git_idx = sections.iter().position(|s| s.starts_with("## GIT STATE"));
    let dec_idx = sections.iter().position(|s| s.starts_with("## KEY DECISIONS"));

    for opt_idx in [git_idx, dec_idx] {
        if let Some(idx) = opt_idx {
            if sections[idx].len() < budget {
                budget -= sections[idx].len() + 2;
                compressed.push(sections[idx].clone());
            } else {
                dropped_sections.push("decisions/git (truncated)");
            }
        }
    }

    // Priority 3: conversation context — trim from beginning to fit
    let convo_idx = sections.iter().position(|s| s.starts_with("## CONVERSATION CONTEXT"));
    if let Some(idx) = convo_idx {
        let convo = &sections[idx];
        if convo.len() < budget {
            budget -= convo.len() + 2;
            compressed.push(convo.clone());
        } else if budget > 500 {
            // Fit what we can: take the end of conversation (most recent turns)
            let header = "## CONVERSATION CONTEXT\n\n[Earlier turns omitted to fit context budget]\n\n";
            let available = budget.saturating_sub(header.len() + 50);
            let start = convo.len().saturating_sub(available);
            // Find a safe char boundary
            let mut safe_start = start;
            while safe_start < convo.len() && !convo.is_char_boundary(safe_start) {
                safe_start += 1;
            }
            // Find the next line boundary for clean cut
            if let Some(nl) = convo[safe_start..].find('\n') {
                safe_start += nl + 1;
            }
            let trimmed = format!("{}{}", header, &convo[safe_start..]);
            budget -= trimmed.len() + 2;
            compressed.push(trimmed);
            dropped_sections.push("older conversation turns");
        } else {
            dropped_sections.push("conversation context");
        }
    }

    // Priority 4: todos, recent files
    let todo_idx = sections.iter().position(|s| s.starts_with("## PROGRESS"));
    let files_idx = sections.iter().position(|s| s.starts_with("## RECENTLY CHANGED"));

    for (opt_idx, name) in [(todo_idx, "todos"), (files_idx, "recent files")] {
        if let Some(idx) = opt_idx {
            if sections[idx].len() < budget {
                budget -= sections[idx].len() + 2;
                compressed.push(sections[idx].clone());
            } else {
                dropped_sections.push(name);
            }
        }
    }

    // Always add instructions at the end
    if let Some(idx) = instr_idx {
        compressed.push(sections[idx].clone());
    }

    let _ = budget; // suppress unused warning

    // Add compression note if sections were dropped
    if !dropped_sections.is_empty() {
        compressed.push(format!(
            "[Context compressed to fit {} token budget. Omitted: {}]",
            max_tokens,
            dropped_sections.join(", ")
        ));
    }

    let mut result = compressed.join("\n\n");

    // Final safety: hard truncate if still over budget
    if result.len() > max_chars {
        let mut end = max_chars;
        while end > 0 && !result.is_char_boundary(end) {
            end -= 1;
        }
        result.truncate(end);
        result.push_str("\n\n[...truncated to fit context limit]");
    }

    Ok(result)
}

/// Save handoff to a file for reference/debugging.
pub fn save_handoff(handoff: &str, project_dir: &std::path::Path) -> Result<std::path::PathBuf> {
    let relay_dir = project_dir.join(".relay");
    std::fs::create_dir_all(&relay_dir)?;

    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let path = relay_dir.join(format!("handoff_{ts}.md"));
    std::fs::write(&path, handoff)?;

    Ok(path)
}

fn truncate_smart(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    // Find a valid UTF-8 char boundary at or before `max` to avoid panicking
    // on multi-byte characters (e.g., non-ASCII file paths, emoji, CJK text).
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    // Try to cut at a line boundary for cleaner output
    let cut = &s[..end];
    if let Some(last_nl) = cut.rfind('\n') {
        format!("{}\n[...truncated]", &s[..last_nl])
    } else {
        format!("{}...", cut)
    }
}
