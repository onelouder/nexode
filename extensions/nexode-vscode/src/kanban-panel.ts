import * as vscode from 'vscode';

import type { WebviewToHostMessage } from '../webview/shared/types';
import { DaemonClient } from './daemon-client';
import { createMoveTaskCommand } from './kanban-commands';
import { StateCache } from './state';
import { configureWebview, createStateMessage } from './webview-support';

const KANBAN_VIEW_TYPE = 'nexode.kanban';

export class KanbanPanel implements vscode.Disposable {
  private panel?: vscode.WebviewPanel;
  private ready = false;
  private readonly stateSubscription;

  public constructor(
    private readonly extensionUri: vscode.Uri,
    private readonly state: StateCache,
    private readonly client: DaemonClient,
  ) {
    this.stateSubscription = this.state.onDidChange(() => {
      void this.postState();
    });
  }

  public show(): void {
    if (this.panel) {
      this.panel.reveal();
      void this.postState();
      return;
    }

    const panel = vscode.window.createWebviewPanel(
      KANBAN_VIEW_TYPE,
      'Nexode: Macro Kanban',
      vscode.ViewColumn.Beside,
      {
        enableScripts: true,
        retainContextWhenHidden: true,
      },
    );

    this.ready = false;
    this.panel = panel;
    panel.webview.onDidReceiveMessage((message: WebviewToHostMessage) => {
      void this.handleMessage(message);
    });

    configureWebview(panel.webview, this.extensionUri, {
      bundleName: 'kanban',
      surface: 'macro-kanban',
      title: 'Nexode Macro Kanban',
    });

    panel.onDidDispose(() => {
      this.panel = undefined;
      this.ready = false;
    });
  }

  public dispose(): void {
    this.stateSubscription.dispose();
    this.panel?.dispose();
  }

  private async handleMessage(message: WebviewToHostMessage): Promise<void> {
    if (message.type === 'ready') {
      this.ready = true;
      await this.postState();
      return;
    }

    if (message.type !== 'moveTask') {
      return;
    }

    try {
      await this.client.dispatchCommand(createMoveTaskCommand(message));
    } catch (error) {
      const detail = error instanceof Error ? error.message : 'Failed to dispatch Kanban move';
      void vscode.window.showErrorMessage(detail);
    }
  }

  private async postState(): Promise<void> {
    if (!this.panel || !this.ready) {
      return;
    }

    await this.panel.webview.postMessage(createStateMessage(this.state, 'macro-kanban'));
  }
}
