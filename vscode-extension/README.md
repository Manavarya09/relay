# Relay — VS Code Extension

Manage Claude Code session handoffs directly in VS Code.

## Features

- 📋 **Session Panel** — View current task, git state, and available agents in the sidebar
- 🤖 **One-Click Handoff** — Hand off to any available agent directly from VS Code
- 📜 **History** — Browse past handoffs and what changed
- 📊 **Diff Viewer** — See exactly what changed since last handoff
- 🔄 **Auto-Refresh** — Session state updates automatically

## Requirements

- VS Code 1.84+
- [Relay CLI](https://github.com/Manavarya09/relay) installed and in your PATH

## Installation

1. Clone the relay repository
2. Navigate to `vscode-extension` folder
3. Install dependencies:
   ```bash
   npm install
   ```
4. Compile TypeScript:
   ```bash
   npm run compile
   ```
5. Open in VS Code and press `F5` to run the extension in debug mode

## Configuration

Edit `.vscode/settings.json` in your workspace:

```json
{
  "relay.binaryPath": "relay",
  "relay.autoRefreshInterval": 5000
}
```

- `relay.binaryPath` — Path to relay binary (default: "relay")
- `relay.autoRefreshInterval` — Auto-refresh interval in ms (0 to disable)

## Commands

- **Relay: Show Session Status** — Display current session details
- **Relay: Handoff to Agent** — Select and handoff to an agent
- **Relay: Copy Handoff to Clipboard** — Copy handoff prompt
- **Relay: View Handoff History** — Browse past handoffs
- **Relay: View Diff Since Handoff** — See what changed

## Usage

1. Open a project with a Claude Code session in VS Code
2. Look for "Relay Session" panel in the Explorer sidebar
3. Click the agent name to handoff, or use commands from the palette
4. Session state updates automatically

## Development

```bash
# Watch for changes
npm run watch

# Compile once
npm run compile

# Lint
npm run lint
```

## Publishing

```bash
# Install vsce
npm install -g vsce

# Package
vsce package

# Publish to marketplace
vsce publish
```
