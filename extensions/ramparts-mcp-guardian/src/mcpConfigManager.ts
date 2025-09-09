import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { watch } from 'chokidar';

export interface McpServerConfig {
    name?: string;
    command?: string;
    args?: string[];
    env?: Record<string, string>;
    url?: string;
    type?: string;
    description?: string;
    headers?: Record<string, string>;
}

export interface McpConfig {
    mcpServers?: Record<string, McpServerConfig>;
    servers?: McpServerConfig[];
    [key: string]: any;
}

export class McpConfigManager {
    private context: vscode.ExtensionContext;
    private watchers: any[] = [];
    private originalConfigs: Map<string, string> = new Map();
    private modifiedConfigs: Set<string> = new Set();

    constructor(context: vscode.ExtensionContext) {
        this.context = context;
    }

    async startWatching() {
        const configPaths = this.getConfigPaths();

        for (const configPath of configPaths) {
            if (fs.existsSync(configPath)) {
                console.log(`Watching MCP config: ${configPath}`);

                // Backup original config
                await this.backupOriginalConfig(configPath);

                // Apply ramparts proxy if enabled
                await this.applyRampartsProxy(configPath);

                // Watch for changes
                const watcher = watch(configPath, { persistent: false });
                watcher.on('change', () => this.onConfigChanged(configPath));
                this.watchers.push(watcher);
            }
        }

        // Also watch the policy file if configured
        this.watchPolicyFile();
    }

    async reapplyProxies() {
        const configPaths = this.getConfigPaths();
        for (const configPath of configPaths) {
            if (fs.existsSync(configPath)) {
                await this.applyRampartsProxy(configPath);
            }
        }
        // Refresh policy watcher as well
        this.watchPolicyFile(true);
    }

    private watchPolicyFile(forceRestart: boolean = false) {
        const config = vscode.workspace.getConfiguration('ramparts');
        const policyPath = config.get<string>('policyFile');
        if (!policyPath) return;

        // Restart watchers if requested
        if (forceRestart) {
            this.watchers = this.watchers.filter(w => {
                if (w.__type === 'policy') {
                    try { w.close(); } catch {}
                    return false;
                }
                return true;
            });
        }

        try {
            if (fs.existsSync(policyPath)) {
                const policyWatcher = watch(policyPath, { persistent: false });
                (policyWatcher as any).__type = 'policy';
                policyWatcher.on('change', async () => {
                    console.log(`Policy file changed: ${policyPath}`);
                    await this.reapplyProxies();
                });
                this.watchers.push(policyWatcher);
            }
        } catch (e) {
            console.error('Failed to watch policy file:', e);
        }
    }

    private async onConfigChanged(configPath: string) {
        // If we modified this config, ignore the change to avoid loops
        if (this.modifiedConfigs.has(configPath)) {
            this.modifiedConfigs.delete(configPath);
            return;
        }

        console.log(`MCP config changed: ${configPath}`);
        
        // Re-backup and re-apply proxy
        await this.backupOriginalConfig(configPath);
        await this.applyRampartsProxy(configPath);
    }

    private async backupOriginalConfig(configPath: string) {
        try {
            const content = fs.readFileSync(configPath, 'utf8');
            this.originalConfigs.set(configPath, content);
        } catch (error) {
            console.error(`Failed to backup config ${configPath}:`, error);
        }
    }

    async applyRampartsProxy(configPath: string) {
        const config = vscode.workspace.getConfiguration('ramparts');
        if (!config.get<boolean>('enabled', true) || !config.get<boolean>('autoProxy', true)) {
            return;
        }

        try {
            const content = fs.readFileSync(configPath, 'utf8');
            const mcpConfig: McpConfig = JSON.parse(content);
            let modified = false;

            // Handle different config formats
            if (mcpConfig.mcpServers) {
                // Cursor/VS Code format
                for (const [serverName, serverConfig] of Object.entries(mcpConfig.mcpServers)) {
                    if (this.shouldProxyServer(serverName, serverConfig)) {
                        this.wrapServerWithProxy(serverConfig);
                        modified = true;
                    }
                }
            } else if (mcpConfig.servers) {
                // Array format
                for (const serverConfig of mcpConfig.servers) {
                    const serverName = serverConfig.name || 'unnamed';
                    if (this.shouldProxyServer(serverName, serverConfig)) {
                        this.wrapServerWithProxy(serverConfig);
                        modified = true;
                    }
                }
            }

            if (modified) {
                this.modifiedConfigs.add(configPath);
                fs.writeFileSync(configPath, JSON.stringify(mcpConfig, null, 2));
                console.log(`Applied Ramparts proxy to ${configPath}`);
            }
        } catch (error) {
            console.error(`Failed to apply proxy to ${configPath}:`, error);
        }
    }

    private shouldProxyServer(serverName: string, serverConfig: McpServerConfig): boolean {
        const config = vscode.workspace.getConfiguration('ramparts');
        const bypassedServers = config.get<string[]>('bypassedServers', []);

        // Skip if server is bypassed
        if (bypassedServers.includes(serverName)) {
            console.log(`Skipping proxy for bypassed server: ${serverName}`);
            return false;
        }

        // Skip if already using ramparts proxy
        if (serverConfig.command?.includes('ramparts-mcp-proxy')) {
            console.log(`Server ${serverName} already proxied`);
            return false;
        }

        // Proxy stdio servers (command-based)
        if (serverConfig.command) {
            console.log(`Will proxy stdio server: ${serverName}`);
            return true;
        }

        // For HTTP/SSE servers, we'll log but not proxy yet (future enhancement)
        if (serverConfig.url) {
            console.log(`HTTP/SSE server ${serverName} found but not proxied yet (future feature)`);
            return false;
        }

        return false;
    }

    private wrapServerWithProxy(serverConfig: McpServerConfig) {
        if (!serverConfig.command) return;

        // Store original command and args in environment
        const originalCommand = serverConfig.command;
        const originalArgs = serverConfig.args || [];
        
        // Update to use ramparts proxy
        serverConfig.command = this.getRampartsProxyPath();
        serverConfig.env = serverConfig.env || {};
        serverConfig.env.RAMPARTS_TARGET_CMD = originalCommand;
        serverConfig.env.RAMPARTS_TARGET_ARGS = JSON.stringify(originalArgs);
        
        // Pass through ramparts configuration
        const config = vscode.workspace.getConfiguration('ramparts');
        const apiKey = config.get<string>('javelinApiKey');
        if (apiKey && apiKey.trim() !== '') {
            serverConfig.env.JAVELIN_API_KEY = apiKey;
        } else {
            // Use test mode if no API key is configured
            serverConfig.env.JAVELIN_API_KEY = 'test-mode';
        }
        if (config.get<boolean>('failOpen')) {
            serverConfig.env.RAMPARTS_FAIL_OPEN = 'true';
        }
        if (config.get<string>('logLevel')) {
            serverConfig.env.RUST_LOG = config.get<string>('logLevel')!;
        }
        if (config.get<string>('policyFile')) {
            serverConfig.env.RAMPARTS_POLICY_FILE = config.get<string>('policyFile')!;
        }
        if (config.get<string>('javelinEndpoint')) {
            serverConfig.env.JAVELIN_BASE_URL = config.get<string>('javelinEndpoint')!;
        }

        // Clear original args since they're now in env
        serverConfig.args = [];
    }

    private getRampartsProxyPath(): string {
        // Try to find ramparts-mcp-proxy-stdio in PATH or extension bundle
        const extensionPath = this.context.extensionPath;
        const bundledProxy = path.join(extensionPath, 'bin', 'ramparts-mcp-proxy-stdio');
        
        if (fs.existsSync(bundledProxy)) {
            return bundledProxy;
        }

        // Fallback to PATH
        return 'ramparts-mcp-proxy-stdio';
    }

    async restoreOriginalConfigs() {
        for (const [configPath, originalContent] of this.originalConfigs) {
            try {
                fs.writeFileSync(configPath, originalContent);
                console.log(`Restored original config: ${configPath}`);
            } catch (error) {
                console.error(`Failed to restore config ${configPath}:`, error);
            }
        }
        this.originalConfigs.clear();
        this.modifiedConfigs.clear();
    }

    private getConfigPaths(): string[] {
        const paths: string[] = [];
        const home = os.homedir();

        // VS Code paths
        paths.push(path.join(home, '.vscode', 'mcp.json'));
        paths.push(path.join(home, '.vscode', 'settings.json'));

        // Cursor paths  
        paths.push(path.join(home, '.cursor', 'mcp.json'));
        paths.push(path.join(home, 'Library', 'Application Support', 'Cursor', 'User', 'mcp.json'));

        // Workspace-specific paths
        if (vscode.workspace.workspaceFolders) {
            for (const folder of vscode.workspace.workspaceFolders) {
                const workspacePath = folder.uri.fsPath;
                paths.push(path.join(workspacePath, '.vscode', 'mcp.json'));
                paths.push(path.join(workspacePath, '.cursor', 'mcp.json'));
                paths.push(path.join(workspacePath, '.cursor', 'rules', 'mcp.json'));
            }
        }

        return paths;
    }

    dispose() {
        // Stop all watchers
        for (const watcher of this.watchers) {
            watcher.close();
        }
        this.watchers = [];

        // Restore original configs
        this.restoreOriginalConfigs();
    }
}
