/// <reference types="node" />

import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

import { afterEach, expect, test, vi } from 'vite-plus/test';

vi.mock('node:child_process', async (importOriginal) => {
  const actual = await importOriginal<typeof import('node:child_process')>();
  return { ...actual, spawnSync: vi.fn() };
});

const SCRIPT_PATH = resolve(import.meta.dirname, '../cli-help-diff.ts');
const tempDirs: string[] = [];

afterEach(() => {
  vi.mocked(spawnSync).mockReset();
  vi.restoreAllMocks();
  vi.resetModules();
  for (const dir of tempDirs.splice(0)) {
    rmSync(dir, { force: true, recursive: true });
  }
});

test.each([
  { useMetadata: false, failCapture: false },
  { useMetadata: true, failCapture: false },
  { useMetadata: true, failCapture: true },
])(
  'isolates upstream help capture and cleans up (metadata: $useMetadata, failure: $failCapture)',
  async ({ useMetadata, failCapture }) => {
    const root = mkdtempSync(join(tmpdir(), 'vite-plus-cli-help-test-'));
    tempDirs.push(root);
    mkdirSync(join(root, 'vite/packages/vite'), { recursive: true });
    writeFileSync(join(root, 'vite/packages/vite/package.json'), '{"version":"1.0.0"}');
    writeFileSync(
      join(root, 'pnpm-workspace.yaml'),
      'catalog:\n  vitest: 1.0.0\n  oxlint: =1.0.0\n  oxfmt: =1.0.0\n  tsdown: ^1.0.0\noverrides:\n  vite: workspace:@voidzero-dev/vite-plus-core@*\n',
    );
    const toolNames = ['vite', 'vitest', 'oxlint', 'oxfmt', 'tsdown'];
    writeFileSync(
      join(root, 'versions.json'),
      JSON.stringify(
        Object.fromEntries(toolNames.map((name) => [name, { new: '2.0.0', tag: 'v2.0.0' }])),
      ),
    );
    vi.spyOn(process, 'cwd').mockReturnValue(root);
    vi.spyOn(console, 'log').mockImplementation(() => {});
    const captureDirs = new Set<string>();
    const version = useMetadata ? '2.0.0' : '1.0.0';
    vi.mocked(spawnSync).mockImplementation((command, args, options) => {
      expect(command).toBe('pnpm');
      const cwd = String(options?.cwd ?? root);
      captureDirs.add(cwd);
      expect(existsSync(cwd)).toBe(true);
      expect(cwd.startsWith(root)).toBe(false);
      expect(existsSync(join(cwd, 'pnpm-workspace.yaml'))).toBe(false);
      expect(args?.slice(0, 2)).toEqual(['--silent', 'dlx']);
      expect(args?.[2]).toMatch(new RegExp(`@${version.replaceAll('.', '\\.')}$`));
      const stdout = `upstream help ${args?.slice(3).join(' ')}\n`;
      const stderr = failCapture ? 'upstream capture failed' : '';
      return {
        pid: 1,
        output: [null, stdout, stderr],
        stdout,
        stderr,
        status: failCapture ? 1 : 0,
        signal: null,
      };
    });
    const originalArgv = process.argv;
    process.argv = [
      process.execPath,
      SCRIPT_PATH,
      'capture',
      '--output',
      join(root, 'snapshot.json'),
    ];
    if (useMetadata) {
      process.argv.push('--versions', join(root, 'versions.json'));
    }
    try {
      const capture = import('../cli-help-diff.ts');
      if (failCapture) {
        await expect(capture).rejects.toThrow(
          'Failed to capture Vite help (--help):\nupstream capture failed',
        );
        expect(existsSync(join(root, 'snapshot.json'))).toBe(false);
      } else {
        await capture;
        const snapshot = JSON.parse(readFileSync(join(root, 'snapshot.json'), 'utf8'));
        expect(Object.keys(snapshot.tools)).toEqual(toolNames);
        for (const name of toolNames) {
          expect(snapshot.tools[name]).toEqual({
            version,
            help:
              name === 'vite'
                ? '$ vite --help\nupstream help --help\n\n$ vite build --help\nupstream help build --help\n\n$ vite preview --help\nupstream help preview --help'
                : `$ ${name} --help\nupstream help --help`,
          });
        }
        expect(spawnSync).toHaveBeenCalledTimes(7);
      }
    } finally {
      process.argv = originalArgv;
    }
    expect(captureDirs.size).toBeGreaterThan(0);
    for (const dir of captureDirs) {
      expect(existsSync(dir)).toBe(false);
    }
  },
);

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
