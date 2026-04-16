import * as vscode from 'vscode';
import { RelayClient } from '../relayClient';
import { getInstalledAgents, launchAgent } from '../agents/registry';

interface AgentQuickPick extends vscode.QuickPickItem {
	extensionId: string;
}

function openTerminal(command: string, name: string): void {
	const terminal = vscode.window.createTerminal(name);
	terminal.sendText(command);
	terminal.show();
}

function projectDir(): string | undefined {
	return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
}

export function registerCommands(context: vscode.ExtensionContext, relayClient: RelayClient): void {
	context.subscriptions.push(
		vscode.commands.registerCommand('relay.handoff', async () => {
			const agents = getInstalledAgents();

			if (agents.length < 1) {
				vscode.window.showWarningMessage('No agent extensions found. Install Copilot, Gemini, or similar.');
				return;
			}

			const toItems = (list: ReturnType<typeof getInstalledAgents>): AgentQuickPick[] =>
				list.map(a => ({
					label: a.name,
					extensionId: a.id,
					description: a.autoTrigger ? 'Auto' : 'Clipboard',
				}));

			const from = await vscode.window.showQuickPick(toItems(agents), {
				placeHolder: 'Handing off from...',
				title: 'Relay Handoff (1/2)',
			});
			if (!from) { return; }

			const to = await vscode.window.showQuickPick(
				toItems(agents.filter(a => a.id !== from.extensionId)),
				{
					placeHolder: 'Hand off to...',
					title: `Relay Handoff: ${from.label} → (2/2)`,
				}
			);
			if (!to) { return; }

			await vscode.window.withProgress(
				{ location: vscode.ProgressLocation.Notification, title: `${from.label} → ${to.label}`, cancellable: false },
				async (progress) => {
					try {
						progress.report({ message: 'Capturing session...' });
						const result = await relayClient.getHandoffText(to.extensionId, projectDir());

						const handoffWithContext = `[Handoff from ${from.label}]\n\n${result.handoff_text}`;

						progress.report({ message: `Launching ${to.label}...` });
						await launchAgent(to.extensionId, handoffWithContext);

						vscode.window.showInformationMessage(
							`Session handed off from ${from.label} to ${to.label}. Saved: ${result.handoff_file}`
						);
					} catch (error) {
						vscode.window.showErrorMessage(`Handoff failed: ${error}`);
					}
				}
			);
		})
	);

	context.subscriptions.push(
		vscode.commands.registerCommand('relay.copyHandoff', async () => {
			openTerminal(relayClient.buildCopyHandoffCommand(projectDir()), 'Relay: Copy Handoff');
		})
	);

	context.subscriptions.push(
		vscode.commands.registerCommand('relay.showStatus', async () => {
			openTerminal(relayClient.buildStatusCommand(projectDir()), 'Relay: Status');
		})
	);

	context.subscriptions.push(
		vscode.commands.registerCommand('relay.history', async () => {
			openTerminal(relayClient.buildHistoryCommand(10, projectDir()), 'Relay: History');
		})
	);

	context.subscriptions.push(
		vscode.commands.registerCommand('relay.viewDiff', async () => {
			openTerminal(relayClient.buildDiffCommand(projectDir()), 'Relay: Diff');
		})
	);
}
