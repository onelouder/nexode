import * as crypto from 'crypto';
import * as vscode from 'vscode';

import type { HostToWebviewMessage, SurfaceKind, StateEnvelope } from '../webview/shared/types';
import { StateCache } from './state';

interface WebviewDocumentOptions {
  bundleName: 'synapse-grid' | 'kanban';
  surface: SurfaceKind;
  title: string;
}

export function configureWebview(
  webview: vscode.Webview,
  extensionUri: vscode.Uri,
  options: WebviewDocumentOptions,
): void {
  webview.options = {
    enableScripts: true,
    localResourceRoots: [vscode.Uri.joinPath(extensionUri, 'dist', 'webview')],
  };
  webview.html = renderWebviewHtml(webview, extensionUri, options);
}

export function createStateMessage(state: StateCache, surface: SurfaceKind): HostToWebviewMessage {
  const payload: StateEnvelope = {
    surface,
    connection: state.getConnectionStatus(),
    snapshot: state.getSnapshot(),
    agents: state.getAgentStates(),
    alerts: state.getAlerts(),
    metrics: state.getAggregateMetrics(),
    hasSnapshot: state.hasSnapshot(),
  };

  return {
    type: 'state',
    payload,
  };
}

function renderWebviewHtml(
  webview: vscode.Webview,
  extensionUri: vscode.Uri,
  options: WebviewDocumentOptions,
): string {
  const nonce = crypto.randomBytes(16).toString('base64');
  const scriptUri = webview.asWebviewUri(
    vscode.Uri.joinPath(extensionUri, 'dist', 'webview', `${options.bundleName}.js`),
  );
  const styleUri = webview.asWebviewUri(
    vscode.Uri.joinPath(extensionUri, 'dist', 'webview', `${options.bundleName}.css`),
  );
  const csp = [
    "default-src 'none'",
    `img-src ${webview.cspSource} https: data:`,
    `style-src ${webview.cspSource}`,
    `script-src 'nonce-${nonce}'`,
    `font-src ${webview.cspSource}`,
  ].join('; ');

  return `<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <meta http-equiv="Content-Security-Policy" content="${csp}" />
    <title>${escapeHtml(options.title)}</title>
    <link rel="stylesheet" href="${styleUri}" />
  </head>
  <body>
    <div id="root" data-surface="${options.surface}"></div>
    <script nonce="${nonce}" src="${scriptUri}"></script>
  </body>
</html>`;
}

function escapeHtml(value: string): string {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;');
}
