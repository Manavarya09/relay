//! Cost estimation for API handoffs — shows estimated token count and price.

/// Estimate tokens from character count.
/// Uses model-specific ratios for better accuracy.
pub fn estimate_tokens(text: &str, model: &str) -> usize {
    let chars = text.len();
    let ratio = match model {
        m if m.contains("gpt-4o") => 3.5,
        m if m.contains("gpt-5") => 3.5,
        m if m.contains("gemini") => 3.8,
        m if m.contains("claude") => 3.5,
        m if m.contains("llama") => 3.8,
        _ => 3.5, // conservative default
    };
    (chars as f64 / ratio).ceil() as usize
}

/// Pricing per 1M input tokens (USD).
/// Known model ids come from the `llm-prices` table; anything it doesn't
/// recognise (dated ids, local models) falls back to the substring match below.
fn price_per_million_input(model: &str) -> f64 {
    if let Some(input) = llm_prices::lookup(model).and_then(|p| p.input) {
        return input;
    }
    match model {
        m if m.contains("gpt-4o") => 2.50,
        m if m.contains("gpt-5.4") => 5.00,
        m if m.contains("gpt-4") => 10.00,
        m if m.contains("o4-mini") => 1.10,
        m if m.contains("gemini-2.5-pro") => 1.25,
        m if m.contains("gemini-2.5-flash") => 0.15,
        m if m.contains("llama") => 0.0, // local
        _ => 0.0, // unknown or free
    }
}

/// Pricing per 1M output tokens (USD), when the model is in the `llm-prices` table.
fn price_per_million_output(model: &str) -> Option<f64> {
    llm_prices::lookup(model).and_then(|p| p.output)
}

#[derive(Debug, serde::Serialize)]
pub struct CostEstimate {
    pub tokens: usize,
    pub model: String,
    pub estimated_cost_usd: f64,
    pub output_price_per_million: Option<f64>,
    pub is_free: bool,
}

/// Estimate cost for a handoff to a specific agent/model.
pub fn estimate_cost(text: &str, agent: &str, model: &str) -> CostEstimate {
    let tokens = estimate_tokens(text, model);
    let price = price_per_million_input(model);
    let cost = tokens as f64 * price / 1_000_000.0;
    let is_free = price == 0.0 || matches!(agent, "codex" | "claude" | "aider" | "copilot" | "opencode" | "ollama");

    CostEstimate {
        tokens,
        model: model.to_string(),
        estimated_cost_usd: cost,
        output_price_per_million: price_per_million_output(model),
        is_free,
    }
}

/// Format cost for display.
pub fn format_cost(estimate: &CostEstimate) -> String {
    if estimate.is_free {
        return format!("~{} tokens (free — local/CLI agent)", estimate.tokens);
    }
    let output = match estimate.output_price_per_million {
        Some(price) => format!(", plus output at ${:.2}/1M", price),
        None => String::new(),
    };
    if estimate.estimated_cost_usd < 0.01 {
        format!("~{} tokens (<$0.01 on {}{})", estimate.tokens, estimate.model, output)
    } else {
        format!("~{} tokens (~${:.3} on {}{})", estimate.tokens, estimate.estimated_cost_usd, estimate.model, output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_tokens_reasonable() {
        let text = "a".repeat(3500);
        let tokens = estimate_tokens(&text, "gpt-4o");
        assert!(tokens >= 900 && tokens <= 1100);
    }

    #[test]
    fn free_agents_have_zero_cost() {
        let est = estimate_cost("hello world", "ollama", "llama3");
        assert!(est.is_free);
        assert_eq!(est.estimated_cost_usd, 0.0);
    }

    #[test]
    fn api_agents_have_cost() {
        let text = "a".repeat(35000); // ~10k tokens
        let est = estimate_cost(&text, "openai", "gpt-4o");
        assert!(!est.is_free);
        assert!(est.estimated_cost_usd > 0.0);
    }

    #[test]
    fn format_shows_free() {
        let est = estimate_cost("test", "codex", "o4-mini");
        let fmt = format_cost(&est);
        assert!(fmt.contains("free"));
    }

    #[test]
    fn known_models_use_listed_prices() {
        assert_eq!(price_per_million_input("gpt-4.1"), 2.0);
        assert_eq!(price_per_million_input("gpt-5.4"), 2.5);
        assert_eq!(price_per_million_input("gemini-2.5-flash"), 0.3);
        assert_eq!(price_per_million_output("gpt-4o"), Some(10.0));
    }

    #[test]
    fn unknown_ids_fall_back_to_substring_match() {
        assert_eq!(price_per_million_input("gpt-4o-2024-08-06"), 2.50);
        assert_eq!(price_per_million_input("llama3"), 0.0);
        assert_eq!(price_per_million_output("llama3"), None);
    }

    #[test]
    fn format_shows_output_price() {
        let text = "a".repeat(35000);
        let est = estimate_cost(&text, "openai", "gpt-4o");
        assert!(format_cost(&est).contains("output at $10.00/1M"));
    }
}
