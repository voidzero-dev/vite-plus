import { execSync } from 'node:child_process';
import { existsSync, mkdtempSync, rmSync } from 'node:fs';
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
});
