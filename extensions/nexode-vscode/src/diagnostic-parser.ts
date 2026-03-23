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

  // Rust/cargo: error[E0308]: ... --> src/main.rs:42:5
  // Also matches: --> file:line:col
  const rustPattern = /^\s*-->\s+(.+?):(\d+):(\d+)/gm;

  // TypeScript/tsc: src/app.ts(42,5): error TS2345: message
  const tscPattern = /^(.+?)\((\d+),(\d+)\):\s*(error|warning)\s+\w+:\s*(.+)$/gm;

  // Generic: file:line:col: error: message (gcc, eslint, etc.)
  const genericPattern = /^(.+?):(\d+):(\d+):\s*(error|warning|note|info):\s*(.+)$/gm;

  // Parse each pattern
  for (const match of combined.matchAll(rustPattern)) {
    results.push({
      filePath: match[1],
      line: parseInt(match[2], 10),
      column: parseInt(match[3], 10),
      severity: 'error',
      message: 'Build error (see output for details)',
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
