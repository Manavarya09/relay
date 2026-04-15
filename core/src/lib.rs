pub mod agents;
pub mod analytics;
pub mod capture;
pub mod clean;
pub mod cost;
pub mod detect;
pub mod diff;
pub mod handoff;
pub mod history;
pub mod plugins;
pub mod replay;
pub mod resume;
pub mod retry;
pub mod scoring;
pub mod secrets;
pub mod sessions;
pub mod tui;
pub mod validate;
pub mod watch;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ─── Config ──────────────────────────────────────────────────────────────────

/// Relay configuration — loaded from ~/.relay/config.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub agents: AgentsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Priority order for fallback agents
    #[serde(default = "default_priority")]
    pub priority: Vec<String>,
    /// Max tokens for handoff context
    #[serde(default = "default_max_context")]
    pub max_context_tokens: usize,
    /// Auto-handoff on rate limit detection
    #[serde(default = "default_true")]
    pub auto_handoff: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            priority: default_priority(),
            max_context_tokens: 8000,
            auto_handoff: true,
        }
    }
}

fn default_priority() -> Vec<String> {
    vec![
        "codex".into(),
        "claude".into(),
        "aider".into(),
        "gemini".into(),
        "copilot".into(),
        "opencode".into(),
        "ollama".into(),
        "openai".into(),
    ]
}
fn default_max_context() -> usize { 8000 }
fn default_true() -> bool { true }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentsConfig {
    #[serde(default)]
    pub codex: CodexConfig,
    #[serde(default)]
    pub claude: ClaudeConfig,
    #[serde(default)]
    pub aider: AiderConfig,
    #[serde(default)]
    pub gemini: GeminiConfig,
    #[serde(default)]
    pub copilot: CopilotConfig,
    #[serde(default)]
    pub opencode: OpenCodeConfig,
    #[serde(default)]
    pub ollama: OllamaConfig,
    #[serde(default)]
    pub openai: OpenAIConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodexConfig {
    /// Path to codex CLI binary (default: search PATH)
    pub binary: Option<String>,
    /// Model to use
    #[serde(default = "codex_default_model")]
    pub model: String,
}
fn codex_default_model() -> String { "o4-mini".into() }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeminiConfig {
    pub api_key: Option<String>,
    #[serde(default = "gemini_default_model")]
    pub model: String,
}
fn gemini_default_model() -> String { "gemini-2.5-pro".into() }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OllamaConfig {
    #[serde(default = "ollama_default_url")]
    pub url: String,
    #[serde(default = "ollama_default_model")]
    pub model: String,
}
fn ollama_default_url() -> String { "http://localhost:11434".into() }
fn ollama_default_model() -> String { "llama3".into() }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenAIConfig {
    pub api_key: Option<String>,
    #[serde(default = "openai_default_model")]
    pub model: String,
}
fn openai_default_model() -> String { "gpt-4o".into() }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClaudeConfig {
    pub binary: Option<String>,
    #[serde(default = "default_true")]
    pub resume: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AiderConfig {
    #[serde(default = "aider_default_model")]
    pub model: String,
}
fn aider_default_model() -> String { "sonnet".into() }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CopilotConfig {
    pub binary: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenCodeConfig {
    pub binary: Option<String>,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let path = config_path();
        if path.exists() {
            let text = std::fs::read_to_string(&path)?;
            Ok(toml::from_str(&text)?)
        } else {
            Ok(Self {
                general: GeneralConfig::default(),
                agents: AgentsConfig::default(),
            })
        }
    }

    pub fn save_default(path: &std::path::Path) -> anyhow::Result<()> {
        let cfg = Self {
            general: GeneralConfig::default(),
            agents: AgentsConfig::default(),
        };
        let text = toml::to_string_pretty(&cfg)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, text)?;
        Ok(())
    }
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir())
}

pub fn config_path() -> PathBuf {
    home_dir().join(".relay/config.toml")
}

pub fn data_dir() -> PathBuf {
    home_dir().join(".relay")
}

// ─── Core Types ──────────────────────────────────────────────────────────────

/// The state of a session at time of handoff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    /// What the user was working on (extracted from conversation)
    pub current_task: String,
    /// Todo items and their states
    pub todos: Vec<TodoItem>,
    /// Key decisions made in the session
    pub decisions: Vec<String>,
    /// Recent errors or blockers
    pub last_error: Option<String>,
    /// Last tool output (truncated)
    pub last_output: Option<String>,
    /// Git state: branch, diff summary, recent commits
    pub git_state: Option<GitState>,
    /// Project directory
    pub project_dir: String,
    /// Files recently edited
    pub recent_files: Vec<String>,
    /// When the snapshot was taken
    pub timestamp: String,
    /// Deadline if set
    pub deadline: Option<String>,
    /// FULL conversation context from Claude session transcript
    pub conversation: Vec<ConversationTurn>,
}

/// A single turn in the conversation (user message, assistant text, or tool call/result).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub content: String,
    pub status: String, // pending, in_progress, completed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitState {
    pub branch: String,
    pub status_summary: String,
    pub recent_commits: Vec<String>,
    pub diff_summary: String,
    pub uncommitted_files: Vec<String>,
}

/// Result of a handoff attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffResult {
    pub agent: String,
    pub success: bool,
    pub message: String,
    pub handoff_file: Option<String>,
}

/// Agent availability status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub name: String,
    pub available: bool,
    pub reason: String,
    pub version: Option<String>,
}
