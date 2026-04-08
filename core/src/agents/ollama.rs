//! Ollama local agent adapter — uses the Ollama REST API.

use super::Agent;
use crate::{AgentStatus, HandoffResult, OllamaConfig};
use anyhow::Result;
use std::process::Command;

pub struct OllamaAgent {
    url: String,
    model: String,
}

impl OllamaAgent {
    pub fn new(config: &OllamaConfig) -> Self {
        Self {
            url: config.url.clone(),
            model: config.model.clone(),
        }
    }
}

impl Agent for OllamaAgent {
    fn name(&self) -> &str { "ollama" }

    fn check_available(&self) -> AgentStatus {
        // Use curl with a 2s timeout — avoids ureq hanging on refused connections
        let tag_url = format!("{}/api/tags", self.url);
        let output = Command::new("curl")
            .args(["--silent", "--max-time", "2", &tag_url])
            .output();

        match output {
            Ok(o) if o.status.success() => {
                let body: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap_or_default();
                let models = body.get("models").and_then(|m| m.as_array()).map(|a| a.len()).unwrap_or(0);
                AgentStatus {
                    name: "ollama".into(),
                    available: true,
                    reason: format!("Running at {}, {} models available", self.url, models),
                    version: Some(self.model.clone()),
                }
            }
            _ => AgentStatus {
                name: "ollama".into(),
                available: false,
                reason: format!("Not reachable at {}", self.url),
                version: None,
            },
        }
    }

    fn execute(&self, handoff_prompt: &str, _project_dir: &str) -> Result<HandoffResult> {
        let url = format!("{}/api/generate", self.url);
        let body = serde_json::json!({
            "model": self.model,
            "prompt": handoff_prompt,
            "stream": false
        });

        let retry_config = crate::retry::RetryConfig::default();
        let url_clone = url.clone();
        let body_clone = body.clone();

        let resp = match crate::retry::with_retry(&retry_config, || {
            ureq::post(&url_clone)
                .set("Content-Type", "application/json")
                .send_json(&body_clone)
        }) {
            Ok(resp) => resp,
            Err(ureq::Error::Status(code, resp)) => {
                let error_body = resp.into_string().unwrap_or_default();
                let api_msg = serde_json::from_str::<serde_json::Value>(&error_body)
                    .ok()
                    .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
                    .unwrap_or(error_body);
                return Ok(HandoffResult {
                    agent: "ollama".into(),
                    success: false,
                    message: format!("Ollama API error (HTTP {}): {}", code, api_msg),
                    handoff_file: None,
                });
            }
            Err(ureq::Error::Transport(t)) => {
                return Ok(HandoffResult {
                    agent: "ollama".into(),
                    success: false,
                    message: format!("Ollama unreachable: {}", t),
                    handoff_file: None,
                });
            }
        };

        let resp_json: serde_json::Value = resp.into_json()?;
        let text = resp_json.get("response").and_then(|r| r.as_str()).unwrap_or("(no response)");
        println!("{text}");

        Ok(HandoffResult {
            agent: "ollama".into(),
            success: true,
            message: format!("Ollama ({}) responded", self.model),
            handoff_file: None,
        })
    }
}
