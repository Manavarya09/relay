# Relay

**When Claude Code hits its rate limit, another agent picks up exactly where you left off — with full conversation context.**

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![npm](https://img.shields.io/npm/v/@masyv/relay)](https://www.npmjs.com/package/@masyv/relay)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

![Relay — Claude to Codex handoff](relay-hero.png)

## The Problem

It's 6:20 PM. Your submission is at 7 PM. You're deep in a Claude Code session — 45 minutes of context, decisions, half-finished code. Then:

> **Rate limit reached. Please wait.**

Your entire session context is gone. You open Codex or Gemini and spend 20 minutes re-explaining everything. By the time you're set up, it's 6:50.

## The Solution

```bash
relay handoff --to codex
```

Relay reads your **actual Claude Code session** — the full conversation, every tool call, every file edit, every error — compresses it into a handoff package, and opens Codex (or Gemini, Aider, Ollama, etc.) with complete context. The new agent knows exactly what you were doing and waits for your instructions.

## What Relay Captures

This is NOT just git state. Relay reads Claude's actual `.jsonl` session transcript:

```
  ════════════════════════════════════════════════════════
  📋  Session Snapshot
  ════════════════════════════════════════════════════════

  📁  /Users/dev/myproject
  🕐  2026-04-05 14:46

  🎯 Current Task
  ──────────────────────────────────────────────────
  Fix the mobile/desktop page separation in the footer

  📝 Progress
  ──────────────────────────────────────────────────
  ✅  Database schema + REST API
  ✅  Landing page overhaul
  🔄  Footer link separation (IN PROGRESS)
  ⏳  Auth system

  🚨 Last Error
  ──────────────────────────────────────────────────
  Error: Next.js couldn't find the package from project directory

  💡 Key Decisions
  ──────────────────────────────────────────────────
  • Using Socket.io instead of raw WebSockets
  • Clean reinstall fixed the @next/swc-darwin-arm64 issue

  💬 Conversation (25 turns)
  ──────────────────────────────────────────────────
  🤖 AI   Now update the landing page footer too.
  🔧 TOOL [Edit] pages/index.tsx (replacing 488 chars)
  📤 OUT  File updated successfully.
  🤖 AI   Add /mobile to the Layout bypass list.
  🔧 TOOL [Edit] components/Layout.tsx (replacing 99 chars)
  🔧 TOOL [Bash] npx next build
  📤 OUT  ✓ Build passed — 12 pages compiled
```

The fallback agent sees **everything**: what Claude was thinking, what files it edited, what errors it hit, and where it stopped.

## 8 Supported Agents

```
  ════════════════════════════════════════════════════════
  🤖  Available Agents
  ════════════════════════════════════════════════════════

  Priority: codex → claude → aider → gemini → copilot → opencode → ollama → openai

  ✅  codex        Found at /opt/homebrew/bin/codex
  ✅  copilot      Found at /opt/homebrew/bin/copilot
  ❌  claude       Install: npm install -g @anthropic-ai/claude-code
  ❌  aider        Install: pip install aider-chat
  ❌  gemini       Set GEMINI_API_KEY env var
  ❌  opencode     Install: go install github.com/opencode-ai/opencode@latest
  ❌  ollama       Not reachable at http://localhost:11434
  ❌  openai       Set OPENAI_API_KEY env var

  🚀 2 agents ready for handoff
```

| Agent | Type | How it launches |
|-------|------|-----------------|
| **Codex** | CLI (OpenAI) | Opens interactive TUI with context |
| **Claude** | CLI (Anthropic) | New Claude session with context |
| **Aider** | CLI (open source) | Opens with --message handoff |
| **Gemini** | API / CLI | Gemini CLI or REST API |
| **Copilot** | CLI (GitHub) | Opens with context |
| **OpenCode** | CLI (Go) | Opens with context |
| **Ollama** | Local API | REST call to local model |
| **OpenAI** | API | GPT-4o / GPT-5.4 API call |

## Quick Start

```bash
# Install
git clone https://github.com/Manavarya09/relay
cd relay && ./scripts/build.sh

# Symlink to PATH (avoids macOS quarantine)
ln -sf $(pwd)/core/target/release/relay ~/.cargo/bin/relay

# Generate config
relay init

# Check what agents you have
relay agents

# See your current session snapshot
relay status

# Hand off to Codex (interactive — opens TUI)
relay handoff --to codex

# Interactive agent picker
relay handoff

# With deadline urgency
relay handoff --to codex --deadline "7:00 PM"
```

## Context Control

```bash
# Default: last 25 conversation turns + everything
relay handoff --to codex

# Light: 10 turns only
relay handoff --to codex --turns 10

# Only git state + todos (no conversation)
relay handoff --to codex --include git,todos

# Only conversation
relay handoff --to codex --include conversation

# Dry run — see what gets sent without launching
relay handoff --dry-run
```

## How It Works

1. **Reads** `~/.claude/projects/<project>/<session>.jsonl` — Claude's actual transcript
2. **Extracts** user messages, assistant responses, tool calls (Bash, Read, Write, Edit), tool results, errors
3. **Reads** TodoWrite state from the JSONL (your live todo list)
4. **Captures** git branch, diff summary, uncommitted files, recent commits
5. **Compresses** into a handoff prompt optimized for the target agent
6. **Launches** the agent interactively with inherited stdin/stdout

## Config

`~/.relay/config.toml`:

```toml
[general]
priority = ["codex", "claude", "aider", "gemini", "copilot", "opencode", "ollama", "openai"]
max_context_tokens = 8000
auto_handoff = true

[agents.codex]
model = "gpt-5.4"

[agents.gemini]
api_key = "your-key"

[agents.openai]
api_key = "your-key"

[agents.ollama]
url = "http://localhost:11434"
model = "llama3"
```

## Auto-Handoff (PostToolUse Hook)

Add to `~/.claude/settings.json`:

```json
{
  "hooks": {
    "PostToolUse": [
      { "matcher": "*", "hooks": [{ "type": "command", "command": "relay hook" }] }
    ]
  }
}
```

Relay detects rate limit signals in tool output and automatically hands off.

## Performance

- **4.6 MB** binary
- **< 100ms** session capture
- **Zero network calls** for capture
- **Rust** — no runtime, no GC

## License

MIT

---

Built by [@masyv](https://github.com/Manavarya09)
