import * as vscode from 'vscode';

import type { WebviewToHostMessage } from '../webview/shared/types';
import type { DaemonClient } from './daemon-client';
import { StateCache } from './state';
import { configureWebview, createStateMessage } from './webview-support';

const SYNAPSE_GRID_VIEW_TYPE = 'nexode.synapseGrid';

export class SynapseGridPanel implements vscode.Disposable {
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
      SYNAPSE_GRID_VIEW_TYPE,
      'Nexode: Synapse Grid',
      vscode.ViewColumn.One,
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
      bundleName: 'synapse-grid',
      surface: 'synapse-grid',
      title: 'Nexode Synapse Grid',
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

    if (message.type === 'viewSlotOutput') {
      await vscode.commands.executeCommand('nexode.showSlotOutput');
      return;
    }

    if (message.type === 'openSlotDiff') {
      await vscode.commands.executeCommand('nexode.openSlotDiff', message.slotId);
    }
  }

  private async postState(): Promise<void> {
    if (!this.panel || !this.ready) {
      return;
    }

    await this.panel.webview.postMessage(createStateMessage(this.state, 'synapse-grid'));
  }
}

export class SynapseSidebarProvider implements vscode.WebviewViewProvider, vscode.Disposable {
  private view?: vscode.WebviewView;
  private ready = false;
  private readonly stateSubscription;

  public constructor(
    private readonly extensionUri: vscode.Uri,
    private readonly state: StateCache,
  ) {
    this.stateSubscription = this.state.onDidChange(() => {
      void this.postState();
    });
  }

  public resolveWebviewView(webviewView: vscode.WebviewView): void {
    this.view = webviewView;
    this.ready = false;
    webviewView.webview.onDidReceiveMessage((message: WebviewToHostMessage) => {
      if (message.type === 'ready') {
        this.ready = true;
        void this.postState();
      }
    });

    configureWebview(webviewView.webview, this.extensionUri, {
      bundleName: 'synapse-grid',
      surface: 'synapse-sidebar',
      title: 'Nexode Synapse Sidebar',
    });

    void this.postState();
  }

  public dispose(): void {
    this.stateSubscription.dispose();
    this.view = undefined;
    this.ready = false;
  }

  private async postState(): Promise<void> {
    if (!this.view || !this.ready) {
      return;
    }

    await this.view.webview.postMessage(createStateMessage(this.state, 'synapse-sidebar'));
  }
}
