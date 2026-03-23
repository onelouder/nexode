import * as vscode from 'vscode';

import { parseDiagnostics } from './diagnostic-parser';
import type { StateCache } from './state';

export { parseDiagnostics } from './diagnostic-parser';

export class DiagnosticManager implements vscode.Disposable {
  private readonly collection: vscode.DiagnosticCollection;
  private readonly stateSubscription: vscode.Disposable;

  constructor(private readonly state: StateCache) {
    this.collection = vscode.languages.createDiagnosticCollection('nexode');
    this.stateSubscription = this.state.onDidChange(() => this.refresh());
  }

  private refresh(): void {
    // Clear all, then repopulate from current verification results
    this.collection.clear();

    const results = this.state.getVerificationResults();
    for (const [, result] of results) {
      if (result.success) {
        continue;
      }

      // Find the worktree path for this slot
      const worktreePath = this.findWorktreePath(result.slotId);
      if (!worktreePath) {
        continue;
      }

      const diagnostics = parseDiagnostics(result.stdout, result.stderr);

      // Group by file
      const byFile = new Map<string, vscode.Diagnostic[]>();
      for (const d of diagnostics) {
        const fullPath = d.filePath.startsWith('/') ? d.filePath : `${worktreePath}/${d.filePath}`;
        const uri = vscode.Uri.file(fullPath).toString();
        if (!byFile.has(uri)) {
          byFile.set(uri, []);
        }

        const severity =
          d.severity === 'error'
            ? vscode.DiagnosticSeverity.Error
            : d.severity === 'warning'
              ? vscode.DiagnosticSeverity.Warning
              : vscode.DiagnosticSeverity.Information;

        const range = new vscode.Range(
          Math.max(0, d.line - 1),
          Math.max(0, d.column - 1),
          Math.max(0, d.line - 1),
          200,
        );
        const diag = new vscode.Diagnostic(range, d.message, severity);
        diag.source = `nexode (${result.slotId})`;
        byFile.get(uri)!.push(diag);
      }

      for (const [uriStr, diags] of byFile) {
        this.collection.set(vscode.Uri.parse(uriStr), diags);
      }
    }
  }

  private findWorktreePath(slotId: string): string | undefined {
    for (const project of this.state.getProjects()) {
      for (const slot of project.slots) {
        if (slot.id === slotId && slot.worktreePath) {
          return slot.worktreePath;
        }
      }
    }
    return undefined;
  }

  dispose(): void {
    this.stateSubscription.dispose();
    this.collection.dispose();
  }
}
