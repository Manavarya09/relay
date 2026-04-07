import * as vscode from 'vscode';
import { RelayClient } from '../relayClient';

export function registerCommands(
	context: vscode.ExtensionContext,
	relayClient: RelayClient
): void {
	// Helper to open terminal with command
	function openTerminalWithCommand(command: string, name: string = 'Relay'): void {
		const terminal = vscode.window.createTerminal(name);
		terminal.sendText(command);
		terminal.show();
	}

	// Handoff
	context.subscriptions.push(
		vscode.commands.registerCommand('relay.handoff', async () => {
			try {
				const projectDir = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;

				const agents = ['claude', 'codex', 'gemini', 'aider', 'copilot', 'opencode', 'ollama', 'openai'];

				const selected = await vscode.window.showQuickPick(agents, {
					placeHolder: 'Select agent to handoff to',
					title: 'Relay Handoff',
				});

				if (!selected) {
					return;
				}

				const command = relayClient.buildHandoffCommand(selected, projectDir);
				openTerminalWithCommand(command, `Relay → ${selected}`);
			} catch (error) {
				vscode.window.showErrorMessage(`Error: ${error}`);
			}
		})
	);

	// Copy Handoff
	context.subscriptions.push(
		vscode.commands.registerCommand('relay.copyHandoff', async () => {
			try {
				const projectDir = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
				const command = relayClient.buildCopyHandoffCommand(projectDir);
				openTerminalWithCommand(command, 'Relay: Copy Handoff');
			} catch (error) {
				vscode.window.showErrorMessage(`Error: ${error}`);
			}
		})
	);

	// Status
	context.subscriptions.push(
		vscode.commands.registerCommand('relay.showStatus', async () => {
			try {
				const projectDir = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
				const command = relayClient.buildStatusCommand(projectDir);
				openTerminalWithCommand(command, 'Relay: Status');
			} catch (error) {
				vscode.window.showErrorMessage(`Error: ${error}`);
			}
		})
	);

	// History
	context.subscriptions.push(
		vscode.commands.registerCommand('relay.history', async () => {
			try {
				const projectDir = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
				const command = relayClient.buildHistoryCommand(10, projectDir);
				openTerminalWithCommand(command, 'Relay: History');
			} catch (error) {
				vscode.window.showErrorMessage(`Error: ${error}`);
			}
		})
	);

	// View Diff
	context.subscriptions.push(
		vscode.commands.registerCommand('relay.viewDiff', async () => {
			try {
				const projectDir = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
				const command = relayClient.buildDiffCommand(projectDir);
				openTerminalWithCommand(command, 'Relay: Diff');
			} catch (error) {
				vscode.window.showErrorMessage(`Error: ${error}`);
			}
		})
	);
}
