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
use std::process::Command;

/// Cross-platform binary finder: uses `where` on Windows, `which` on Unix.
pub fn find_binary(name: &str) -> Option<String> {
    let cmd = if cfg!(target_os = "windows") { "where" } else { "which" };
    let output = Command::new(cmd).arg(name).output().ok()?;
    if output.status.success() {
        // `where` can return multiple lines; take the first
        let out = String::from_utf8_lossy(&output.stdout);
        Some(out.lines().next()?.trim().to_string())
    } else {
        None
    }
}

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
            "aider"    => agents.push(Box::new(aider::AiderAgent::new(&config.agents.aider))),
            "claude"   => agents.push(Box::new(claude::ClaudeAgent::new(&config.agents.claude))),
            "copilot"  => agents.push(Box::new(copilot::CopilotAgent::new(&config.agents.copilot))),
            "opencode" => agents.push(Box::new(opencode::OpenCodeAgent::new(&config.agents.opencode))),
            _ => {} // unknown agent, skip
        }
    }
    // Also load plugin agents
    agents.extend(crate::plugins::discover_plugins());

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

/// Execute handoff on a specific named agent, with optional chain fallback.
pub fn handoff_to_named(
    config: &Config,
    agent_name: &str,
    handoff_prompt: &str,
    project_dir: &str,
    chain: bool,
) -> Result<HandoffResult> {
    let agents = get_agents(config);
    for agent in &agents {
        if agent.name() == agent_name {
            let status = agent.check_available();
            if !status.available {
                if chain {
                    tracing::warn!("{} unavailable ({}), falling back...", agent_name, status.reason);
                    eprintln!("  \u{26a0}\u{fe0f}  {} unavailable. Trying next agent...", agent_name);
                    return handoff_to_first_available_skipping(config, agent_name, handoff_prompt, project_dir);
                }
                return Ok(HandoffResult {
                    agent: agent_name.into(),
                    success: false,
                    message: format!("{} is not available: {}", agent_name, status.reason),
                    handoff_file: None,
                });
            }
            let result = agent.execute(handoff_prompt, project_dir)?;
            if !result.success && chain {
                tracing::warn!("{} failed ({}), falling back...", agent_name, result.message);
                eprintln!("  \u{26a0}\u{fe0f}  {} failed. Trying next agent...", agent_name);
                return handoff_to_first_available_skipping(config, agent_name, handoff_prompt, project_dir);
            }
            return Ok(result);
        }
    }
    Ok(HandoffResult {
        agent: agent_name.into(),
        success: false,
        message: format!("Unknown agent: {agent_name}"),
        handoff_file: None,
    })
}

fn handoff_to_first_available_skipping(
    config: &Config,
    skip: &str,
    handoff_prompt: &str,
    project_dir: &str,
) -> Result<HandoffResult> {
    let agents = get_agents(config);
    for agent in &agents {
        if agent.name() == skip { continue; }
        let status = agent.check_available();
        if status.available {
            tracing::info!("Chain fallback: handing off to {}", agent.name());
            return agent.execute(handoff_prompt, project_dir);
        }
    }
    Ok(HandoffResult {
        agent: "none".into(),
        success: false,
        message: format!("No agents available after {} failed", skip),
        handoff_file: None,
    })
}
