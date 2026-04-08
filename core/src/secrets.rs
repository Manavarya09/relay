//! Secret detection — scans handoff text for potential sensitive data.

use regex::Regex;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SecretFinding {
    pub pattern_name: String,
    pub line_number: usize,
    pub redacted_match: String,
}

/// Scan text for potential secrets and return findings.
pub fn scan_for_secrets(text: &str) -> Vec<SecretFinding> {
    let patterns: Vec<(&str, Regex)> = vec![
        ("AWS Access Key", Regex::new(r"AKIA[0-9A-Z]{16}").unwrap()),
        ("AWS Secret Key", Regex::new(r"(?i)aws[_\-]?secret[_\-]?access[_\-]?key\s*[=:]\s*\S+").unwrap()),
        ("Generic API Key", Regex::new(r#"(?i)(api[_\-]?key|apikey)\s*[=:]\s*['"]?[A-Za-z0-9\-_]{20,}"#).unwrap()),
        ("Generic Secret", Regex::new(r#"(?i)(secret|token|password|passwd|pwd)\s*[=:]\s*['"]?[A-Za-z0-9\-_]{8,}"#).unwrap()),
        ("Private Key", Regex::new(r"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----").unwrap()),
        ("GitHub Token", Regex::new(r"gh[pousr]_[A-Za-z0-9_]{36,}").unwrap()),
        ("Slack Token", Regex::new(r"xox[bpoa]-[A-Za-z0-9\-]+").unwrap()),
        ("Connection String", Regex::new(r"(?i)(mongodb|postgres|mysql|redis)://[^\s]+").unwrap()),
        ("Bearer Token", Regex::new(r"(?i)bearer\s+[A-Za-z0-9\-_.~+/]+=*").unwrap()),
        ("OpenAI Key", Regex::new(r"sk-[A-Za-z0-9]{20,}").unwrap()),
    ];

    let mut findings = Vec::new();

    for (line_num, line) in text.lines().enumerate() {
        for (name, regex) in &patterns {
            if let Some(m) = regex.find(line) {
                let matched = m.as_str();
                // Redact: show first 4 and last 2 chars
                let redacted = if matched.len() > 10 {
                    format!("{}...{}", &matched[..4], &matched[matched.len()-2..])
                } else {
                    format!("{}...", &matched[..matched.len().min(4)])
                };

                findings.push(SecretFinding {
                    pattern_name: name.to_string(),
                    line_number: line_num + 1,
                    redacted_match: redacted,
                });
            }
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_aws_access_key() {
        let text = "export AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE";
        let findings = scan_for_secrets(text);
        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.pattern_name == "AWS Access Key"));
    }

    #[test]
    fn detects_openai_key() {
        let text = "OPENAI_API_KEY=sk-abc123def456ghi789jkl012mno345pqr678";
        let findings = scan_for_secrets(text);
        assert!(findings.iter().any(|f| f.pattern_name == "OpenAI Key"));
    }

    #[test]
    fn detects_private_key() {
        let text = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAK...";
        let findings = scan_for_secrets(text);
        assert!(findings.iter().any(|f| f.pattern_name == "Private Key"));
    }

    #[test]
    fn detects_github_token() {
        let text = "gh_token = ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij";
        let findings = scan_for_secrets(text);
        assert!(findings.iter().any(|f| f.pattern_name == "GitHub Token"));
    }

    #[test]
    fn no_false_positives_on_normal_text() {
        let text = "This is a normal conversation about fixing the login page.\nThe error was in auth.rs on line 42.";
        let findings = scan_for_secrets(text);
        assert!(findings.is_empty());
    }

    #[test]
    fn detects_connection_string() {
        let text = "DATABASE_URL=postgres://user:pass@localhost:5432/mydb";
        let findings = scan_for_secrets(text);
        assert!(findings.iter().any(|f| f.pattern_name == "Connection String"));
    }

    #[test]
    fn redacts_matched_secrets() {
        let text = "AKIAIOSFODNN7EXAMPLE";
        let findings = scan_for_secrets(text);
        assert!(!findings.is_empty());
        // Should not contain the full key
        assert!(!findings[0].redacted_match.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(findings[0].redacted_match.contains("..."));
    }
}
