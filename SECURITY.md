# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.0.x   | Yes       |
| < 1.0   | No        |

## Reporting a Vulnerability

If you discover a security vulnerability in Relay, please report it responsibly:

1. **Email**: smanavarya@gmail.com
2. **Subject**: `[SECURITY] Relay — <brief description>`
3. **Do not** open a public GitHub issue for security vulnerabilities

I'll acknowledge your report within 48 hours and work on a fix.

## Scope

Relay reads Claude Code session transcripts from disk and passes them to other agents. Security concerns include:
- Sensitive data in session transcripts being forwarded unintentionally
- API keys in config.toml being exposed
- Handoff files containing credentials

Relay stores handoff files locally in `.relay/` — these should be gitignored (they are by default).
