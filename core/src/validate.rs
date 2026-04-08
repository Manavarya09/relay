//! Config validation — tests agent connectivity and API key validity.

use crate::Config;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidationResult {
    pub agent: String,
    pub status: String, // "ok", "warn", "error"
    pub message: String,
}

/// Validate all configured agents and return results.
pub fn validate_config(config: &Config) -> Vec<ValidationResult> {
    let mut results = Vec::new();

    // Check config file exists
    let config_path = crate::config_path();
    if !config_path.exists() {
        results.push(ValidationResult {
            agent: "config".into(),
            status: "warn".into(),
            message: "No config file found. Run 'relay init' to create one.".into(),
        });
    } else {
        results.push(ValidationResult {
            agent: "config".into(),
            status: "ok".into(),
            message: format!("Loaded from {}", config_path.display()),
        });
    }

    // Validate each agent in priority order
    for name in &config.general.priority {
        let result = match name.as_str() {
            "codex" => validate_cli_agent("codex", &config.agents.codex.binary.as_deref().unwrap_or("codex")),
            "claude" => validate_cli_agent("claude", "claude"),
            "aider" => validate_cli_agent("aider", "aider"),
            "copilot" => validate_cli_agent("copilot", "copilot"),
            "opencode" => validate_cli_agent("opencode", "opencode"),
            "gemini" => validate_gemini(config),
            "openai" => validate_openai(config),
            "ollama" => validate_ollama(config),
            _ => ValidationResult {
                agent: name.clone(),
                status: "warn".into(),
                message: "Unknown agent".into(),
            },
        };
        results.push(result);
    }

    results
}

fn validate_cli_agent(name: &str, binary: &str) -> ValidationResult {
    match crate::agents::find_binary(binary) {
        Some(path) => ValidationResult {
            agent: name.into(),
            status: "ok".into(),
            message: format!("Binary found at {}", path),
        },
        None => ValidationResult {
            agent: name.into(),
            status: "error".into(),
            message: format!("'{}' not found in PATH", binary),
        },
    }
}

fn validate_gemini(config: &Config) -> ValidationResult {
    // Check CLI first
    if crate::agents::find_binary("gemini").is_some() {
        return ValidationResult {
            agent: "gemini".into(),
            status: "ok".into(),
            message: "Gemini CLI found in PATH".into(),
        };
    }

    let api_key = config.agents.gemini.api_key.clone()
        .or_else(|| std::env::var("GEMINI_API_KEY").ok())
        .or_else(|| std::env::var("GOOGLE_API_KEY").ok());

    match api_key {
        Some(key) if !key.is_empty() && !key.contains("your-key") => ValidationResult {
            agent: "gemini".into(),
            status: "ok".into(),
            message: format!("API key set, model: {}", config.agents.gemini.model),
        },
        Some(_) => ValidationResult {
            agent: "gemini".into(),
            status: "warn".into(),
            message: "API key appears to be a placeholder".into(),
        },
        None => ValidationResult {
            agent: "gemini".into(),
            status: "error".into(),
            message: "No API key. Set GEMINI_API_KEY or add to config.toml".into(),
        },
    }
}

fn validate_openai(config: &Config) -> ValidationResult {
    let api_key = config.agents.openai.api_key.clone()
        .or_else(|| std::env::var("OPENAI_API_KEY").ok());

    match api_key {
        Some(key) if !key.is_empty() && !key.contains("your-key") => ValidationResult {
            agent: "openai".into(),
            status: "ok".into(),
            message: format!("API key set, model: {}", config.agents.openai.model),
        },
        Some(_) => ValidationResult {
            agent: "openai".into(),
            status: "warn".into(),
            message: "API key appears to be a placeholder".into(),
        },
        None => ValidationResult {
            agent: "openai".into(),
            status: "error".into(),
            message: "No API key. Set OPENAI_API_KEY or add to config.toml".into(),
        },
    }
}

fn validate_ollama(config: &Config) -> ValidationResult {
    let tag_url = format!("{}/api/tags", config.agents.ollama.url);
    let output = std::process::Command::new("curl")
        .args(["--silent", "--max-time", "2", &tag_url])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let body: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap_or_default();
            let models = body.get("models").and_then(|m| m.as_array()).map(|a| a.len()).unwrap_or(0);
            ValidationResult {
                agent: "ollama".into(),
                status: "ok".into(),
                message: format!("Running at {}, {} models loaded", config.agents.ollama.url, models),
            }
        }
        _ => ValidationResult {
            agent: "ollama".into(),
            status: "error".into(),
            message: format!("Not reachable at {}", config.agents.ollama.url),
        },
    }
}
