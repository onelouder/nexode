import * as vscode from 'vscode';

import type { WebviewToHostMessage } from '../webview/shared/types';
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
    configureWebview(panel.webview, this.extensionUri, {
      bundleName: 'synapse-grid',
      surface: 'synapse-grid',
      title: 'Nexode Synapse Grid',
    });

    panel.onDidDispose(() => {
      this.panel = undefined;
      this.ready = false;
    });

    panel.webview.onDidReceiveMessage((message: WebviewToHostMessage) => {
      if (message.type === 'ready') {
        this.ready = true;
        void this.postState();
      }
    });
  }

  public dispose(): void {
    this.stateSubscription.dispose();
    this.panel?.dispose();
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
    configureWebview(webviewView.webview, this.extensionUri, {
      bundleName: 'synapse-grid',
      surface: 'synapse-sidebar',
      title: 'Nexode Synapse Sidebar',
    });

    webviewView.webview.onDidReceiveMessage((message: WebviewToHostMessage) => {
      if (message.type === 'ready') {
        this.ready = true;
        void this.postState();
      }
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
