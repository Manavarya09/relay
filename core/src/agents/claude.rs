//! Claude CLI agent — starts a new Claude Code session with full context.
//! Useful when the rate limit resets or you have a second subscription.

use super::{Agent, find_binary};
use crate::{AgentStatus, HandoffResult};
use anyhow::Result;
use std::process::Command;

#[derive(Default)]
pub struct ClaudeAgent;

impl ClaudeAgent {
    pub fn new() -> Self { Self }
}

impl Agent for ClaudeAgent {
    fn name(&self) -> &str { "claude" }

    fn check_available(&self) -> AgentStatus {
        match find_binary("claude") {
            Some(path) => AgentStatus {
                name: "claude".into(),
                available: true,
                reason: format!("Found at {path}"),
                version: Command::new(&path).arg("--version").output().ok()
                    .filter(|o| o.status.success())
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()),
            },
            None => AgentStatus {
                name: "claude".into(),
                available: false,
                reason: "Not found. Install: npm install -g @anthropic-ai/claude-code".into(),
                version: None,
            },
        }
    }

    fn execute(&self, handoff_prompt: &str, project_dir: &str) -> Result<HandoffResult> {
        let binary = find_binary("claude").unwrap_or("claude".into());
        let tmp = std::env::temp_dir().join("relay_handoff.md");
        std::fs::write(&tmp, handoff_prompt)?;

        // Launch Claude interactively with the handoff as initial prompt
        let status = Command::new(&binary)
            .current_dir(project_dir)
            .arg("--resume")
            .arg(handoff_prompt)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()?;

        Ok(HandoffResult {
            agent: "claude".into(),
            success: status.success(),
            message: "Claude session ended".into(),
            handoff_file: Some(tmp.to_string_lossy().to_string()),
        })
    }
}
