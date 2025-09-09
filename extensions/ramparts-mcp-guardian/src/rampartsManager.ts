import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { spawn, ChildProcess } from 'child_process';

export class RampartsManager {
    private context: vscode.ExtensionContext;
    private outputChannel: vscode.OutputChannel;
    private enabled: boolean = false;
    private proxyProcesses: Map<string, ChildProcess> = new Map();
    private status: string = 'Initializing';

    constructor(context: vscode.ExtensionContext) {
        this.context = context;
        this.outputChannel = vscode.window.createOutputChannel('Ramparts MCP Guardian');
        context.subscriptions.push(this.outputChannel);
    }

    async enable() {
        this.enabled = true;
        this.status = 'Enabled';
        this.log('Ramparts MCP Guardian enabled');
        
        // Ensure ramparts proxy binary is available
        await this.ensureProxyBinary();
        
        // Update configuration to enable
        const config = vscode.workspace.getConfiguration('ramparts');
        await config.update('enabled', true, vscode.ConfigurationTarget.Global);
        
        vscode.window.showInformationMessage('Ramparts MCP Guardian enabled. All MCP requests will now be secured.');
    }

    async disable() {
        this.enabled = false;
        this.status = 'Disabled';
        this.log('Ramparts MCP Guardian disabled');
        
        // Stop any running proxy processes
        this.stopAllProxyProcesses();
        
        // Update configuration to disable
        const config = vscode.workspace.getConfiguration('ramparts');
        await config.update('enabled', false, vscode.ConfigurationTarget.Global);
        
        vscode.window.showInformationMessage('Ramparts MCP Guardian disabled. MCP requests will use original servers.');
    }

    async reloadConfiguration() {
        this.log('Reloading Ramparts configuration...');

        const config = vscode.workspace.getConfiguration('ramparts');
        this.enabled = config.get<boolean>('enabled', true);

        if (this.enabled) {
            this.status = 'Enabled';
            await this.ensureProxyBinary();
            // Re-apply proxies when settings change (e.g., policy or API key updates)
            try {
                const mcpManager = new (require('./mcpConfigManager').McpConfigManager)(this.context);
                await mcpManager.reapplyProxies?.();
            } catch {}
        } else {
            this.status = 'Disabled';
            this.stopAllProxyProcesses();
        }

        this.log('Configuration reloaded');
    }

    private async ensureProxyBinary() {
        const proxyPath = this.getProxyBinaryPath();
        
        if (!fs.existsSync(proxyPath)) {
            this.log(`Proxy binary not found at ${proxyPath}, attempting to download...`);
            await this.downloadProxyBinary();
        } else {
            this.log(`Proxy binary found at ${proxyPath}`);
        }
    }

    private getProxyBinaryPath(): string {
        const extensionPath = this.context.extensionPath;
        const binDir = path.join(extensionPath, 'bin');
        
        // Ensure bin directory exists
        if (!fs.existsSync(binDir)) {
            fs.mkdirSync(binDir, { recursive: true });
        }
        
        const platform = process.platform;
        const arch = process.arch;
        const extension = platform === 'win32' ? '.exe' : '';
        
        return path.join(binDir, `ramparts-mcp-proxy-stdio-${platform}-${arch}${extension}`);
    }

    private async downloadProxyBinary() {
        try {
            this.status = 'Downloading proxy binary...';
            this.log('Downloading ramparts-mcp-proxy-stdio binary...');
            
            // In a real implementation, this would download from GitHub releases
            // For now, we'll show a message to install ramparts
            const result = await vscode.window.showWarningMessage(
                'Ramparts proxy binary not found. Please install ramparts first.',
                'Install Instructions',
                'Use Local Binary'
            );
            
            if (result === 'Install Instructions') {
                vscode.env.openExternal(vscode.Uri.parse('https://github.com/getjavelin/ramparts#installation'));
            } else if (result === 'Use Local Binary') {
                // Try to use ramparts from PATH
                this.log('Using ramparts from PATH');
            }
            
            this.status = 'Ready';
        } catch (error) {
            this.log(`Failed to download proxy binary: ${error}`);
            this.status = 'Error: Binary not available';
            throw error;
        }
    }

    async bypassServer(serverName: string) {
        const config = vscode.workspace.getConfiguration('ramparts');
        const bypassedServers = config.get<string[]>('bypassedServers', []);
        
        if (!bypassedServers.includes(serverName)) {
            bypassedServers.push(serverName);
            await config.update('bypassedServers', bypassedServers, vscode.ConfigurationTarget.Global);
            this.log(`Server '${serverName}' added to bypass list`);
            vscode.window.showInformationMessage(`Server '${serverName}' will bypass Ramparts security`);
        }
    }

    async unbypassServer(serverName: string) {
        const config = vscode.workspace.getConfiguration('ramparts');
        const bypassedServers = config.get<string[]>('bypassedServers', []);
        
        const index = bypassedServers.indexOf(serverName);
        if (index > -1) {
            bypassedServers.splice(index, 1);
            await config.update('bypassedServers', bypassedServers, vscode.ConfigurationTarget.Global);
            this.log(`Server '${serverName}' removed from bypass list`);
            vscode.window.showInformationMessage(`Server '${serverName}' will now use Ramparts security`);
        }
    }

    showStatus() {
        const config = vscode.workspace.getConfiguration('ramparts');
        const bypassedServers = config.get<string[]>('bypassedServers', []);
        
        const apiKey = config.get<string>('javelinApiKey');
        const apiKeyStatus = apiKey && apiKey.trim() !== '' ? 'Configured' : 'Test Mode (no API key)';

        const statusMessage = `
Ramparts MCP Guardian Status:
- Enabled: ${this.enabled}
- Status: ${this.status}
- Bypassed Servers: ${bypassedServers.length > 0 ? bypassedServers.join(', ') : 'None'}
- Javelin API Key: ${apiKeyStatus}
- Fail Mode: ${config.get<boolean>('failOpen') ? 'Open' : 'Closed'}
        `.trim();
        
        vscode.window.showInformationMessage(statusMessage, { modal: true });
    }

    showLogs() {
        this.outputChannel.show();
    }

    async scanServer(itemOrName?: any) {
        let serverName: string | undefined = undefined;
        let serverItem: any = undefined;

        if (typeof itemOrName === 'string') {
            serverName = itemOrName;
        } else if (itemOrName && typeof itemOrName === 'object' && itemOrName.serverName) {
            serverName = itemOrName.serverName;
            serverItem = itemOrName;
        }

        if (!serverName) {
            // Get servers from scan-config to match what scan-all finds
            try {
                const rampartsPath = await this.findRampartsCLI();
                if (!rampartsPath) {
                    vscode.window.showErrorMessage('Ramparts CLI not found. Please install ramparts first.');
                    return;
                }

                // Run scan-config to get the same servers that scan-all would find
                const scanResult = await this.executeScanCommand(rampartsPath, ['scan-config', '--format', 'json']);

                if (!scanResult.results || scanResult.results.length === 0) {
                    vscode.window.showWarningMessage('No MCP servers found in any IDE configuration.');
                    return;
                }

                const items = scanResult.results.map((result: any) => {
                    const serverName = this.getServerDisplayName(result);
                    const serverDisplay = result.url || 'stdio server';
                    const shortDisplay = serverDisplay.length > 50 ? serverDisplay.substring(0, 47) + '...' : serverDisplay;
                    const status = result.status === 'Success' ? '✅' : '❌';
                    const ideSource = result.ide_source || 'Unknown IDE';

                    return {
                        label: `$(${result.url ? 'globe' : 'terminal'}) ${status} ${serverName}`,
                        description: shortDisplay,
                        detail: `${ideSource} • ${result.url ? 'HTTP' : 'STDIO'}`,
                        serverName: serverName,
                        serverResult: result
                    };
                });

                const selected = await vscode.window.showQuickPick(items, {
                    placeHolder: 'Select MCP server to scan (from scan-config discovery)',
                    matchOnDescription: true,
                    matchOnDetail: true
                });

                if (!selected) {
                    return;
                }

                // Display the individual server result
                const singleServerResult = {
                    scan_type: 'single_server',
                    server_name: (selected as any).serverName,
                    timestamp: new Date().toISOString(),
                    results: [(selected as any).serverResult]
                };

                await this.displayScanResults(singleServerResult, 'server', (selected as any).serverName);
                return;

            } catch (error) {
                this.log(`Failed to discover servers: ${error}`);
                vscode.window.showErrorMessage(`Failed to discover MCP servers: ${error}`);
                return;
            }
        }

        await this.performScan('server', serverName, serverItem);
    }

    async scanAllServers() {
        await this.performScan('all');
    }

    async scanConfig() {
        await this.performScan('config');
    }

    private async performScan(type: 'server' | 'all' | 'config', serverName?: string, serverItem?: any) {
        const statusBarMessage = vscode.window.setStatusBarMessage('🔍 Ramparts: Scanning...');

        try {
            this.log(`Starting ${type} scan${serverName ? ` for ${serverName}` : ''}...`);

            // Check if ramparts CLI is available
            const rampartsPath = await this.findRampartsCLI();
            if (!rampartsPath) {
                vscode.window.showErrorMessage('Ramparts CLI not found. Please install ramparts first.');
                return;
            }

            let command: string;
            let args: string[] = [];

            switch (type) {
                case 'server':
                    if (!serverName) {
                        throw new Error('Server name required for server scan');
                    }
                    // Find the server configuration
                    const server = this.findServerConfig(serverName);
                    if (!server) {
                        vscode.window.showErrorMessage(`Server ${serverName} not found.`);
                        return;
                    }
                    command = rampartsPath;
                    args = ['scan', server.url || server.command || '', '--format', 'json'];
                    break;
                case 'all':
                    command = rampartsPath;
                    args = ['scan-config', '--format', 'json'];
                    break;
                case 'config':
                    command = rampartsPath;
                    args = ['scan-config', '--format', 'json'];
                    break;
            }

            // Execute the scan
            this.log(`Running: ${command} ${args.map(a => (a.includes(' ') ? `'${a.replace(/'/g, "'\''")}'` : a)).join(' ')}`);

            // Get environment variables from server item or discovery
            let extraEnv: Record<string, string> | undefined;
            if (serverItem?.serverConfig?.env) {
                extraEnv = serverItem.serverConfig.env;
                this.log(`Using server item env: ${JSON.stringify(extraEnv)}`);
            } else {
                extraEnv = this.findServerEnv(serverName);
                this.log(`Found server env for ${serverName}: ${JSON.stringify(extraEnv)}`);
            }

            if (!extraEnv) {
                this.log(`No environment variables found for server ${serverName}`);
            }

            const result = await this.executeScanCommand(command, args, extraEnv);
            await this.displayScanResults(result, type, serverName);

        } catch (error) {
            this.log(`Scan failed: ${error}`);

            // For single server scans, provide a graceful fallback with server info
            if (type === 'server' && serverName) {
                const serverConfig = this.findServerConfig(serverName);
                const fallbackResult = this.createFallbackScanResult(serverName, serverConfig, error);
                await this.displayScanResults(fallbackResult, type, serverName);
            } else {
                vscode.window.showErrorMessage(`Scan failed: ${error}`);
            }
        } finally {
            statusBarMessage.dispose();
        }
    }

    private stopAllProxyProcesses() {
        for (const [name, process] of this.proxyProcesses) {
            this.log(`Stopping proxy process for ${name}`);
            process.kill();
        }
        this.proxyProcesses.clear();
    }

    private log(message: string) {
        const timestamp = new Date().toISOString();
        this.outputChannel.appendLine(`[${timestamp}] ${message}`);
        console.log(`[Ramparts] ${message}`);
    }

    isEnabled(): boolean {
        return this.enabled;
    }

    getStatus(): string {
        return this.status;
    }

    dispose() {
        this.stopAllProxyProcesses();
        this.outputChannel.dispose();
    }

    private async findRampartsCLI(): Promise<string | null> {
        // Use the bundled Ramparts CLI binary from the extension
        const extensionPath = this.context.extensionPath;
        const bundledRampartsPath = path.join(extensionPath, 'bin', 'ramparts');

        if (fs.existsSync(bundledRampartsPath)) {
            // Make sure it's executable
            try {
                fs.chmodSync(bundledRampartsPath, 0o755);
                return bundledRampartsPath;
            } catch (error) {
                this.log(`Failed to make bundled ramparts executable: ${error}`);
            }
        }

        // Fallback: try to find ramparts in PATH (for development)
        const { exec } = require('child_process');
        const { promisify } = require('util');
        const execAsync = promisify(exec);

        try {
            const { stdout } = await execAsync('which ramparts');
            return stdout.trim();
        } catch {
            // Try common installation locations
            const commonPaths = [
                '/usr/local/bin/ramparts',
                '/opt/homebrew/bin/ramparts',
                `${require('os').homedir()}/.cargo/bin/ramparts`,
                `${require('os').homedir()}/.local/bin/ramparts`
            ];

            for (const p of commonPaths) {
                if (fs.existsSync(p)) {
                    return p;
                }
            }
        }

        return null;
    }

    private getDiscoveredServersWithEnv(): Array<{name: string, url?: string, command?: string, type: string, env?: Record<string,string>}> {
        const servers: Array<{name: string, url?: string, command?: string, type: string, env?: Record<string,string>}> = [];

        // Discover MCP servers from known config locations (mirrors tree provider)
        const paths: string[] = [];
        const home = os.homedir();

        // VS Code
        paths.push(path.join(home, '.vscode', 'mcp.json'));
        // Cursor
        paths.push(path.join(home, '.cursor', 'mcp.json'));
        paths.push(path.join(home, 'Library', 'Application Support', 'Cursor', 'User', 'mcp.json'));
        // Workspace-specific
        if (vscode.workspace.workspaceFolders) {
            for (const folder of vscode.workspace.workspaceFolders) {
                const workspacePath = folder.uri.fsPath;
                paths.push(path.join(workspacePath, '.vscode', 'mcp.json'));
                paths.push(path.join(workspacePath, '.cursor', 'mcp.json'));
                paths.push(path.join(workspacePath, '.cursor', 'rules', 'mcp.json'));
            }
        }

        for (const p of paths) {
            if (!fs.existsSync(p)) continue;
            try {
                const content = fs.readFileSync(p, 'utf8');
                const mcpConfig = JSON.parse(content);
                const mcpServers = mcpConfig.mcpServers || mcpConfig.servers || {};
                for (const [name, cfg] of Object.entries<any>(mcpServers)) {
                    // Prefer explicit URLs if present (SSE/HTTP style)
                    const url = (cfg as any)?.url || (cfg as any)?.sse?.url || (cfg as any)?.http?.url;
                    if (url) {
                        servers.push({ name, url, type: 'http', env: (cfg as any)?.env });
                        continue;
                    }
                    // Otherwise, construct stdio command specifier from command + args
                    const cmd = (cfg as any)?.command;
                    const args: string[] = (cfg as any)?.args || [];
                    if (cmd) {
                        const spec = this.buildStdioSpecifier(cmd, args);
                        servers.push({ name, command: spec, type: 'stdio', env: (cfg as any)?.env });
                    }
                }
            } catch (e) {
                this.log(`Failed parsing MCP config at ${p}: ${e}`);
            }
        }

        return servers;
    }

    private getDiscoveredServers(): Array<{name: string, url?: string, command?: string, type: string}> {
        return this.getDiscoveredServersWithEnv().map(({name, url, command, type}) => ({name, url, command, type}));
    }

    private getDiscoveredServersWithSource(): Array<{name: string, url?: string, command?: string, type: string, ide_source: string, config_path: string}> {
        const servers: Array<{name: string, url?: string, command?: string, type: string, ide_source: string, config_path: string}> = [];

        // Discover MCP servers from known config locations with source info
        const paths: string[] = [];
        const home = os.homedir();

        // VS Code
        paths.push(path.join(home, '.vscode', 'mcp.json'));
        // Cursor
        paths.push(path.join(home, '.cursor', 'mcp.json'));
        paths.push(path.join(home, 'Library', 'Application Support', 'Cursor', 'User', 'mcp.json'));
        // Workspace-specific
        if (vscode.workspace.workspaceFolders) {
            for (const folder of vscode.workspace.workspaceFolders) {
                const workspacePath = folder.uri.fsPath;
                paths.push(path.join(workspacePath, '.vscode', 'mcp.json'));
                paths.push(path.join(workspacePath, '.cursor', 'mcp.json'));
                paths.push(path.join(workspacePath, '.cursor', 'rules', 'mcp.json'));
            }
        }

        for (const p of paths) {
            if (!fs.existsSync(p)) continue;
            try {
                const content = fs.readFileSync(p, 'utf8');
                const mcpConfig = JSON.parse(content);
                const mcpServers = mcpConfig.mcpServers || mcpConfig.servers || {};

                // Determine IDE source from path
                let ideSource = 'Unknown';
                if (p.includes('.vscode')) ideSource = 'VS Code';
                else if (p.includes('.cursor')) ideSource = 'Cursor';

                // Determine if workspace or global and format path accordingly
                const workspaceFolder = vscode.workspace.workspaceFolders?.find(folder =>
                    p.startsWith(folder.uri.fsPath)
                );

                let displayPath: string;
                if (workspaceFolder) {
                    ideSource += ' (Workspace)';
                    // Show relative path from workspace root
                    displayPath = path.relative(workspaceFolder.uri.fsPath, p);
                } else {
                    ideSource += ' (Global)';
                    // Show path relative to home
                    displayPath = p.replace(home, '~');
                }

                for (const [name, cfg] of Object.entries<any>(mcpServers)) {
                    // Prefer explicit URLs if present (SSE/HTTP style)
                    const url = (cfg as any)?.url || (cfg as any)?.sse?.url || (cfg as any)?.http?.url;
                    if (url) {
                        servers.push({
                            name,
                            url,
                            type: 'http',
                            ide_source: ideSource,
                            config_path: displayPath
                        });
                        continue;
                    }
                    // Otherwise, construct stdio command specifier from command + args
                    const cmd = (cfg as any)?.command;
                    const args: string[] = (cfg as any)?.args || [];
                    if (cmd) {
                        const spec = this.buildStdioSpecifier(cmd, args);
                        servers.push({
                            name,
                            command: spec,
                            type: 'stdio',
                            ide_source: ideSource,
                            config_path: displayPath
                        });
                    }
                }
            } catch (e) {
                this.log(`Failed parsing MCP config at ${p}: ${e}`);
            }
        }

        return servers;
    }

    private findServerEnv(serverName?: string): Record<string, string> | undefined {
        if (!serverName) return undefined;
        const servers = this.getDiscoveredServersWithEnv();
        const match = servers.find(s => s.name === serverName);
        return match?.env as Record<string,string> | undefined;
    }

    private createFallbackScanResult(serverName: string, serverConfig: any, error: any): any {
        const timestamp = new Date().toISOString();

        // Determine if this looks like a non-MCP target
        const isNonMcpTarget = serverConfig?.env?.RAMPARTS_TARGET_CMD &&
                              !serverConfig?.env?.RAMPARTS_TARGET_CMD.includes('mcp');

        let errorType = 'Unknown Error';
        let suggestion = 'Check server configuration and try again.';

        if (error.toString().includes('serde error expected value') ||
            error.toString().includes('connection closed: initialize response')) {
            if (isNonMcpTarget) {
                errorType = 'Non-MCP Target Detected';
                suggestion = 'This appears to be a non-MCP command wrapped by Ramparts proxy. The target command does not implement the MCP protocol.';
            } else {
                errorType = 'MCP Protocol Error';
                suggestion = 'The server failed to respond with valid MCP protocol messages. Check if the server is properly configured.';
            }
        } else if (error.toString().includes('No such file or directory')) {
            errorType = 'Server Not Found';
            suggestion = 'The server executable was not found. Check the command path and ensure the server is installed.';
        }

        return {
            scan_type: 'single_server',
            server_name: serverName,
            timestamp,
            status: 'Failed',
            error_type: errorType,
            error_message: error.toString(),
            suggestion,
            server_config: {
                command: serverConfig?.command,
                url: serverConfig?.url,
                env: serverConfig?.env,
                type: serverConfig?.url ? 'HTTP/SSE' : 'STDIO'
            },
            tools: [],
            resources: [],
            prompts: [],
            security_issues: {
                scan_failed: true,
                reason: errorType
            }
        };
    }

    private buildStdioSpecifier(command: string, args: string[] = []): string {
        // Handle cases where command is already a stdio specifier
        if (command.startsWith('stdio:')) {
            return command; // Already properly formatted
        }

        // Use the colon-delimited stdio form supported by ramparts (e.g., stdio:node:/path/server.js)
        const parts = [command, ...args];
        return `stdio:${parts.join(':')}`;
    }

    private findServerConfig(serverName: string): {url?: string, command?: string, env?: Record<string,string>} | null {
        const servers = this.getDiscoveredServersWithEnv();
        const match = servers.find(s => s.name === serverName);
        return match ? { url: match.url, command: match.command, env: match.env } : null;
    }

    private async executeScanCommand(command: string, args: string[], extraEnv?: Record<string, string>): Promise<any> {
        const { spawn } = require('child_process');

        const finalEnv = { ...process.env, ...(extraEnv || {}) };
        this.log(`Executing: ${command} ${args.join(' ')}`);
        this.log(`Extra env vars: ${JSON.stringify(extraEnv || {})}`);

        // Log the specific environment variables that the MCP proxy needs
        if (extraEnv) {
            const requiredVars = ['RAMPARTS_TARGET_CMD', 'RAMPARTS_TARGET_ARGS', 'JAVELIN_API_KEY'];
            for (const varName of requiredVars) {
                if (finalEnv[varName]) {
                    this.log(`${varName}=${finalEnv[varName]}`);
                } else {
                    this.log(`WARNING: Missing required env var: ${varName}`);
                }
            }
        }

        return new Promise((resolve, reject) => {
            const child = spawn(command, args, {
                env: finalEnv,
                shell: false
            });

            let stdout = '';
            let stderr = '';

            child.stdout.on('data', (data: Buffer) => {
                stdout += data.toString();
            });

            child.stderr.on('data', (data: Buffer) => {
                const s = data.toString();
                stderr += s;
                this.log(s.trim());
            });

            child.on('error', (err: any) => {
                reject(new Error(`Failed to start process: ${err.message}`));
            });

            child.on('close', (code: number) => {
                if (code !== 0) {
                    // Even on non-zero exit, sometimes JSON is printed. Try to salvage it first.
                    const salvaged = this.tryParseJson(stdout);
                    if (salvaged !== null) {
                        return resolve(salvaged);
                    }
                    return reject(new Error(`Process exited with code ${code}: ${stderr || stdout}`));
                }
                try {
                    resolve(JSON.parse(stdout));
                } catch (e: any) {
                    // Ramparts prints a banner and logs before JSON. Try extracting the JSON portion.
                    const salvaged = this.tryParseJson(stdout);
                    if (salvaged !== null) {
                        return resolve(salvaged);
                    }
                    reject(new Error(`Failed to parse JSON: ${e.message}. Output: ${stdout}`));
                }
            });
        });
    }

    private tryParseJson(output: string): any | null {
        // Ramparts CLI outputs a banner and logs before JSON. Try to find and extract the JSON.
        try {
            // First try parsing the whole output
            return JSON.parse(output);
        } catch {
            // Look for JSON starting with { and ending with }
            const lines = output.split('\n');
            let jsonStart = -1;
            let jsonEnd = -1;
            let braceCount = 0;

            for (let i = 0; i < lines.length; i++) {
                const line = lines[i].trim();
                if (line.startsWith('{') && jsonStart === -1) {
                    jsonStart = i;
                    braceCount = 1;
                } else if (jsonStart !== -1) {
                    for (const char of line) {
                        if (char === '{') braceCount++;
                        if (char === '}') braceCount--;
                        if (braceCount === 0) {
                            jsonEnd = i;
                            break;
                        }
                    }
                    if (jsonEnd !== -1) break;
                }
            }

            if (jsonStart !== -1 && jsonEnd !== -1) {
                const jsonLines = lines.slice(jsonStart, jsonEnd + 1);
                const jsonStr = jsonLines.join('\n');
                try {
                    return JSON.parse(jsonStr);
                } catch {
                    return null;
                }
            }

            return null;
        }
    }

    private async displayScanResults(results: any, type: string, serverName?: string) {
        // Create a new document to display scan results
        const doc = await vscode.workspace.openTextDocument({
            content: this.formatScanResults(results, type, serverName),
            language: 'markdown'
        });

        await vscode.window.showTextDocument(doc);

        // Also show summary in notification
        const summary = this.getScanSummary(results);
        vscode.window.showInformationMessage(`Scan completed: ${summary}`);
    }

    private formatScanResults(results: any, type: string, serverName?: string): string {
        const timestamp = results.timestamp || new Date().toISOString();
        let content = `# Ramparts Security Scan Results\n\n`;
        content += `**Scan Type:** ${type === 'server' ? 'Single Server' : type}\n`;
        content += `**Timestamp:** ${timestamp}\n`;

        if (results.server_name) {
            content += `**Server:** ${results.server_name}\n`;
        } else if (serverName) {
            content += `**Server:** ${serverName}\n`;
        }

        content += `\n`;

        // Add summary table for multi-server scans
        if (results.results && Array.isArray(results.results) && results.results.length > 1) {
            content += this.createSummaryTable(results.results);
        }

        content += `\n---\n\n`;

        // Handle fallback/error results (single server that failed)
        if (results.status === 'Failed' && results.error_type) {
            content += `## ❌ ${results.error_type}\n\n`;
            content += `**Status:** ${results.status}\n\n`;
            content += `### Error Details\n\n`;
            content += `${results.suggestion}\n\n`;

            if (results.server_config) {
                content += `### Server Configuration\n\n`;
                content += `- **Type:** ${results.server_config.type}\n`;
                if (results.server_config.command) {
                    content += `- **Command:** \`${results.server_config.command}\`\n`;
                }
                if (results.server_config.url) {
                    content += `- **URL:** ${results.server_config.url}\n`;
                }
                if (results.server_config.env) {
                    content += `- **Environment Variables:**\n`;
                    for (const [key, value] of Object.entries(results.server_config.env)) {
                        content += `  - \`${key}=${value}\`\n`;
                    }
                }
                content += `\n`;
            }

            content += `### Technical Error\n\n`;
            content += `\`\`\`\n${results.error_message}\n\`\`\`\n\n`;
            return content;
        }

        // Handle normal scan results
        const resultsArray = results.results || (Array.isArray(results) ? results : [results]);

        if (resultsArray.length === 0) {
            content += `## ⚠️ No Results\n\nNo servers found or all scans failed.\n\n`;
            return content;
        }

        for (const result of resultsArray) {
            content += this.formatSingleScanResult(result);
            if (resultsArray.length > 1) {
                content += `\n---\n\n`;
            }
        }

        return content;
    }

    private createSummaryTable(results: any[]): string {
        let table = `## 📊 Summary\n\n`;
        table += `| Server                                    | Status      | Issues | Tools | Resources | Error         |\n`;
        table += `|-------------------------------------------|-------------|--------|-------|-----------|---------------|\n`;

        for (const result of results) {
            const serverInfo = this.getServerDisplayInfo(result);
            const status = this.getServerStatus(result);
            const issuesCount = this.getIssuesCount(result);
            const toolsCount = result.tools ? result.tools.length : 0;
            const resourcesCount = result.resources ? result.resources.length : 0;
            const error = this.getServerError(result);

            // Format with proper padding for alignment (trim to fit column width)
            const serverCol = serverInfo.length > 41 ? serverInfo.substring(0, 38) + '...' : serverInfo.padEnd(41);
            const statusCol = status.padEnd(11);
            const issuesCol = issuesCount.toString().padEnd(6);
            const toolsCol = toolsCount.toString().padEnd(5);
            const resourcesCol = resourcesCount.toString().padEnd(9);
            const errorCol = error.length > 13 ? error.substring(0, 10) + '...' : error.padEnd(13);

            table += `| ${serverCol} | ${statusCol} | ${issuesCol} | ${toolsCol} | ${resourcesCol} | ${errorCol} |\n`;
        }

        table += `\n`;
        return table;
    }

    private getServerDisplayName(result: any): string {
        if (result.url) {
            // Extract server name from URL
            if (result.url.includes('github')) return 'GitHub Copilot';
            if (result.url.includes('huggingface') || result.url.includes('hf.co')) return 'Hugging Face';
            if (result.url.includes('deepwiki')) return 'DeepWiki';
            return result.url.replace(/^https?:\/\//, '').split('/')[0];
        }
        if (result.server_info?.name) return result.server_info.name;
        return 'Unknown';
    }

    private getServerDisplayInfo(result: any): string {
        let name = this.getServerDisplayName(result);
        let url = result.url || '';

        // Format: "Name (url)" or just "Name" if no URL
        if (url) {
            const shortUrl = url.length > 25 ? url.substring(0, 22) + '...' : url;
            return `${name} (${shortUrl})`;
        }
        return name;
    }

    private getServerStatus(result: any): string {
        if (result.status === 'Success') return '✅ Success';
        if (result.status && typeof result.status === 'object' && result.status.Failed) return '❌ Failed';
        if (result.errors && result.errors.length > 0) return '❌ Error';
        return '⚠️ Unknown';
    }

    private getIssuesCount(result: any): string {
        if (!result.security_issues) return '0';

        let count = 0;
        if (result.security_issues.tool_issues) count += result.security_issues.tool_issues.length;
        if (result.security_issues.prompt_issues) count += result.security_issues.prompt_issues.length;
        if (result.security_issues.resource_issues) count += result.security_issues.resource_issues.length;

        // Add YARA rule matches
        if (result.yara_results) {
            const warnings = result.yara_results.filter((r: any) => r.status === 'warning');
            count += warnings.length;
        }

        return count > 0 ? `⚠️ ${count}` : '✅ 0';
    }

    private getServerError(result: any): string {
        if (result.status === 'Success') return '-';
        if (result.status && typeof result.status === 'object' && result.status.Failed) {
            const error = result.status.Failed;
            if (error.includes('401')) return '401 Auth';
            if (error.includes('404')) return '404 Not Found';
            if (error.includes('connection')) return 'Connection';
            return 'Failed';
        }
        if (result.errors && result.errors.length > 0) {
            return result.errors[0].substring(0, 20) + '...';
        }
        return '-';
    }

    private formatSingleScanResult(result: any): string {
        let content = `## ${result.url || result.server_name || 'Unknown Server'}\n\n`;

        if (result.status) {
            content += `**Status:** ${result.status}\n\n`;
        }

        if (result.security_issues && result.security_issues.length > 0) {
            content += `### 🚨 Security Issues (${result.security_issues.length})\n\n`;
            for (const issue of result.security_issues) {
                content += `- **${issue.severity}**: ${issue.title}\n`;
                if (issue.description) {
                    content += `  - ${issue.description}\n`;
                }
            }
            content += `\n`;
        } else {
            content += `### ✅ No Security Issues Found\n\n`;
        }

        if (result.tools && result.tools.length > 0) {
            content += `### 🔧 Tools (${result.tools.length})\n\n`;
            for (const tool of result.tools) {
                content += `- **${tool.name}**: ${tool.description || 'No description'}\n`;
            }
            content += `\n`;
        }

        if (result.resources && result.resources.length > 0) {
            content += `### 📁 Resources (${result.resources.length})\n\n`;
            for (const resource of result.resources) {
                content += `- **${resource.name}**: ${resource.description || 'No description'}\n`;
            }
            content += `\n`;
        }

        return content;
    }

    private getScanSummary(results: any): string {
        if (Array.isArray(results)) {
            const totalIssues = results.reduce((sum, result) =>
                sum + (result.security_issues ? result.security_issues.length : 0), 0);
            return `${results.length} servers scanned, ${totalIssues} security issues found`;
        } else {
            const issueCount = results.security_issues ? results.security_issues.length : 0;
            return `${issueCount} security issues found`;
        }
    }

    async autoProxyAllServers() {
        this.log('Auto-proxying all MCP servers...');
        try {
            const { McpConfigManager } = require('./mcpConfigManager');
            const mcpManager = new McpConfigManager(this.context);
            await mcpManager.reapplyProxies();
            vscode.window.showInformationMessage('All MCP servers have been wrapped with Ramparts proxy for runtime protection.');
        } catch (error) {
            this.log(`Failed to auto-proxy servers: ${error}`);
            vscode.window.showErrorMessage(`Failed to auto-proxy servers: ${error}`);
        }
    }

    async reloadPolicy() {
        this.log('Reloading security policy...');
        const config = vscode.workspace.getConfiguration('ramparts');
        const policyFile = config.get<string>('policyFile');

        if (!policyFile) {
            vscode.window.showWarningMessage('No policy file configured. Set ramparts.policyFile in settings.');
            return;
        }

        if (!fs.existsSync(policyFile)) {
            vscode.window.showErrorMessage(`Policy file not found: ${policyFile}`);
            return;
        }

        try {
            // Re-apply proxies with updated policy
            const { McpConfigManager } = require('./mcpConfigManager');
            const mcpManager = new McpConfigManager(this.context);
            await mcpManager.reapplyProxies();

            this.log(`Policy reloaded from: ${policyFile}`);
            vscode.window.showInformationMessage(`Security policy reloaded successfully from ${policyFile}`);
        } catch (error) {
            this.log(`Failed to reload policy: ${error}`);
            vscode.window.showErrorMessage(`Failed to reload policy: ${error}`);
        }
    }

    async testJavelinConnection() {
        this.log('Testing Javelin API connection...');
        const config = vscode.workspace.getConfiguration('ramparts');

        let apiKey = config.get<string>('javelinApiKey');
        if (!apiKey || apiKey.trim() === '') {
            apiKey = process.env.JAVELIN_API_KEY;
        }

        if (!apiKey || apiKey.trim() === '') {
            vscode.window.showWarningMessage('No Javelin API key configured. Set ramparts.javelinApiKey or JAVELIN_API_KEY environment variable.');
            return;
        }

        const endpoint = config.get<string>('javelinEndpoint', 'https://api.getjavelin.com');

        try {
            // Use Node.js https module instead of fetch for compatibility
            const https = require('https');
            const url = require('url');

            const testUrl = `${endpoint}/health`;
            const parsedUrl = url.parse(testUrl);

            const options = {
                hostname: parsedUrl.hostname,
                port: parsedUrl.port || 443,
                path: parsedUrl.path,
                method: 'GET',
                headers: {
                    'Authorization': `Bearer ${apiKey}`,
                    'User-Agent': 'Ramparts-MCP-Guardian/0.1.0'
                }
            };

            const statusBarMessage = vscode.window.setStatusBarMessage('🔍 Testing Javelin connection...');

            const response = await new Promise<{statusCode: number, data: string}>((resolve, reject) => {
                const req = https.request(options, (res: any) => {
                    let data = '';
                    res.on('data', (chunk: any) => data += chunk);
                    res.on('end', () => resolve({ statusCode: res.statusCode, data }));
                });
                req.on('error', reject);
                req.setTimeout(10000, () => reject(new Error('Request timeout')));
                req.end();
            });

            statusBarMessage.dispose();

            if (response.statusCode === 200) {
                this.log(`Javelin connection successful: ${testUrl}`);
                vscode.window.showInformationMessage(`✅ Javelin API connection successful!\nEndpoint: ${endpoint}\nStatus: Connected`);
            } else {
                this.log(`Javelin connection failed: HTTP ${response.statusCode}`);
                vscode.window.showWarningMessage(`⚠️ Javelin API returned HTTP ${response.statusCode}\nEndpoint: ${endpoint}\nCheck your API key and endpoint configuration.`);
            }
        } catch (error) {
            this.log(`Javelin connection error: ${error}`);
            vscode.window.showErrorMessage(`❌ Failed to connect to Javelin API\nEndpoint: ${endpoint}\nError: ${error}\n\nCheck your network connection and API configuration.`);
        }
    }
}
