import assert from 'node:assert/strict';
import test from 'node:test';

import { parseDiagnostics } from '../src/diagnostic-parser';

test('parseDiagnostics parses Rust error pattern', () => {
  const stdout = '';
  const stderr = 'error[E0308]: mismatched types\n  --> src/main.rs:42:5\n';
  const results = parseDiagnostics(stdout, stderr);

  assert.equal(results.length, 1);
  assert.equal(results[0].filePath, 'src/main.rs');
  assert.equal(results[0].line, 42);
  assert.equal(results[0].column, 5);
  assert.equal(results[0].severity, 'error');
});

test('parseDiagnostics parses TypeScript error pattern', () => {
  const stdout = 'src/app.ts(10,3): error TS2345: Argument of type string is not assignable';
  const stderr = '';
  const results = parseDiagnostics(stdout, stderr);

  assert.equal(results.length, 1);
  assert.equal(results[0].filePath, 'src/app.ts');
  assert.equal(results[0].line, 10);
  assert.equal(results[0].column, 3);
  assert.equal(results[0].severity, 'error');
  assert.equal(results[0].message, 'Argument of type string is not assignable');
});

test('parseDiagnostics parses generic error pattern', () => {
  const stdout = '';
  const stderr = 'file.c:10:5: error: undeclared identifier';
  const results = parseDiagnostics(stdout, stderr);

  assert.equal(results.length, 1);
  assert.equal(results[0].filePath, 'file.c');
  assert.equal(results[0].line, 10);
  assert.equal(results[0].column, 5);
  assert.equal(results[0].severity, 'error');
  assert.equal(results[0].message, 'undeclared identifier');
});

test('parseDiagnostics returns empty array for empty input', () => {
  const results = parseDiagnostics('', '');
  assert.equal(results.length, 0);
});

test('parseDiagnostics works with stderr-only input', () => {
  const results = parseDiagnostics('', '  --> lib/utils.rs:7:12');
  assert.equal(results.length, 1);
  assert.equal(results[0].filePath, 'lib/utils.rs');
  assert.equal(results[0].line, 7);
  assert.equal(results[0].column, 12);
});

test('parseDiagnostics deduplicates same file:line:col', () => {
  const stderr = [
    '  --> src/main.rs:42:5',
    '  --> src/main.rs:42:5',
  ].join('\n');
  const results = parseDiagnostics('', stderr);
  assert.equal(results.length, 1);
});

test('parseDiagnostics parses warning severity', () => {
  const stdout = 'src/app.ts(20,1): warning TS6133: unused variable';
  const results = parseDiagnostics(stdout, '');

  assert.equal(results.length, 1);
  assert.equal(results[0].severity, 'warning');
});

test('parseDiagnostics parses generic warning and info', () => {
  const stderr = [
    'file.c:1:1: warning: implicit declaration',
    'file.c:2:1: note: declared here',
  ].join('\n');
  const results = parseDiagnostics('', stderr);

  assert.equal(results.length, 2);
  assert.equal(results[0].severity, 'warning');
  assert.equal(results[1].severity, 'info');
});
