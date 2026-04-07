# Extension Development Guide

## Project Structure

```
vscode-extension/
├── src/
│   ├── extension.ts          # Entry point
│   ├── relayClient.ts        # CLI wrapper
│   ├── commands/
│   │   └── index.ts          # Command handlers
│   ├── views/
│   │   └── sessionPanel.ts   # Sidebar panel
│   └── utils/
├── package.json              # Extension manifest
├── tsconfig.json             # TypeScript config
└── README.md
```

## Setup

```bash
# Install dependencies
npm install

# Watch for changes
npm run watch

# In VS Code: Press F5 to launch extension in debug mode
```

## Architecture

**extension.ts** — Activation and setup
- Initialize RelayClient
- Register commands
- Create status bar item
- Set up auto-refresh

**relayClient.ts** — Wraps relay CLI
- Executes relay commands as subprocesses
- Parses JSON output
- Handles errors

**commands/index.ts** — Command handlers
- showStatus
- handoff
- copyHandoff
- history
- viewDiff

**views/sessionPanel.ts** — Sidebar tree view
- Displays session snapshot
- Lists available agents
- Updates on refresh

## Adding a New Command

1. Add command to `package.json` under `contributes.commands`
2. Register handler in `commands/index.ts`:

```typescript
context.subscriptions.push(
  vscode.commands.registerCommand('relay.newCommand', async () => {
    try {
      // Your code here
      vscode.window.showInformationMessage('Done!');
    } catch (error) {
      vscode.window.showErrorMessage(`Error: ${error}`);
    }
  })
);
```

## Debugging

1. Press `F5` to launch in debug mode
2. Set breakpoints in TypeScript files
3. Use Debug Console for output
4. Hot-reload: `npm run watch` in another terminal

## Testing

```bash
# Manually test in debug extension window:
# - Run commands from command palette (Cmd+Shift+P)
# - Check sidebar panel updates
# - Verify error messages display correctly
```

## Common Issues

**"relay binary not found"**
- Ensure relay is installed: `cargo install --path core`
- Or set `relay.binaryPath` in settings

**Panel doesn't show**
- Check that relay binary is available
- Reload VS Code window (Cmd+Shift+P → Developer: Reload Window)

**Commands don't appear**
- Rebuild: `npm run compile`
- Reload VS Code

## Publishing to VS Code Marketplace

1. Install `vsce`: `npm install -g vsce`
2. Get a personal access token
3. Login: `vsce login relay`
4. Publish: `vsce publish`

See [VS Code Extension Publishing Guide](https://code.visualstudio.com/api/working-with-extensions/publishing-extension)
