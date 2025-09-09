import * as vscode from 'vscode';
import { RampartsManager } from './rampartsManager';
import { McpConfigManager } from './mcpConfigManager';
import { RampartsStatusBar } from './statusBar';
import { RampartsTreeProvider } from './treeProvider';

let rampartsManager: RampartsManager;
let mcpConfigManager: McpConfigManager;
let statusBar: RampartsStatusBar;
let treeProvider: RampartsTreeProvider;

export async function activate(context: vscode.ExtensionContext) {
    console.log('Ramparts MCP Guardian is activating...');

    // Initialize managers
    rampartsManager = new RampartsManager(context);
    mcpConfigManager = new McpConfigManager(context);
    statusBar = new RampartsStatusBar();
    treeProvider = new RampartsTreeProvider(rampartsManager);

    // Register tree view
    vscode.window.createTreeView('rampartsServers', {
        treeDataProvider: treeProvider,
        showCollapseAll: true
    });

    // Register commands
    const commands = [
        vscode.commands.registerCommand('ramparts.enable', async () => {
            await rampartsManager.enable();
            statusBar.updateStatus(true, 'Enabled');
            treeProvider.refresh();
        }),

        vscode.commands.registerCommand('ramparts.disable', async () => {
            await rampartsManager.disable();
            statusBar.updateStatus(false, 'Disabled');
            treeProvider.refresh();
        }),

        vscode.commands.registerCommand('ramparts.status', () => {
            rampartsManager.showStatus();
        }),

        vscode.commands.registerCommand('ramparts.openSettings', () => {
            vscode.commands.executeCommand('workbench.action.openSettings', 'ramparts');
        }),

        vscode.commands.registerCommand('ramparts.viewLogs', () => {
            rampartsManager.showLogs();
        }),

        vscode.commands.registerCommand('ramparts.bypassServer', async (serverName: string) => {
            await rampartsManager.bypassServer(serverName);
            treeProvider.refresh();
        }),

        vscode.commands.registerCommand('ramparts.unbypassServer', async (serverName: string) => {
            await rampartsManager.unbypassServer(serverName);
            treeProvider.refresh();
        }),

        vscode.commands.registerCommand('ramparts.scanServer', async (item?: any) => {
            // Handle both string serverName and McpServerItem from context menu
            const serverName = typeof item === 'string' ? item : item?.serverName;
            await rampartsManager.scanServer(item);
        }),

        vscode.commands.registerCommand('ramparts.scanAllServers', async () => {
            await rampartsManager.scanAllServers();
        }),

        vscode.commands.registerCommand('ramparts.scanConfig', async () => {
            await rampartsManager.scanConfig();
        }),

        vscode.commands.registerCommand('ramparts.autoProxyAll', async () => {
            await rampartsManager.autoProxyAllServers();
        }),

        vscode.commands.registerCommand('ramparts.reloadPolicy', async () => {
            await rampartsManager.reloadPolicy();
        }),

        vscode.commands.registerCommand('ramparts.testJavelinConnection', async () => {
            await rampartsManager.testJavelinConnection();
        })
    ];

    context.subscriptions.push(...commands, statusBar, treeProvider);

    // Watch for configuration changes
    context.subscriptions.push(
        vscode.workspace.onDidChangeConfiguration(async (e) => {
            if (e.affectsConfiguration('ramparts')) {
                await rampartsManager.reloadConfiguration();
                statusBar.updateStatus(rampartsManager.isEnabled(), rampartsManager.getStatus());
                treeProvider.refresh();
            }
        })
    );

    // Auto-enable if configured
    const config = vscode.workspace.getConfiguration('ramparts');
    if (config.get<boolean>('enabled', true)) {
        await rampartsManager.enable();
        statusBar.updateStatus(true, 'Enabled');
    } else {
        statusBar.updateStatus(false, 'Disabled');
    }

    // Start watching for MCP config changes and policy reloads
    await mcpConfigManager.startWatching();

    // Run scan on IDE startup if enabled
    if (config.get<boolean>('startupScan', true)) {
        try {
            await rampartsManager.scanConfig();
        } catch (e) {
            console.error('Startup scan failed:', e);
        }
    }

    console.log('Ramparts MCP Guardian activated successfully');
}

export function deactivate() {
    console.log('Ramparts MCP Guardian is deactivating...');
    
    if (rampartsManager) {
        rampartsManager.dispose();
    }
    
    if (mcpConfigManager) {
        mcpConfigManager.dispose();
    }
}
