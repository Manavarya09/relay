import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

export interface SessionSnapshot {
	current_task: string;
	todos: { content: string; status: string }[];
	decisions: string[];
	last_error: string | null;
	last_output: string | null;
	git_state: {
		branch: string;
		status_summary: string;
		recent_commits: string[];
		diff_summary: string;
		uncommitted_files: string[];
	} | null;
	project_dir: string;
	recent_files: string[];
	timestamp: string;
	deadline: string | null;
	conversation: { role: string; content: string }[];
}

export interface HandoffEntry {
	filename: string;
	timestamp: string;
	agent: string;
	task: string;
}

export interface AgentStatus {
	name: string;
	available: boolean;
	reason: string;
	version: string | null;
}

export class RelayClient {
	private binaryPath: string;

	constructor(binaryPath: string = 'relay') {
		this.binaryPath = binaryPath;
	}

	private normalizePath(path: string): string {
		// Convert backslashes to forward slashes for cross-platform compatibility
		return path.replace(/\\/g, '/');
	}

	async isAvailable(): Promise<boolean> {
		try {
			await this.exec(['--version']);
			return true;
		} catch {
			return false;
		}
	}

	async getStatus(projectDir?: string): Promise<SessionSnapshot> {
		const args = ['status', '--json'];
		if (projectDir) {
			args.push('--project', this.normalizePath(projectDir));
		}
		const output = await this.exec(args);
		return JSON.parse(output);
	}

	async getAgents(projectDir?: string): Promise<AgentStatus[]> {
		const args = ['agents', '--json'];
		if (projectDir) {
			args.push('--project', this.normalizePath(projectDir));
		}
		const output = await this.exec(args);
		return JSON.parse(output);
	}

	async handoff(
		agent: string,
		projectDir?: string,
		template: string = 'full'
	): Promise<string> {
		const args = ['handoff', '--to', agent, '--template', template];
		if (projectDir) {
			args.push('--project', this.normalizePath(projectDir));
		}
		const output = await this.exec(args);
		return output;
	}

	async copyHandoff(projectDir?: string): Promise<void> {
		const args = ['handoff', '--clipboard'];
		if (projectDir) {
			args.push('--project', this.normalizePath(projectDir));
		}
		await this.exec(args);
	}

	async getHistory(limit: number = 10, projectDir?: string): Promise<HandoffEntry[]> {
		const args = ['history', '--limit', limit.toString(), '--json'];
		if (projectDir) {
			args.push('--project', this.normalizePath(projectDir));
		}
		const output = await this.exec(args);
		return JSON.parse(output);
	}

	async getDiff(projectDir?: string): Promise<string> {
		const args = ['diff', '--json'];
		if (projectDir) {
			args.push('--project', this.normalizePath(projectDir));
		}
		const output = await this.exec(args);
		return output;
	}

	buildHandoffCommand(agent: string, projectDir?: string, template: string = 'full'): string {
		const args = ['handoff', '--to', agent, '--template', template, '--clipboard'];
		if (projectDir) {
			args.push('--project', this.normalizePath(projectDir));
		}
		return this.buildCommand(args);
	}

	buildCopyHandoffCommand(projectDir?: string): string {
		const args = ['handoff', '--clipboard'];
		if (projectDir) {
			args.push('--project', this.normalizePath(projectDir));
		}
		return this.buildCommand(args);
	}

	buildStatusCommand(projectDir?: string): string {
		const args = ['status'];
		if (projectDir) {
			args.push('--project', this.normalizePath(projectDir));
		}
		return this.buildCommand(args);
	}

	buildHistoryCommand(limit: number = 10, projectDir?: string): string {
		const args = ['history', '--limit', limit.toString()];
		if (projectDir) {
			args.push('--project', this.normalizePath(projectDir));
		}
		return this.buildCommand(args);
	}

	buildDiffCommand(projectDir?: string): string {
		const args = ['diff'];
		if (projectDir) {
			args.push('--project', this.normalizePath(projectDir));
		}
		return this.buildCommand(args);
	}

	private buildCommand(args: string[]): string {
		const argsStr = args
			.map((arg) => (arg.includes(' ') ? `"${arg}"` : arg))
			.join(' ');
		let binaryPath = this.binaryPath;
		if (process.platform === 'win32' && binaryPath.includes('\\')) {
			binaryPath = `"${binaryPath}"`;
		}
		return `${binaryPath} ${argsStr}`;
	}

	private async exec(args: string[]): Promise<string> {
		try {
			// Build command with proper quoting for arguments
			const argsStr = args
				.map((arg) => (arg.includes(' ') ? `"${arg}"` : arg))
				.join(' ');

			// Quote binary path on Windows with backslashes
			let binaryPath = this.binaryPath;
			if (process.platform === 'win32' && binaryPath.includes('\\')) {
				binaryPath = `"${binaryPath}"`;
			}

			const cmd = `${binaryPath} ${argsStr}`;
			console.log(`Executing: ${cmd}`);

			const { stdout } = await execAsync(cmd);
			return stdout.trim();
		} catch (error) {
			console.error(`Relay exec error: ${error}`);
			throw error;
		}
	}
}
