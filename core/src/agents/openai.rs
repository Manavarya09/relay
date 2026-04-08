//! OpenAI GPT agent adapter.

use super::Agent;
use crate::{AgentStatus, HandoffResult, OpenAIConfig};
use anyhow::Result;

pub struct OpenAIAgent {
    api_key: Option<String>,
    model: String,
}

impl OpenAIAgent {
    pub fn new(config: &OpenAIConfig) -> Self {
        let api_key = config.api_key.clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok());
        Self {
            api_key,
            model: config.model.clone(),
        }
    }
}

impl Agent for OpenAIAgent {
    fn name(&self) -> &str { "openai" }

    fn check_available(&self) -> AgentStatus {
        match &self.api_key {
            Some(_) => AgentStatus {
                name: "openai".into(),
                available: true,
                reason: format!("API key configured, model: {}", self.model),
                version: Some(self.model.clone()),
            },
            None => AgentStatus {
                name: "openai".into(),
                available: false,
                reason: "No API key. Set OPENAI_API_KEY env var or add to config.toml".into(),
                version: None,
            },
        }
    }

    fn execute(&self, handoff_prompt: &str, _project_dir: &str) -> Result<HandoffResult> {
        let api_key = self.api_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No OpenAI API key"))?;

        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are a coding assistant picking up work from a Claude Code session that hit its rate limit. Follow the handoff instructions precisely. Be efficient and direct — the user has a deadline."
                },
                {
                    "role": "user",
                    "content": handoff_prompt
                }
            ],
            "max_tokens": 4096
        });

        let retry_config = crate::retry::RetryConfig::default();
        let api_key_clone = api_key.clone();
        let body_clone = body.clone();

        let resp = match crate::retry::with_retry(&retry_config, || {
            ureq::post("https://api.openai.com/v1/chat/completions")
                .set("Authorization", &format!("Bearer {}", api_key_clone))
                .set("Content-Type", "application/json")
                .send_json(&body_clone)
        }) {
            Ok(resp) => resp,
            Err(ureq::Error::Status(code, resp)) => {
                let error_body = resp.into_string().unwrap_or_default();
                let api_msg = serde_json::from_str::<serde_json::Value>(&error_body)
                    .ok()
                    .and_then(|v| v.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()).map(String::from))
                    .unwrap_or(error_body);
                return Ok(HandoffResult {
                    agent: "openai".into(),
                    success: false,
                    message: format!("OpenAI API error (HTTP {}): {}", code, api_msg),
                    handoff_file: None,
                });
            }
            Err(ureq::Error::Transport(t)) => {
                return Ok(HandoffResult {
                    agent: "openai".into(),
                    success: false,
                    message: format!("OpenAI API unreachable: {}", t),
                    handoff_file: None,
                });
            }
        };

        let resp_json: serde_json::Value = resp.into_json()?;
        let text = resp_json
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("(no response)");

        println!("{text}");

        Ok(HandoffResult {
            agent: "openai".into(),
            success: true,
            message: format!("OpenAI ({}) responded", self.model),
            handoff_file: None,
        })
    }
}
