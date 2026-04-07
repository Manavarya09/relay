import * as vscode from 'vscode';
import { RelayClient } from './relayClient';

// Known agent/code-assistant extensions
const AGENT_EXTENSIONS = [
	{ id: 'github.copilot-chat', name: 'GitHub Copilot Chat' },
	{ id: 'google.geminicodeassist', name: 'Google Gemini Code Assist' },
	{ id: 'Codeium.codeium', name: 'Codeium' },
	{ id: 'aws.amazonq', name: 'Amazon Q' },
	{ id: 'TabNine.tabnine', name: 'Tabnine' },
	{ id: 'continue.continue', name: 'Continue' },
	{ id: 'supermaven.supermaven', name: 'Supermaven' },
];

function getInstalledAgentExtensions(): typeof AGENT_EXTENSIONS {
	return AGENT_EXTENSIONS.filter(agent => {
		const ext = vscode.extensions.getExtension(agent.id);
		return ext !== undefined;
	});
}

export async function activate(context: vscode.ExtensionContext) {
	console.log('Relay extension activating...');

	// Initialize relay client (just for building commands, no execution)
	const relayClient = new RelayClient('relay');

	// Create status bar item
	const statusBarItem = vscode.window.createStatusBarItem(
		vscode.StatusBarAlignment.Left,
		100
	);
	statusBarItem.command = 'relay.showStatus';
	statusBarItem.text = '⚡ Relay';
	statusBarItem.show();
	context.subscriptions.push(statusBarItem);

	// Register commands
	context.subscriptions.push(
		vscode.commands.registerCommand('relay.handoff', async () => {
			const projectDir = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;

			// Get installed VS Code extensions that are agent/code-assistant extensions
			const agentExtensions = getInstalledAgentExtensions();

			if (agentExtensions.length === 0) {
				vscode.window.showWarningMessage('No agent extensions found. Install Copilot, Gemini, or similar.');
				return;
			}

			interface AgentQuickPick extends vscode.QuickPickItem {
				extensionId: string;
			}

			const items: AgentQuickPick[] = agentExtensions.map(ext => ({
				label: ext.name,
				extensionId: ext.id,
				description: 'Ready to receive handoff',
			}));

			const selected = await vscode.window.showQuickPick(items, {
				placeHolder: 'Select agent extension to handoff to',
				title: 'Relay Handoff',
			});

			if (!selected) {
				return;
			}

			try {
				// Build and execute handoff command
				const command = relayClient.buildHandoffCommand(selected.extensionId, projectDir);
				const terminal = vscode.window.createTerminal(`Relay → ${selected.label}`);
				terminal.sendText(command);
				terminal.show();

				// Activate the target extension
				const targetExt = vscode.extensions.getExtension(selected.extensionId);
				if (targetExt && !targetExt.isActive) {
					await targetExt.activate();
				}

				vscode.window.showInformationMessage(
					`Handoff ready! Content copied to clipboard. Paste in ${selected.label} to continue the session.`
				);
			} catch (error) {
				vscode.window.showErrorMessage(`Handoff failed: ${error}`);
			}
		})
	);

	context.subscriptions.push(
		vscode.commands.registerCommand('relay.showStatus', async () => {
			const projectDir = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
			const command = relayClient.buildStatusCommand(projectDir);
			const terminal = vscode.window.createTerminal('Relay: Status');
			terminal.sendText(command);
			terminal.show();
		})
	);

	context.subscriptions.push(
		vscode.commands.registerCommand('relay.copyHandoff', async () => {
			const projectDir = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
			const command = relayClient.buildCopyHandoffCommand(projectDir);
			const terminal = vscode.window.createTerminal('Relay: Copy Handoff');
			terminal.sendText(command);
			terminal.show();
		})
	);

	context.subscriptions.push(
		vscode.commands.registerCommand('relay.history', async () => {
			const projectDir = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
			const command = relayClient.buildHistoryCommand(10, projectDir);
			const terminal = vscode.window.createTerminal('Relay: History');
			terminal.sendText(command);
			terminal.show();
		})
	);

	context.subscriptions.push(
		vscode.commands.registerCommand('relay.viewDiff', async () => {
			const projectDir = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
			const command = relayClient.buildDiffCommand(projectDir);
			const terminal = vscode.window.createTerminal('Relay: Diff');
			terminal.sendText(command);
			terminal.show();
		})
	);

	console.log('Relay extension activated successfully');
}

export function deactivate() {
	console.log('Relay extension deactivating...');
}
