import * as vscode from 'vscode';

export class RampartsStatusBar {
    private statusBarItem: vscode.StatusBarItem;

    constructor() {
        this.statusBarItem = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Right,
            100
        );
        this.statusBarItem.command = 'ramparts.status';
        this.statusBarItem.show();
    }

    updateStatus(enabled: boolean, status: string) {
        if (enabled) {
            this.statusBarItem.text = `$(shield) Ramparts: ${status}`;
            this.statusBarItem.backgroundColor = undefined;
            this.statusBarItem.tooltip = 'Ramparts MCP Guardian is protecting your MCP requests. Click for details.';
        } else {
            this.statusBarItem.text = `$(shield) Ramparts: Disabled`;
            this.statusBarItem.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
            this.statusBarItem.tooltip = 'Ramparts MCP Guardian is disabled. MCP requests are not protected. Click for details.';
        }
    }

    dispose() {
        this.statusBarItem.dispose();
    }
}
