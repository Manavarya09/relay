//! Aider agent adapter — launches the aider TUI interactively.
//! https://github.com/paul-gauthier/aider

use super::{Agent, find_binary};
use crate::{AgentStatus, HandoffResult};
use anyhow::Result;
use std::process::Command;

pub struct AiderAgent {
    model: String,
}

impl AiderAgent {
    pub fn new(model: Option<&str>) -> Self {
        Self {
            model: model.unwrap_or("sonnet").to_string(),
        }
    }
}

impl Agent for AiderAgent {
    fn name(&self) -> &str { "aider" }

    fn check_available(&self) -> AgentStatus {
        match find_binary("aider") {
            Some(path) => AgentStatus {
                name: "aider".into(),
                available: true,
                reason: format!("Found at {path}"),
                version: Command::new(&path).arg("--version").output().ok()
                    .filter(|o| o.status.success())
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()),
            },
            None => AgentStatus {
                name: "aider".into(),
                available: false,
                reason: "Not found. Install: pip install aider-chat".into(),
                version: None,
            },
        }
    }

    fn execute(&self, handoff_prompt: &str, project_dir: &str) -> Result<HandoffResult> {
        let binary = find_binary("aider").unwrap_or("aider".into());
        let tmp = std::env::temp_dir().join("relay_handoff.md");
        std::fs::write(&tmp, handoff_prompt)?;

        let status = Command::new(&binary)
            .current_dir(project_dir)
            .arg("--model")
            .arg(&self.model)
            .arg("--message")
            .arg(handoff_prompt)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .status()?;

        Ok(HandoffResult {
            agent: "aider".into(),
            success: status.success(),
            message: format!("Aider ({}) session ended", self.model),
            handoff_file: Some(tmp.to_string_lossy().to_string()),
        })
    }
}
