import * as vscode from 'vscode';

export interface Agent {
	id: string;
	name: string;
	autoTrigger: boolean;
	launch: (handoffText: string) => Promise<void>;
}

const openChat = (text: string): Thenable<unknown> =>
	vscode.commands.executeCommand('workbench.action.chat.open', { query: text });

const withFallback = async (primary: () => Thenable<unknown>, fallback: () => Thenable<unknown>): Promise<void> => {
	try { await primary(); } catch { await fallback(); }
};

const delay = (ms: number) => new Promise(r => setTimeout(r, ms));

const clipboardLaunch = async (text: string, ...commands: string[]): Promise<void> => {
	for (const cmd of commands) {
		try {
			await vscode.commands.executeCommand(cmd);
			await delay(500);
			break;
		} catch { continue; }
	}
	await vscode.env.clipboard.writeText(text);
	vscode.window.showInformationMessage('Handoff copied to clipboard — paste (Ctrl+V) in the chat input.');
};

export const AGENTS: Agent[] = [
	{
		id: 'github.copilot-chat',
		name: 'GitHub Copilot Chat',
		autoTrigger: true,
		launch: async (text) => { await openChat(text); },
	},
	{
		id: 'google.geminicodeassist',
		name: 'Google Gemini Code Assist',
		autoTrigger: true,
		launch: async (text) => {
			await withFallback(
				() => vscode.commands.executeCommand('geminicodeassist.startagent'),
				() => Promise.resolve()
			);
			await delay(800);
			await openChat(text);
		},
	},
	{
		id: 'anthropic.claude-code',
		name: 'Claude Code',
		autoTrigger: true,
		launch: async (text) => {
			await withFallback(
				() => vscode.commands.executeCommand('claude.newTask', text),
				() => openChat(text)
			);
		},
	},
	{
		id: 'saoudrizwan.claude-dev',
		name: 'Cline',
		autoTrigger: true,
		launch: async (text) => {
			await withFallback(
				() => vscode.commands.executeCommand('cline.newTask', text),
				() => vscode.commands.executeCommand('cline.openInNewTab')
			);
		},
	},
	{
		id: 'rooveterinaryinc.roo-cline',
		name: 'Roo Code',
		autoTrigger: true,
		launch: async (text) => {
			await withFallback(
				() => vscode.commands.executeCommand('roo-cline.newTask', text),
				() => vscode.commands.executeCommand('roo-cline.openInNewTab')
			);
		},
	},
	{
		id: 'sourcegraph.cody-ai',
		name: 'Sourcegraph Cody',
		autoTrigger: true,
		launch: async (text) => {
			await withFallback(
				() => vscode.commands.executeCommand('cody.chat.newEditorPanel'),
				() => openChat(text)
			);
		},
	},
	{
		id: 'continue.continue',
		name: 'Continue',
		autoTrigger: true,
		launch: async (text) => {
			await withFallback(
				() => vscode.commands.executeCommand('continue.focusContinueInput', text),
				() => openChat(text)
			);
		},
	},
	{
		id: 'amazonwebservices.amazon-q-vscode',
		name: 'Amazon Q',
		autoTrigger: true,
		launch: async (text) => {
			await withFallback(
				() => vscode.commands.executeCommand('aws.amazonq.chat', text),
				() => vscode.commands.executeCommand('aws.amazonq.focusChat')
			);
		},
	},
	{
		id: 'codeium.codeium',
		name: 'Codeium / Windsurf',
		autoTrigger: true,
		launch: async (text) => {
			await withFallback(
				() => vscode.commands.executeCommand('codeium.openChat'),
				() => openChat(text)
			);
		},
	},
	{
		id: 'tabnine.tabnine-vscode',
		name: 'Tabnine',
		autoTrigger: true,
		launch: async (text) => {
			await withFallback(
				() => vscode.commands.executeCommand('tabnine.chat.focus-input'),
				() => openChat(text)
			);
		},
	},
	{
		id: 'codium.qodogen',
		name: 'Qodo Gen',
		autoTrigger: true,
		launch: async (text) => {
			await withFallback(
				() => vscode.commands.executeCommand('qodogen.openChat'),
				() => openChat(text)
			);
		},
	},
	{
		id: 'openai.chatgpt',
		name: 'Codex (OpenAI)',
		autoTrigger: false,
		launch: (text) => clipboardLaunch(text, 'chatgpt.openSidebar', 'chatgpt.newChat'),
	},
	{
		id: 'blackboxapp.blackboxagent',
		name: 'Blackbox AI Agent',
		autoTrigger: false,
		launch: (text) => clipboardLaunch(text, 'workbench.view.extension.blackboxai-dev-ActivityBar', 'blackbox.enableChatModeClicked'),
	},
	{
		id: 'blackboxapp.blackbox',
		name: 'Blackbox AI',
		autoTrigger: false,
		launch: (text) => clipboardLaunch(text, 'workbench.view.extension.blackboxai-dev-ActivityBar', 'blackbox.enableChatModeClicked'),
	},
	{
		id: 'mistralai.mistral-code',
		name: 'Mistral Code',
		autoTrigger: true,
		launch: async (text) => { await openChat(text); },
	},
	{
		id: 'phind.phind',
		name: 'Phind',
		autoTrigger: true,
		launch: async (text) => {
			await withFallback(
				() => vscode.commands.executeCommand('phind.search', text),
				() => openChat(text)
			);
		},
	},
	{
		id: 'meshintelligentTechnologiesinc.pieces-vscode',
		name: 'Pieces for Developers',
		autoTrigger: true,
		launch: async (text) => {
			await withFallback(
				() => vscode.commands.executeCommand('pieces.copilot.open'),
				() => openChat(text)
			);
		},
	},
	{
		id: 'supermaven.supermaven',
		name: 'Supermaven',
		autoTrigger: true,
		launch: async (text) => { await openChat(text); },
	},
	{
		id: 'bito.bito',
		name: 'Bito AI',
		autoTrigger: true,
		launch: async (text) => { await openChat(text); },
	},
	{
		id: 'huggingface.huggingface-vscode',
		name: 'HuggingFace',
		autoTrigger: true,
		launch: async (text) => { await openChat(text); },
	},
	{
		id: 'warm3snow.vscode-ollama',
		name: 'Ollama',
		autoTrigger: true,
		launch: async (text) => {
			await withFallback(
				() => vscode.commands.executeCommand('vscode-ollama.openChat'),
				() => openChat(text)
			);
		},
	},
	{
		id: 'lee2py.aider-composer',
		name: 'Aider Composer',
		autoTrigger: true,
		launch: async (text) => {
			await withFallback(
				() => vscode.commands.executeCommand('aider-composer.newSession', text),
				() => openChat(text)
			);
		},
	},
	{
		id: 'kiteco.kite',
		name: 'Kite',
		autoTrigger: true,
		launch: async (text) => { await openChat(text); },
	},
];

function findExtension(id: string): vscode.Extension<unknown> | undefined {
	const lower = id.toLowerCase();
	return vscode.extensions.all.find(ext => ext.id.toLowerCase() === lower);
}

export function getInstalledAgents(): Agent[] {
	return AGENTS.filter(agent => findExtension(agent.id) !== undefined);
}

export async function launchAgent(id: string, handoffText: string): Promise<void> {
	const ext = findExtension(id);
	if (ext && !ext.isActive) {
		await ext.activate();
	}
	await vscode.env.clipboard.writeText(handoffText);

	const agent = AGENTS.find(a => a.id === id);
	await (agent ? agent.launch(handoffText) : openChat(handoffText));
}
