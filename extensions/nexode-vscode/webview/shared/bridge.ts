import type { HostToWebviewMessage, SurfaceKind, WebviewToHostMessage } from './types';

interface VsCodeApi {
  postMessage(message: unknown): void;
  getState(): unknown;
  setState(state: unknown): void;
}

declare global {
  interface Window {
    acquireVsCodeApi?: () => VsCodeApi;
  }
}

const vscode = typeof window.acquireVsCodeApi === 'function' ? window.acquireVsCodeApi() : undefined;

export function postHostMessage(message: WebviewToHostMessage): void {
  vscode?.postMessage(message);
}

export function postReady(surface: SurfaceKind): void {
  postHostMessage({
    type: 'ready',
    surface,
  });
}

export function onHostMessage(listener: (message: HostToWebviewMessage) => void): () => void {
  const handleMessage = (event: MessageEvent<HostToWebviewMessage>) => {
    listener(event.data);
  };

  window.addEventListener('message', handleMessage);
  return () => {
    window.removeEventListener('message', handleMessage);
  };
}
