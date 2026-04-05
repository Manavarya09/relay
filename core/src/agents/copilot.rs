//! GitHub Copilot CLI agent adapter.

use super::Agent;
use crate::{AgentStatus, HandoffResult};
use anyhow::Result;
use std::process::Command;

pub struct CopilotAgent;

impl CopilotAgent {
    pub fn new() -> Self { Self }

    fn find_binary() -> Option<String> {
        let output = Command::new("which").arg("copilot").output().ok()?;
        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            None
        }
    }
}

impl Agent for CopilotAgent {
    fn name(&self) -> &str { "copilot" }

    fn check_available(&self) -> AgentStatus {
        match Self::find_binary() {
            Some(path) => AgentStatus {
                name: "copilot".into(),
                available: true,
                reason: format!("Found at {path}"),
                version: None, // copilot --version hangs, skip it
            },
            None => AgentStatus {
                name: "copilot".into(),
                available: false,
                reason: "Not found. Install: gh extension install github/gh-copilot".into(),
                version: None,
            },
        }
    }

    fn execute(&self, handoff_prompt: &str, project_dir: &str) -> Result<HandoffResult> {
        let binary = Self::find_binary().unwrap_or("copilot".into());
        let tmp = std::env::temp_dir().join("relay_handoff.md");
        std::fs::write(&tmp, handoff_prompt)?;

        let status = Command::new(&binary)
            .current_dir(project_dir)
            .arg(handoff_prompt)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()?;

        Ok(HandoffResult {
            agent: "copilot".into(),
            success: status.success(),
            message: "Copilot session ended".into(),
            handoff_file: Some(tmp.to_string_lossy().to_string()),
        })
    }
}
