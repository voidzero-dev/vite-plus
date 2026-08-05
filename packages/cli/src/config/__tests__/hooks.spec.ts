import { execSync } from 'node:child_process';
import {
  chmodSync,
  existsSync,
  linkSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

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

  it('skips install when VP_GIT_HOOKS=0', () => {
    const prev = process.env.VP_GIT_HOOKS;
    process.env.VP_GIT_HOOKS = '0';
    try {
      const result = install('.vite-hooks');
      expect(result).toEqual({ message: 'skip install (git hooks disabled)', isError: false });
    } finally {
      if (prev === undefined) {
        delete process.env.VP_GIT_HOOKS;
      } else {
        process.env.VP_GIT_HOOKS = prev;
      }
    }
  });

  it('skips install when deprecated VITE_GIT_HOOKS=0', () => {
    const prev = process.env.VITE_GIT_HOOKS;
    process.env.VITE_GIT_HOOKS = '0';
    try {
      const result = install('.vite-hooks');
      expect(result).toEqual({ message: 'skip install (git hooks disabled)', isError: false });
    } finally {
      if (prev === undefined) {
        delete process.env.VITE_GIT_HOOKS;
      } else {
        process.env.VITE_GIT_HOOKS = prev;
      }
    }
  });

  it('rejects an absolute hooks directory', () => {
    expect(install(resolve(tmpdir(), 'external-hooks'))).toEqual({
      message: 'absolute hooks directory not allowed',
      isError: false,
    });
  });

  it.each(['', '.', './'])('rejects the project root as hooks directory: %j', (hooksDir) => {
    expect(install(hooksDir)).toEqual({
      message: 'hooks directory must be a project subdirectory',
      isError: false,
    });
  });

  it.skipIf(process.platform === 'win32')('does not replace an existing Husky hooks path', () => {
    const tmp = mkdtempSync(join(tmpdir(), 'hooks-husky-path-test-'));
    const originalCwd = process.cwd();
    try {
      execSync('git init', { cwd: tmp, stdio: 'ignore' });
      execSync('git config core.hooksPath .husky/_', { cwd: tmp });
      mkdirSync(join(tmp, '.husky'));
      writeFileSync(join(tmp, '.husky', 'pre-commit'), 'npm test\n');
      process.chdir(tmp);

      expect(install()).toEqual({
        message: 'core.hooksPath is already set to ".husky/_", skipping',
        isError: false,
      });
      expect(execSync('git config --local core.hooksPath', { cwd: tmp }).toString().trim()).toBe(
        '.husky/_',
      );
      expect(readFileSync(join(tmp, '.husky', 'pre-commit'), 'utf8')).toBe('npm test\n');
      expect(existsSync(join(tmp, '.vite-hooks'))).toBe(false);
    } finally {
      process.chdir(originalCwd);
      rmSync(tmp, { recursive: true, force: true });
    }
  });

  it('refreshes an equivalent hooks path spelling', () => {
    const tmp = mkdtempSync(join(tmpdir(), 'hooks-equivalent-path-test-'));
    const originalCwd = process.cwd();
    try {
      execSync('git init', { cwd: tmp, stdio: 'ignore' });
      execSync('git config core.hooksPath .custom-hooks/_', { cwd: tmp });
      process.chdir(tmp);

      expect(install('./.custom-hooks')).toEqual({ message: '', isError: false });
      expect(existsSync(join(tmp, '.custom-hooks', '_', 'pre-commit'))).toBe(true);
    } finally {
      process.chdir(originalCwd);
      rmSync(tmp, { recursive: true, force: true });
    }
  });

  it.skipIf(process.platform === 'win32')(
    'does not claim success over a worktree-scoped hooks path',
    () => {
      const tmp = mkdtempSync(join(tmpdir(), 'hooks-worktree-path-test-'));
      const originalCwd = process.cwd();
      try {
        execSync('git init', { cwd: tmp, stdio: 'ignore' });
        execSync('git config extensions.worktreeConfig true', { cwd: tmp });
        execSync('git config --worktree core.hooksPath .worktree-hooks', { cwd: tmp });
        process.chdir(tmp);

        expect(install()).toEqual({
          message: 'core.hooksPath is already set to ".worktree-hooks", skipping',
          isError: false,
        });
        expect(existsSync(join(tmp, '.vite-hooks'))).toBe(false);
        expect(execSync('git config --get core.hooksPath', { cwd: tmp }).toString().trim()).toBe(
          '.worktree-hooks',
        );
      } finally {
        process.chdir(originalCwd);
        rmSync(tmp, { recursive: true, force: true });
      }
    },
  );

  it.skipIf(process.platform === 'win32')(
    'does not write through a symbolic dispatcher file',
    () => {
      const tmp = mkdtempSync(join(tmpdir(), 'hooks-symlink-test-'));
      const originalCwd = process.cwd();
      try {
        execSync('git init', { cwd: tmp, stdio: 'ignore' });
        const externalFile = join(tmp, 'external-hook-runner');
        mkdirSync(join(tmp, '.vite-hooks', '_'), { recursive: true });
        writeFileSync(externalFile, 'keep me\n');
        symlinkSync(externalFile, join(tmp, '.vite-hooks', '_', 'h'));
        process.chdir(tmp);

        expect(install()).toEqual({
          message: 'symbolic hook path ".vite-hooks/_/h" not allowed',
          isError: false,
        });
        expect(readFileSync(externalFile, 'utf8')).toBe('keep me\n');
        expect(() => execSync('git config --local --get core.hooksPath', { cwd: tmp })).toThrow();
      } finally {
        process.chdir(originalCwd);
        rmSync(tmp, { recursive: true, force: true });
      }
    },
  );

  it.skipIf(process.platform === 'win32')(
    'does not write through a hard-linked dispatcher file',
    () => {
      const tmp = mkdtempSync(join(tmpdir(), 'hooks-hardlink-test-'));
      const originalCwd = process.cwd();
      try {
        execSync('git init', { cwd: tmp, stdio: 'ignore' });
        const externalFile = join(tmp, 'external-hook-runner');
        mkdirSync(join(tmp, '.vite-hooks', '_'), { recursive: true });
        writeFileSync(externalFile, 'keep me\n');
        linkSync(externalFile, join(tmp, '.vite-hooks', '_', 'h'));
        process.chdir(tmp);

        expect(install()).toEqual({
          message: 'multiply linked hook path ".vite-hooks/_/h" not allowed',
          isError: false,
        });
        expect(readFileSync(externalFile, 'utf8')).toBe('keep me\n');
        expect(() => execSync('git config --local --get core.hooksPath', { cwd: tmp })).toThrow();
      } finally {
        process.chdir(originalCwd);
        rmSync(tmp, { recursive: true, force: true });
      }
    },
  );

  it.skipIf(process.platform === 'win32')('restores executable dispatcher permissions', () => {
    const tmp = mkdtempSync(join(tmpdir(), 'hooks-mode-test-'));
    const originalCwd = process.cwd();
    try {
      execSync('git init', { cwd: tmp, stdio: 'ignore' });
      const internalDir = join(tmp, '.vite-hooks', '_');
      mkdirSync(internalDir, { recursive: true });
      writeFileSync(join(internalDir, 'h'), 'stale\n');
      writeFileSync(join(internalDir, 'pre-commit'), 'stale\n');
      chmodSync(join(internalDir, 'h'), 0o600);
      chmodSync(join(internalDir, 'pre-commit'), 0o600);
      process.chdir(tmp);

      expect(install()).toEqual({ message: '', isError: false });
      expect(statSync(join(internalDir, 'h')).mode & 0o777).toBe(0o755);
      expect(statSync(join(internalDir, 'pre-commit')).mode & 0o777).toBe(0o755);
    } finally {
      process.chdir(originalCwd);
      rmSync(tmp, { recursive: true, force: true });
    }
  });
});

describe('hookScript env gates', () => {
  it('honors VP_GIT_HOOKS and keeps VITE_GIT_HOOKS as a deprecated alias', () => {
    const script = hookScript('.vite-hooks');
    expect(script).toContain('"$VP_GIT_HOOKS" = "2"');
    expect(script).toContain('"$VITE_GIT_HOOKS" = "2"');
    expect(script).toContain('"${VP_GIT_HOOKS-}" = "0"');
    expect(script).toContain('"${VITE_GIT_HOOKS-}" = "0"');
  });
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

  it.skipIf(process.platform !== 'win32')('should handle Windows separators in nested dirs', () => {
    const withWindowsSeparators = hookScript('.config\\husky');
    const withPosixSeparators = hookScript('.config/husky');
    expect(countDirnameCalls(withWindowsSeparators)).toBe(countDirnameCalls(withPosixSeparators));
    expect(countDirnameCalls(withWindowsSeparators)).toBe(4);
  });

  it.skipIf(process.platform === 'win32')('treats a backslash as a literal POSIX filename', () => {
    expect(countDirnameCalls(hookScript('.config\\husky'))).toBe(3);
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
