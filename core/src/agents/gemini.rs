//! Gemini agent adapter — uses the Gemini API.

use super::Agent;
use crate::{AgentStatus, GeminiConfig, HandoffResult};
use anyhow::Result;

pub struct GeminiAgent {
    api_key: Option<String>,
    model: String,
}

impl GeminiAgent {
    pub fn new(config: &GeminiConfig) -> Self {
        let api_key = config.api_key.clone()
            .or_else(|| std::env::var("GEMINI_API_KEY").ok())
            .or_else(|| std::env::var("GOOGLE_API_KEY").ok());
        Self {
            api_key,
            model: config.model.clone(),
        }
    }
}

impl Agent for GeminiAgent {
    fn name(&self) -> &str { "gemini" }

    fn check_available(&self) -> AgentStatus {
        // First check if gemini CLI is available
        if let Ok(output) = std::process::Command::new("which").arg("gemini").output() {
            if output.status.success() {
                return AgentStatus {
                    name: "gemini".into(),
                    available: true,
                    reason: "Gemini CLI found in PATH".into(),
                    version: None,
                };
            }
        }

        match &self.api_key {
            Some(_) => AgentStatus {
                name: "gemini".into(),
                available: true,
                reason: format!("API key configured, model: {}", self.model),
                version: Some(self.model.clone()),
            },
            None => AgentStatus {
                name: "gemini".into(),
                available: false,
                reason: "No API key. Set GEMINI_API_KEY env var or add to config.toml".into(),
                version: None,
            },
        }
    }

    fn execute(&self, handoff_prompt: &str, project_dir: &str) -> Result<HandoffResult> {
        // Try Gemini CLI first
        if let Ok(output) = std::process::Command::new("which").arg("gemini").output() {
            if output.status.success() {
                let mut child = std::process::Command::new("gemini")
                    .current_dir(project_dir)
                    .arg(handoff_prompt)
                    .spawn()?;
                let _ = child.wait();
                return Ok(HandoffResult {
                    agent: "gemini".into(),
                    success: true,
                    message: "Gemini CLI launched with handoff context".into(),
                    handoff_file: None,
                });
            }
        }

        // Fall back to API
        let api_key = self.api_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No Gemini API key"))?;

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, api_key
        );

        let body = serde_json::json!({
            "contents": [{
                "parts": [{ "text": handoff_prompt }]
            }]
        });

        let resp = match ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_json(&body)
        {
            Ok(resp) => resp,
            Err(ureq::Error::Status(code, resp)) => {
                let error_body = resp.into_string().unwrap_or_default();
                let api_msg = serde_json::from_str::<serde_json::Value>(&error_body)
                    .ok()
                    .and_then(|v| v.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()).map(String::from))
                    .unwrap_or(error_body);
                return Ok(HandoffResult {
                    agent: "gemini".into(),
                    success: false,
                    message: format!("Gemini API error (HTTP {}): {}", code, api_msg),
                    handoff_file: None,
                });
            }
            Err(ureq::Error::Transport(t)) => {
                return Ok(HandoffResult {
                    agent: "gemini".into(),
                    success: false,
                    message: format!("Gemini API unreachable: {}", t),
                    handoff_file: None,
                });
            }
        };

        let resp_json: serde_json::Value = resp.into_json()?;

        let text = resp_json
            .get("candidates")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.get(0))
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("(no response)");

        println!("{text}");

        Ok(HandoffResult {
            agent: "gemini".into(),
            success: true,
            message: format!("Gemini ({}) responded to handoff", self.model),
            handoff_file: None,
        })
    }
}
