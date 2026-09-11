/// <reference types="node" />

import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

import { afterEach, expect, test } from 'vitest';

const SCRIPT_PATH = resolve(import.meta.dirname, '../cli-help-diff.ts');
const tempDirs: string[] = [];

afterEach(() => {
  for (const dir of tempDirs.splice(0)) {
    rmSync(dir, { force: true, recursive: true });
  }
});

test('reports changed, unchanged, and not-updated CLI help in one comment', () => {
  const tempDir = mkdtempSync(join(tmpdir(), 'vite-plus-cli-help-test-'));
  tempDirs.push(tempDir);
  const beforePath = join(tempDir, 'before.json');
  const afterPath = join(tempDir, 'after.json');
  const githubOutputPath = join(tempDir, 'github-output.txt');
  const reportPath = join(tempDir, 'report.md');
  const before = {
    tools: {
      vite: { help: 'vite/1.0.0\n--old-option', version: '1.0.0' },
      vitest: { help: 'vitest/1.0.0\n--watch', version: '1.0.0' },
      oxlint: { help: 'oxlint 1.0.0\n--fix', version: '1.0.0' },
      oxfmt: { help: 'oxfmt\n--write', version: '1.0.0' },
      tsdown: { help: 'tsdown 1.0.0\n--old-option', version: '1.0.0' },
    },
  };
  const after = {
    tools: {
      vite: { help: 'vite/2.0.0\n--new-option', version: '2.0.0' },
      vitest: { help: 'vitest/1.0.0\n--watch', version: '1.0.0' },
      oxlint: { help: 'oxlint 2.0.0\n--fix', version: '2.0.0' },
      oxfmt: { help: 'oxfmt\n--write', version: '1.0.0' },
      tsdown: { help: 'tsdown 2.0.0\n--new-option', version: '2.0.0' },
    },
  };
  writeFileSync(beforePath, JSON.stringify(before));
  writeFileSync(afterPath, JSON.stringify(after));

  execFileSync(
    process.execPath,
    [
      SCRIPT_PATH,
      'report',
      '--before',
      beforePath,
      '--after',
      afterPath,
      '--output',
      reportPath,
      '--github-output',
      githubOutputPath,
    ],
    { cwd: resolve(import.meta.dirname, '../../..') },
  );

  const report = readFileSync(reportPath, 'utf8');
  expect(report).toContain('## ⚠️ Upstream CLI help changes detected');
  expect(report).toContain('<strong>⚠️ Vite: CLI help changed (1.0.0 → 2.0.0)</strong>');
  expect(report).toContain('<strong>✅ Oxlint: no CLI help changes (1.0.0 → 2.0.0)</strong>');
  expect(report).toContain('<strong>➖ Vitest: no version update (1.0.0)</strong>');
  expect(report).toContain('```diff\n--- vite@1.0.0\n+++ vite@2.0.0');
  expect(report).toContain('---old-option');
  expect(report).toContain('+--new-option');
  expect(report).not.toContain('-vite/1.0.0');
  expect(report).not.toContain('+vite/2.0.0');
  expect(readFileSync(githubOutputPath, 'utf8')).toBe('has-changes=true\n');
});

test('reports no machine-readable changes when help is unchanged', () => {
  const tempDir = mkdtempSync(join(tmpdir(), 'vite-plus-cli-help-test-'));
  tempDirs.push(tempDir);
  const snapshotPath = join(tempDir, 'snapshot.json');
  const githubOutputPath = join(tempDir, 'github-output.txt');
  const reportPath = join(tempDir, 'report.md');
  const snapshot = {
    tools: Object.fromEntries(
      ['vite', 'vitest', 'oxlint', 'oxfmt', 'tsdown'].map((tool) => [
        tool,
        { help: `${tool}/1.0.0\n--help`, version: '1.0.0' },
      ]),
    ),
  };
  writeFileSync(snapshotPath, JSON.stringify(snapshot));

  execFileSync(
    process.execPath,
    [
      SCRIPT_PATH,
      'report',
      '--before',
      snapshotPath,
      '--after',
      snapshotPath,
      '--output',
      reportPath,
      '--github-output',
      githubOutputPath,
    ],
    { cwd: resolve(import.meta.dirname, '../../..') },
  );

  expect(readFileSync(githubOutputPath, 'utf8')).toBe('has-changes=false\n');
});
