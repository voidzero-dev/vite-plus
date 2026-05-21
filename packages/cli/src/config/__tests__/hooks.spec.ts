import { execSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { describe, expect, it } from 'vitest';

import { hookScript, install } from '../hooks.js';

function countDirnameCalls(script: string): number {
  // Count nested dirname calls in the `d=...` line
  const match = script.match(/^d=(.+)$/m);
  if (!match) {
    return 0;
  }
  return (match[1].match(/dirname/g) ?? []).length;
}

describe('install', () => {
  it.skipIf(process.platform === 'win32')(
    'should create _/pre-commit but not pre-commit in hooks dir root',
    () => {
      const tmp = mkdtempSync(join(tmpdir(), 'hooks-test-'));
      const originalCwd = process.cwd();
      try {
        // Set up a temporary git repo
        execSync('git init', { cwd: tmp, stdio: 'ignore' });
        process.chdir(tmp);

        const hooksDir = '.vite-hooks';
        const result = install(hooksDir);
        expect(result.isError).toBe(false);

        // install() creates the internal shim at _/pre-commit
        expect(existsSync(join(tmp, hooksDir, '_', 'pre-commit'))).toBe(true);
        // install() does NOT create pre-commit at the hooks dir root
        expect(existsSync(join(tmp, hooksDir, 'pre-commit'))).toBe(false);
      } finally {
        process.chdir(originalCwd);
        rmSync(tmp, { recursive: true, force: true });
      }
    },
  );
});

describe('hookScript', () => {
  it('should compute correct depth for simple dir', () => {
    // ".vite-hooks" → 1 segment → depth 3
    const script = hookScript('.vite-hooks');
    expect(countDirnameCalls(script)).toBe(3);
  });

  it('should compute correct depth for nested dir', () => {
    // ".config/husky" → 2 segments → depth 4
    const script = hookScript('.config/husky');
    expect(countDirnameCalls(script)).toBe(4);
  });

  it('should handle ./ prefix correctly (bug case)', () => {
    // "./.config/husky" should produce same depth as ".config/husky"
    // Before fix: filter(Boolean) kept "." → 3 segments → depth 5 (wrong)
    // After fix: filter out "." → 2 segments → depth 4 (correct)
    const withDot = hookScript('./.config/husky');
    const withoutDot = hookScript('.config/husky');
    expect(countDirnameCalls(withDot)).toBe(countDirnameCalls(withoutDot));
    expect(countDirnameCalls(withDot)).toBe(4);
  });

  it('should handle ./ prefix for simple dir', () => {
    // "./custom-hooks" should produce same depth as "custom-hooks"
    const withDot = hookScript('./custom-hooks');
    const withoutDot = hookScript('custom-hooks');
    expect(countDirnameCalls(withDot)).toBe(countDirnameCalls(withoutDot));
    expect(countDirnameCalls(withDot)).toBe(3);
  });

  it.skipIf(process.platform === 'win32')(
    'should add Vite+ managed bin to PATH as a fallback before running user hook',
    () => {
      const tmp = mkdtempSync(join(tmpdir(), 'hooks-path-test-'));
      try {
        const hooksDir = join(tmp, '.vite-hooks');
        const internalHooksDir = join(hooksDir, '_');
        const nodeModulesBin = join(tmp, 'node_modules', '.bin');
        const vpHomeBin = join(tmp, 'vp-home', 'bin');
        const systemBin = join(tmp, 'system-bin');

        mkdirSync(internalHooksDir, { recursive: true });
        mkdirSync(nodeModulesBin, { recursive: true });
        mkdirSync(vpHomeBin, { recursive: true });
        mkdirSync(systemBin, { recursive: true });

        writeFileSync(join(internalHooksDir, 'h'), hookScript('.vite-hooks'), { mode: 0o755 });
        writeFileSync(
          join(internalHooksDir, 'pre-commit'),
          '#!/usr/bin/env sh\n. "$(dirname "$0")/h"',
          { mode: 0o755 },
        );
        writeFileSync(join(hooksDir, 'pre-commit'), 'vp staged\n');

        writeFileSync(
          join(nodeModulesBin, 'vp'),
          '#!/bin/sh\nbasedir=$(dirname "$0")\nexec node "$basedir/../vite-plus/bin/vp" "$@"\n',
          { mode: 0o755 },
        );
        writeFileSync(
          join(vpHomeBin, 'node'),
          '#!/bin/sh\necho "fake-node $*" > "$VP_HOME/node-used"\n',
          { mode: 0o755 },
        );
        writeFileSync(
          join(vpHomeBin, 'dirname'),
          '#!/bin/sh\necho "wrong dirname" > "$VP_HOME/dirname-used"\nexit 1\n',
          { mode: 0o755 },
        );
        writeFileSync(
          join(vpHomeBin, 'sh'),
          '#!/bin/sh\necho "wrong sh" > "$VP_HOME/sh-used"\nexit 1\n',
          { mode: 0o755 },
        );

        writeFileSync(join(systemBin, 'sh'), '#!/bin/sh\nexec /bin/sh "$@"\n', {
          mode: 0o755,
        });
        writeFileSync(join(systemBin, 'dirname'), '#!/bin/sh\nexec /usr/bin/dirname "$@"\n', {
          mode: 0o755,
        });
        writeFileSync(join(systemBin, 'basename'), '#!/bin/sh\nexec /usr/bin/basename "$@"\n', {
          mode: 0o755,
        });

        execSync('sh .vite-hooks/_/pre-commit', {
          cwd: tmp,
          env: {
            HOME: join(tmp, 'home'),
            PATH: systemBin,
            VP_HOME: join(tmp, 'vp-home'),
          },
        });

        expect(existsSync(join(tmp, 'vp-home', 'node-used'))).toBe(true);
        expect(existsSync(join(tmp, 'vp-home', 'dirname-used'))).toBe(false);
        expect(existsSync(join(tmp, 'vp-home', 'sh-used'))).toBe(false);
      } finally {
        rmSync(tmp, { recursive: true, force: true });
      }
    },
  );

  it('should compute root and shell before appending Vite+ managed bin', () => {
    const script = hookScript('.vite-hooks');
    expect(script.indexOf('d=')).toBeLessThan(script.indexOf('export PATH="$PATH:$__vp_bin"'));
    expect(script.indexOf('__vp_shell=')).toBeLessThan(
      script.indexOf('export PATH="$PATH:$__vp_bin"'),
    );
    expect(script).toContain('"$__vp_shell" -e "$s" "$@"');
  });
});
