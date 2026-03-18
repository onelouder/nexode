import * as vscode from 'vscode';

import { StateCache, formatTokenCount } from './state';

export class NexodeStatusBar implements vscode.Disposable {
  private readonly item: vscode.StatusBarItem;
  private readonly disposables: vscode.Disposable[] = [];

  public constructor(private readonly state: StateCache) {
    this.item = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
    this.item.name = 'Nexode';
    this.item.command = 'nexode.focusSlots';

    this.disposables.push(
      this.item,
      this.state.onDidChange(() => {
        this.render();
      }),
    );

    this.render();
    this.item.show();
  }

  public dispose(): void {
    vscode.Disposable.from(...this.disposables).dispose();
  }

  private render(): void {
    const status = this.state.getConnectionStatus();

    if (status.state === 'connected') {
      const metrics = this.state.getAggregateMetrics();
      this.item.text = `$(plug) Nexode: Connected · ${metrics.agentCount} agents · ${formatTokenCount(metrics.totalTokens)} tok`;
      this.item.tooltip = `Session cost $${metrics.totalSessionCost.toFixed(2)} / $${this.state
        .getSessionBudgetMaxUsd()
        .toFixed(2)}`;
      return;
    }

    if (status.state === 'reconnecting') {
      this.item.text = `$(sync~spin) Nexode: Reconnecting`;
      this.item.tooltip = status.detail || 'Trying to reconnect to the Nexode daemon';
      return;
    }

    this.item.text = '$(debug-disconnect) Nexode: Disconnected';
    this.item.tooltip = status.detail || 'Nexode daemon is disconnected';
  }
}
