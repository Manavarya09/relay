pub mod aider;
pub mod claude;
pub mod codex;
pub mod copilot;
pub mod gemini;
pub mod ollama;
pub mod openai;
pub mod opencode;

use crate::{AgentStatus, Config, HandoffResult};
use anyhow::Result;

/// Trait for all fallback agents.
pub trait Agent {
    fn name(&self) -> &str;
    fn check_available(&self) -> AgentStatus;
    fn execute(&self, handoff_prompt: &str, project_dir: &str) -> Result<HandoffResult>;
}

/// Get all configured agents in priority order.
pub fn get_agents(config: &Config) -> Vec<Box<dyn Agent>> {
    let mut agents: Vec<Box<dyn Agent>> = Vec::new();
    for name in &config.general.priority {
        match name.as_str() {
            "codex"    => agents.push(Box::new(codex::CodexAgent::new(&config.agents.codex))),
            "gemini"   => agents.push(Box::new(gemini::GeminiAgent::new(&config.agents.gemini))),
            "ollama"   => agents.push(Box::new(ollama::OllamaAgent::new(&config.agents.ollama))),
            "openai"   => agents.push(Box::new(openai::OpenAIAgent::new(&config.agents.openai))),
            "aider"    => agents.push(Box::new(aider::AiderAgent::new(None))),
            "claude"   => agents.push(Box::new(claude::ClaudeAgent::new())),
            "copilot"  => agents.push(Box::new(copilot::CopilotAgent::new())),
            "opencode" => agents.push(Box::new(opencode::OpenCodeAgent::new())),
            _ => {} // unknown agent, skip
        }
    }
    agents
}

/// Check availability of all agents and return statuses.
pub fn check_all_agents(config: &Config) -> Vec<AgentStatus> {
    get_agents(config).iter().map(|a| a.check_available()).collect()
}

/// Execute handoff on the first available agent.
pub fn handoff_to_first_available(
    config: &Config,
    handoff_prompt: &str,
    project_dir: &str,
) -> Result<HandoffResult> {
    let agents = get_agents(config);
    for agent in &agents {
        let status = agent.check_available();
        if status.available {
            tracing::info!("Handing off to {}", agent.name());
            return agent.execute(handoff_prompt, project_dir);
        }
    }
    Ok(HandoffResult {
        agent: "none".into(),
        success: false,
        message: "No agents available. Configure at least one in ~/.relay/config.toml".into(),
        handoff_file: None,
    })
}

/// Execute handoff on a specific named agent.
pub fn handoff_to_named(
    config: &Config,
    agent_name: &str,
    handoff_prompt: &str,
    project_dir: &str,
) -> Result<HandoffResult> {
    let agents = get_agents(config);
    for agent in &agents {
        if agent.name() == agent_name {
            let status = agent.check_available();
            if !status.available {
                return Ok(HandoffResult {
                    agent: agent_name.into(),
                    success: false,
                    message: format!("{} is not available: {}", agent_name, status.reason),
                    handoff_file: None,
                });
            }
            return agent.execute(handoff_prompt, project_dir);
        }
    }
    Ok(HandoffResult {
        agent: agent_name.into(),
        success: false,
        message: format!("Unknown agent: {agent_name}. Available: codex, gemini, ollama, openai"),
        handoff_file: None,
    })
}
