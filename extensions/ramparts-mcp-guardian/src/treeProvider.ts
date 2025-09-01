import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { RampartsManager } from './rampartsManager';

export class RampartsTreeProvider implements vscode.TreeDataProvider<McpServerItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<McpServerItem | undefined | null | void> = new vscode.EventEmitter<McpServerItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<McpServerItem | undefined | null | void> = this._onDidChangeTreeData.event;

    constructor(private rampartsManager: RampartsManager) {}

    refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    dispose(): void {
        // Clean up any resources if needed
    }

    getTreeItem(element: McpServerItem): vscode.TreeItem {
        return element;
    }

    getChildren(element?: McpServerItem): Thenable<McpServerItem[]> {
        if (!element) {
            return Promise.resolve(this.getServers());
        }
        return Promise.resolve([]);
    }

    private getServers(): McpServerItem[] {
        const servers: McpServerItem[] = [];
        const config = vscode.workspace.getConfiguration('ramparts');
        const bypassedServers = config.get<string[]>('bypassedServers', []);
        
        // Discover MCP servers from various config files
        const configPaths = this.getConfigPaths();
        
        for (const configPath of configPaths) {
            if (fs.existsSync(configPath)) {
                try {
                    const content = fs.readFileSync(configPath, 'utf8');
                    const mcpConfig = JSON.parse(content);
                    
                    if (mcpConfig.mcpServers) {
                        for (const [serverName, serverConfig] of Object.entries(mcpConfig.mcpServers)) {
                            const isBypassed = bypassedServers.includes(serverName);
                            const isProxied = (serverConfig as any).command?.includes('ramparts-mcp-proxy');
                            
                            servers.push(new McpServerItem(
                                serverName,
                                serverConfig as any,
                                configPath,
                                isBypassed,
                                isProxied
                            ));
                        }
                    }
                } catch (error) {
                    console.error(`Failed to parse config ${configPath}:`, error);
                }
            }
        }
        
        return servers;
    }

    private getConfigPaths(): string[] {
        const paths: string[] = [];
        const home = os.homedir();

        // VS Code paths
        paths.push(path.join(home, '.vscode', 'mcp.json'));
        
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
}

export class McpServerItem extends vscode.TreeItem {
    constructor(
        public readonly serverName: string,
        public readonly serverConfig: any,
        public readonly configPath: string,
        public readonly isBypassed: boolean,
        public readonly isProxied: boolean
    ) {
        super(serverName, vscode.TreeItemCollapsibleState.None);
        
        this.tooltip = this.getTooltip();
        this.description = this.getDescription();
        this.iconPath = this.getIcon();
        this.contextValue = this.getContextValue();
    }

    private getTooltip(): string {
        const status = this.isBypassed ? 'Bypassed' : (this.isProxied ? 'Protected' : 'Unprotected');
        return `${this.serverName}\nStatus: ${status}\nConfig: ${this.configPath}\nCommand: ${this.serverConfig.command || 'N/A'}`;
    }

    private getDescription(): string {
        if (this.isBypassed) {
            return 'Bypassed';
        } else if (this.isProxied) {
            return 'Protected';
        } else {
            return 'Unprotected';
        }
    }

    private getIcon(): vscode.ThemeIcon {
        if (this.isBypassed) {
            return new vscode.ThemeIcon('circle-slash', new vscode.ThemeColor('charts.orange'));
        } else if (this.isProxied) {
            return new vscode.ThemeIcon('shield', new vscode.ThemeColor('charts.green'));
        } else {
            return new vscode.ThemeIcon('warning', new vscode.ThemeColor('charts.red'));
        }
    }

    private getContextValue(): string {
        if (this.isBypassed) {
            return 'mcpServerBypassed';
        } else {
            return 'mcpServerProtected';
        }
    }
}
