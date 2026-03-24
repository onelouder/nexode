export interface ParsedDiagnostic {
  filePath: string;
  line: number;
  column: number;
  severity: 'error' | 'warning' | 'info';
  message: string;
}

export function parseDiagnostics(stdout: string, stderr: string): ParsedDiagnostic[] {
  const results: ParsedDiagnostic[] = [];
  const combined = stdout + '\n' + stderr;

  // Rust/cargo: the error message is on the line ABOVE the --> pointer.
  // Example:
  //   error[E0308]: mismatched types
  //     --> src/main.rs:42:5
  // We also handle warning[...] lines.
  const rustHeaderPattern = /^(error|warning)(\[\w+\])?:\s*(.+)$/gm;
  const rustPointerPattern = /^\s*-->\s+(.+?):(\d+):(\d+)/gm;

  // Build a map of line-number → {severity, message} from error/warning headers
  const lines = combined.split('\n');
  const rustHeaders = new Map<number, { severity: 'error' | 'warning'; message: string }>();
  for (let i = 0; i < lines.length; i++) {
    const headerMatch = /^(error|warning)(\[\w+\])?:\s*(.+)$/.exec(lines[i]);
    if (headerMatch) {
      rustHeaders.set(i, {
        severity: headerMatch[1] as 'error' | 'warning',
        message: headerMatch[3],
      });
    }
  }

  // Match --> pointers and look backwards for the nearest header
  for (let i = 0; i < lines.length; i++) {
    const pointerMatch = /^\s*-->\s+(.+?):(\d+):(\d+)/.exec(lines[i]);
    if (pointerMatch) {
      // Search backwards for the nearest error/warning header (usually 1 line up)
      let header: { severity: 'error' | 'warning'; message: string } | undefined;
      for (let j = i - 1; j >= Math.max(0, i - 5); j--) {
        header = rustHeaders.get(j);
        if (header) break;
      }
      results.push({
        filePath: pointerMatch[1],
        line: parseInt(pointerMatch[2], 10),
        column: parseInt(pointerMatch[3], 10),
        severity: header?.severity ?? 'error',
        message: header?.message ?? 'Build error (see output for details)',
      });
    }
  }

  // TypeScript/tsc: src/app.ts(42,5): error TS2345: message
  const tscPattern = /^(.+?)\((\d+),(\d+)\):\s*(error|warning)\s+\w+:\s*(.+)$/gm;

  // Generic: file:line:col: error: message (gcc, eslint, etc.)
  const genericPattern = /^(.+?):(\d+):(\d+):\s*(error|warning|note|info):\s*(.+)$/gm;

  // Parse tsc pattern
  for (const match of combined.matchAll(tscPattern)) {
    results.push({
      filePath: match[1],
      line: parseInt(match[2], 10),
      column: parseInt(match[3], 10),
      severity: match[4] === 'warning' ? 'warning' : 'error',
      message: match[5],
    });
  }

  for (const match of combined.matchAll(tscPattern)) {
    results.push({
      filePath: match[1],
      line: parseInt(match[2], 10),
      column: parseInt(match[3], 10),
      severity: match[4] === 'warning' ? 'warning' : 'error',
      message: match[5],
    });
  }

  for (const match of combined.matchAll(genericPattern)) {
    results.push({
      filePath: match[1],
      line: parseInt(match[2], 10),
      column: parseInt(match[3], 10),
      severity: match[4] === 'warning' ? 'warning' : match[4] === 'error' ? 'error' : 'info',
      message: match[5],
    });
  }

  // Deduplicate by file:line:col
  const seen = new Set<string>();
  return results.filter((d) => {
    const key = `${d.filePath}:${d.line}:${d.column}`;
    if (seen.has(key)) {
      return false;
    }
    seen.add(key);
    return true;
  });
}
