import * as vscode from 'vscode';
import { RelayClient, SessionSnapshot, AgentStatus } from '../relayClient';

export class SessionPanelProvider implements vscode.TreeDataProvider<SessionItem> {
	private _onDidChangeTreeData: vscode.EventEmitter<SessionItem | undefined | null | void> =
		new vscode.EventEmitter<SessionItem | undefined | null | void>();
	readonly onDidChangeTreeData: vscode.Event<SessionItem | undefined | null | void> =
		this._onDidChangeTreeData.event;

	private sessionSnapshot: SessionSnapshot | null = null;
	private agents: AgentStatus[] = [];

	constructor(
		private extensionUri: vscode.Uri,
		private relayClient: RelayClient
	) {
		this.refresh();
	}

	refresh(): void {
		this._onDidChangeTreeData.fire();
		this.loadSession();
	}

	private async loadSession(): Promise<void> {
		try {
			const projectDir = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
			this.sessionSnapshot = await this.relayClient.getStatus(projectDir);
			this.agents = await this.relayClient.getAgents(projectDir);
		} catch (error) {
			console.error('Failed to load session:', error);
		}
	}

	getTreeItem(element: SessionItem): vscode.TreeItem {
		return element;
	}

	async getChildren(element?: SessionItem): Promise<SessionItem[]> {
		if (!this.sessionSnapshot) {
			return [];
		}

		if (!element) {
			// Root items
			return [
				new SessionItem(
					'Task',
					`${this.sessionSnapshot.current_task.substring(0, 50)}...`,
					vscode.TreeItemCollapsibleState.None,
					'task'
				),
				new SessionItem(
					'Project',
					this.sessionSnapshot.project_dir.split('/').pop() || 'unknown',
					vscode.TreeItemCollapsibleState.None,
					'project'
				),
				...(this.sessionSnapshot.git_state
					? [
						new SessionItem(
							'Git',
							`${this.sessionSnapshot.git_state.branch} (${this.sessionSnapshot.git_state.status_summary})`,
							vscode.TreeItemCollapsibleState.Collapsed,
							'git'
						),
					]
					: []),
				new SessionItem(
					'Agents',
					`${this.agents.filter((a) => a.available).length}/${this.agents.length} available`,
					vscode.TreeItemCollapsibleState.Collapsed,
					'agents'
				),
				new SessionItem(
					'Handoff',
					'Click to handoff',
					vscode.TreeItemCollapsibleState.None,
					'handoff',
					'relay.handoff'
				),
			];
		}

		// Git details
		if (element.id === 'git' && this.sessionSnapshot.git_state) {
			return [
				new SessionItem(
					'Branch',
					this.sessionSnapshot.git_state.branch,
					vscode.TreeItemCollapsibleState.None,
					'git-branch'
				),
				new SessionItem(
					'Status',
					this.sessionSnapshot.git_state.status_summary,
					vscode.TreeItemCollapsibleState.None,
					'git-status'
				),
				...(this.sessionSnapshot.git_state.recent_commits.length > 0
					? [
						new SessionItem(
							'Recent Commits',
							'',
							vscode.TreeItemCollapsibleState.Collapsed,
							'git-commits'
						),
					]
					: []),
			];
		}

		// Recent commits
		if (element.id === 'git-commits' && this.sessionSnapshot.git_state) {
			return this.sessionSnapshot.git_state.recent_commits.map(
				(commit) =>
					new SessionItem(commit, '', vscode.TreeItemCollapsibleState.None, 'commit')
			);
		}

		// Agents - Show VS Code extensions instead
		if (element.id === 'agents') {
			const agentExtensions = [
				{ id: 'github.copilot-chat', name: 'GitHub Copilot Chat' },
				{ id: 'google.geminicodeassist', name: 'Google Gemini Code Assist' },
				{ id: 'Codeium.codeium', name: 'Codeium' },
				{ id: 'aws.amazonq', name: 'Amazon Q' },
				{ id: 'TabNine.tabnine', name: 'Tabnine' },
				{ id: 'continue.continue', name: 'Continue' },
				{ id: 'supermaven.supermaven', name: 'Supermaven' },
			];

			const installed = agentExtensions.filter(agent => {
				const ext = vscode.extensions.getExtension(agent.id);
				return ext !== undefined;
			});

			return installed.length > 0
				? installed.map(
					(agent) =>
						new SessionItem(
							agent.name,
							'✓ Ready',
							vscode.TreeItemCollapsibleState.None,
							'agent',
							'relay.handoff'
						)
				)
				: [
					new SessionItem(
						'No agent extensions found',
						'Install Copilot, Gemini, or similar',
						vscode.TreeItemCollapsibleState.None,
						'agent'
					),
				];
		}

		return [];
	}
}

class SessionItem extends vscode.TreeItem {
	constructor(
		public readonly label: string,
		public readonly description: string,
		public readonly collapsibleState: vscode.TreeItemCollapsibleState,
		public readonly id: string,
		commandId?: string
	) {
		super(label, collapsibleState);
		this.description = description;
		if (commandId) {
			this.command = {
				command: commandId,
				title: label,
				arguments: [],
			} as vscode.Command;
		}
	}
}
