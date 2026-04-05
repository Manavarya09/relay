//! Handoff package builder — compresses session state into a prompt
//! that any agent can pick up and continue from.

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

    let mut full = sections.join("\n\n");

    // Hard cap at max_tokens (rough estimate: chars / 3.5)
    let max_chars = (max_tokens as f64 * 3.5) as usize;
    if full.len() > max_chars {
        // Find a valid UTF-8 char boundary
        let mut end = max_chars;
        while end > 0 && !full.is_char_boundary(end) {
            end -= 1;
        }
        full.truncate(end);
        full.push_str("\n\n[...truncated to fit context limit]");
    }

    Ok(full)
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
    // Try to cut at a line boundary
    let cut = &s[..max];
    if let Some(last_nl) = cut.rfind('\n') {
        format!("{}\n[...truncated]", &s[..last_nl])
    } else {
        format!("{}...", &s[..max])
    }
}
