import * as vscode from 'vscode';
import { RelayClient } from './relayClient';
import { registerCommands } from './commands';
import { SessionPanelProvider } from './views/sessionPanel';

export async function activate(context: vscode.ExtensionContext) {
	const relayClient = new RelayClient('relay');

	const statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
	statusBarItem.command = 'relay.showStatus';
	statusBarItem.text = '⚡ Relay';
	statusBarItem.show();
	context.subscriptions.push(statusBarItem);

	const sessionPanel = new SessionPanelProvider(context.extensionUri, relayClient);
	context.subscriptions.push(
		vscode.window.registerTreeDataProvider('relay.sessionPanel', sessionPanel)
	);

	registerCommands(context, relayClient);
}

export function deactivate() {}
