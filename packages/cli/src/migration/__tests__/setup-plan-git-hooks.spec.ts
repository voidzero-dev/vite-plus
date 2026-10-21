import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { HOOK_STUB_SCRIPT, hookScript } from '../../config/hooks.ts';
import { PackageManager } from '../../types/index.ts';
import { log } from '../../utils/terminal.ts';
import { installGitHooks, isViteHooksSetupComplete } from '../migrator.ts';
import { collectMigrationSetupPlan, type MigrationSetupPlan } from '../setup-plan.ts';

vi.mock('@voidzero-dev/vite-plus-prompts', async (importOriginal) => {
  const mod = await importOriginal<typeof import('@voidzero-dev/vite-plus-prompts')>();
  return {
    ...mod,
    select: vi.fn(),
    confirm: vi.fn(),
    log: { ...mod.log, info: vi.fn(), warn: vi.fn() },
  };
});

vi.mock('../../utils/terminal.ts', async (importOriginal) => {
  const mod = await importOriginal<typeof import('../../utils/terminal.ts')>();
  return { ...mod, log: vi.fn() };
});

const mockSelect = vi.mocked(prompts.select);
const mockConfirm = vi.mocked(prompts.confirm);
const mockLog = vi.mocked(log);

const skipMessage = expect.stringContaining('Git hooks already configured, skipping');

function git(cwd: string, ...args: string[]): void {
  const result = spawnSync('git', args, { cwd, stdio: 'pipe' });
  if (result.status !== 0) {
    throw new Error(`git ${args.join(' ')} failed: ${result.stderr.toString()}`);
  }
}

function writePackageJson(dir: string, pkg: Record<string, unknown>): void {
  fs.writeFileSync(path.join(dir, 'package.json'), JSON.stringify(pkg));
}

function readPackageJson(dir: string): { scripts?: Record<string, string> } {
  return JSON.parse(fs.readFileSync(path.join(dir, 'package.json'), 'utf8'));
}

/** Simulate what `vp config` leaves behind: dispatcher files and core.hooksPath. */
function installDispatcher(dir: string, hookNames: string[]): void {
  const dispatcherDir = path.join(dir, '.vite-hooks', '_');
  fs.mkdirSync(dispatcherDir, { recursive: true });
  function writeScript(name: string, content: string): void {
    const filePath = path.join(dispatcherDir, name);
    fs.writeFileSync(filePath, content, { mode: 0o755 });
    fs.chmodSync(filePath, 0o755);
  }
  writeScript('h', hookScript('.vite-hooks'));
  for (const hookName of hookNames) {
    writeScript(hookName, HOOK_STUB_SCRIPT);
  }
  git(dir, 'config', 'core.hooksPath', '.vite-hooks/_');
}

function expectNoHooksPrompt(): void {
  expect(mockConfirm).not.toHaveBeenCalled();
  expect(mockSelect).not.toHaveBeenCalled();
}

describe('collectMigrationSetupPlan existing Vite+ hooks', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-setup-plan-hooks-'));
    writePackageJson(tmpDir, {});
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
    vi.clearAllMocks();
  });

  function writeProjectHooks(): void {
    fs.mkdirSync(path.join(tmpDir, '.vite-hooks'), { recursive: true });
    fs.writeFileSync(path.join(tmpDir, '.vite-hooks', 'commit-msg'), 'echo msg\n');
    fs.writeFileSync(path.join(tmpDir, '.vite-hooks', 'pre-commit'), 'npm test\n');
  }

  function setUpFullyConfiguredHooks(): void {
    git(tmpDir, 'init', '-q');
    writePackageJson(tmpDir, { scripts: { prepare: 'vp config' } });
    writeProjectHooks();
    installDispatcher(tmpDir, ['commit-msg', 'pre-commit']);
  }

  function collect(options: {
    interactive: boolean;
    hooks?: boolean;
  }): Promise<MigrationSetupPlan> {
    return collectMigrationSetupPlan(
      tmpDir,
      PackageManager.pnpm,
      { ...options, agent: false, editor: false },
      [],
      false,
    );
  }

  describe('fully configured hooks', () => {
    beforeEach(setUpFullyConfiguredHooks);

    it('is detected as complete', () => {
      expect(isViteHooksSetupComplete(tmpDir)).toBe(true);
    });

    it.each([true, false])(
      'skips the hooks step without prompting (interactive=%s)',
      async (interactive) => {
        const plan = await collect({ interactive });

        expect(plan.shouldSetupHooks).toBe(false);
        expectNoHooksPrompt();
        expect(mockLog).toHaveBeenCalledWith(skipMessage);
      },
    );

    it('accepts a postinstall script that runs vp config', async () => {
      writePackageJson(tmpDir, { scripts: { postinstall: 'vp config' } });

      expect(isViteHooksSetupComplete(tmpDir)).toBe(true);
      const plan = await collect({ interactive: true });
      expect(plan.shouldSetupHooks).toBe(false);
      expectNoHooksPrompt();
      expect(mockLog).toHaveBeenCalledWith(skipMessage);
    });

    it.each([
      'tsc -b && vp config',
      'vp config --no-hooks --hooks',
      'pnpm exec vp config',
      'vp "config" --hooks-dir="./.vite-hooks"',
      'echo "build; done" && vp config',
    ])('accepts a lifecycle script running %s', (prepare) => {
      writePackageJson(tmpDir, { scripts: { prepare } });

      expect(isViteHooksSetupComplete(tmpDir)).toBe(true);
    });

    it.each([
      'echo vp config',
      'node setup.js vp config',
      'echo "build && vp config && done"',
      'vp config "--no-hooks"',
      'vp config --help',
      'vp config --hooks-dir .custom-hooks',
      'VP_GIT_HOOKS=0 vp config',
      'cd packages/app && vp config',
    ])('does not count %j as a hook refresh', async (prepare) => {
      writePackageJson(tmpDir, { scripts: { prepare } });

      expect(isViteHooksSetupComplete(tmpDir)).toBe(false);
      expect((await collect({ interactive: true })).shouldSetupHooks).toBe(true);
      expectNoHooksPrompt();
    });

    it.each([true, false])('honors --hooks=%s when setup is complete', async (hooks) => {
      const plan = await collect({ interactive: true, hooks });

      expect(plan.shouldSetupHooks).toBe(hooks);
      expectNoHooksPrompt();
      expect(mockLog).not.toHaveBeenCalledWith(skipMessage);
    });
  });

  describe('partially configured hooks', () => {
    it.each([
      [
        'no prepare script',
        () => {
          git(tmpDir, 'init', '-q');
          writeProjectHooks();
          installDispatcher(tmpDir, ['commit-msg', 'pre-commit']);
        },
      ],
      [
        'core.hooksPath unset',
        () => {
          git(tmpDir, 'init', '-q');
          writePackageJson(tmpDir, { scripts: { prepare: 'vp config' } });
          writeProjectHooks();
        },
      ],
      [
        'dispatcher missing',
        () => {
          setUpFullyConfiguredHooks();
          fs.rmSync(path.join(tmpDir, '.vite-hooks', '_'), { recursive: true });
        },
      ],
      [
        'dispatcher stub missing for a project hook',
        () => {
          setUpFullyConfiguredHooks();
          fs.rmSync(path.join(tmpDir, '.vite-hooks', '_', 'commit-msg'));
        },
      ],
      [
        'the dispatcher is not executable',
        () => {
          setUpFullyConfiguredHooks();
          fs.chmodSync(path.join(tmpDir, '.vite-hooks', '_', 'h'), 0o644);
        },
      ],
      [
        'a dispatcher stub is not executable',
        () => {
          setUpFullyConfiguredHooks();
          fs.chmodSync(path.join(tmpDir, '.vite-hooks', '_', 'pre-commit'), 0o644);
        },
      ],
      [
        'the dispatcher content is stale',
        () => {
          setUpFullyConfiguredHooks();
          fs.writeFileSync(path.join(tmpDir, '.vite-hooks', '_', 'h'), '#!/usr/bin/env sh\n');
        },
      ],
      [
        'a dispatcher stub content is stale',
        () => {
          setUpFullyConfiguredHooks();
          fs.writeFileSync(path.join(tmpDir, '.vite-hooks', '_', 'commit-msg'), 'exit 0\n');
        },
      ],
      [
        'prepare runs vp config --no-hooks',
        () => {
          setUpFullyConfiguredHooks();
          writePackageJson(tmpDir, { scripts: { prepare: 'vp config --no-hooks' } });
        },
      ],
      [
        'postinstall runs vp config --hooks --no-hooks',
        () => {
          setUpFullyConfiguredHooks();
          writePackageJson(tmpDir, {
            scripts: { postinstall: 'tsc -b && vp config --hooks --no-hooks' },
          });
        },
      ],
      [
        'no git repository',
        () => {
          writePackageJson(tmpDir, { scripts: { prepare: 'vp config' } });
          writeProjectHooks();
        },
      ],
    ])('completes setup without prompting when %s', async (testCase, setUp) => {
      if (process.platform === 'win32' && testCase.includes('not executable')) {
        // Windows has no POSIX execute bits, so these cases do not apply.
        return;
      }
      setUp();
      expect(isViteHooksSetupComplete(tmpDir)).toBe(false);

      for (const interactive of [true, false]) {
        const plan = await collect({ interactive });
        expect(plan.shouldSetupHooks).toBe(true);
      }
      expectNoHooksPrompt();
      expect(mockLog).not.toHaveBeenCalledWith(skipMessage);
    });

    it('does not treat hook files alone as fully configured', () => {
      writeProjectHooks();

      expect(isViteHooksSetupComplete(tmpDir)).toBe(false);
    });

    it('preserves existing hook files while completing setup', async () => {
      writeProjectHooks();

      const plan = await collect({ interactive: true });
      expect(plan.shouldSetupHooks).toBe(true);
      expect(installGitHooks(tmpDir, true)).toBe(true);

      expect(fs.readFileSync(path.join(tmpDir, '.vite-hooks', 'pre-commit'), 'utf8')).toBe(
        'npm test\n',
      );
      expect(fs.readFileSync(path.join(tmpDir, '.vite-hooks', 'commit-msg'), 'utf8')).toBe(
        'echo msg\n',
      );
      expect(readPackageJson(tmpDir).scripts?.prepare).toBe('vp config');
      expect(fs.existsSync(path.join(tmpDir, 'vite.config.ts'))).toBe(false);
    });

    it('does not add a redundant prepare script when postinstall runs vp config', async () => {
      writePackageJson(tmpDir, { scripts: { postinstall: 'vp config', prepare: 'tsc -b' } });
      writeProjectHooks();

      const plan = await collect({ interactive: true });
      expect(plan.shouldSetupHooks).toBe(true);
      expect(installGitHooks(tmpDir, true)).toBe(true);

      expect(readPackageJson(tmpDir).scripts).toEqual({
        postinstall: 'vp config',
        prepare: 'tsc -b',
      });
    });

    it('adds a refreshing prepare script when postinstall runs vp config --no-hooks', () => {
      writePackageJson(tmpDir, { scripts: { postinstall: 'vp config --no-hooks' } });
      writeProjectHooks();

      expect(installGitHooks(tmpDir, true)).toBe(true);

      expect(readPackageJson(tmpDir).scripts).toEqual({
        postinstall: 'vp config --no-hooks',
        prepare: 'vp config',
      });
    });

    it.each([
      ['vp config --no-hooks', 'vp config'],
      ['tsc -b && vp config --no-hooks', 'tsc -b && vp config'],
      [
        'echo "你好🦀" && pnpm exec vp config "--no-hooks" && echo "café"',
        'echo "你好🦀" && pnpm exec vp config && echo "café"',
      ],
      [
        'echo "build && --no-hooks" && vp "config" "--no-hooks" && echo --no-hooks',
        'echo "build && --no-hooks" && vp "config" && echo --no-hooks',
      ],
      [
        'vp config --no-hooks --hooks-dir .vite-hooks --no-agent && tsc -b',
        'vp config --hooks-dir .vite-hooks --no-agent && tsc -b',
      ],
    ])('rewrites prepare %j in place to re-enable hooks', (prepare, expected) => {
      writePackageJson(tmpDir, { scripts: { prepare, postinstall: 'node setup.js' } });
      writeProjectHooks();

      expect(installGitHooks(tmpDir, true)).toBe(true);

      expect(readPackageJson(tmpDir).scripts).toEqual({
        prepare: expected,
        postinstall: 'node setup.js',
      });
    });

    it('prepends vp config to an existing prepare script', () => {
      writePackageJson(tmpDir, { scripts: { prepare: 'tsc -b' } });
      writeProjectHooks();

      expect(installGitHooks(tmpDir, true)).toBe(true);

      expect(readPackageJson(tmpDir).scripts?.prepare).toBe('vp config && tsc -b');
    });

    it('still runs the existing preflight checks', async () => {
      git(tmpDir, 'init', '-q');
      writeProjectHooks();
      git(tmpDir, 'config', 'core.hooksPath', '.githooks');

      const plan = await collect({ interactive: true });

      expect(plan.shouldSetupHooks).toBe(false);
      expectNoHooksPrompt();
      expect(prompts.log.warn).toHaveBeenCalledWith(
        expect.stringContaining('core.hooksPath is already set to ".githooks"'),
      );
    });

    it('runs preflight when the dispatcher directory is a regular file', async () => {
      setUpFullyConfiguredHooks();
      const dispatcherDir = path.join(tmpDir, '.vite-hooks', '_');
      fs.rmSync(dispatcherDir, { recursive: true });
      fs.writeFileSync(dispatcherDir, 'preserve this file');

      expect(isViteHooksSetupComplete(tmpDir)).toBe(false);
      const plan = await collect({ interactive: true });

      expect(plan.shouldSetupHooks).toBe(false);
      expectNoHooksPrompt();
      expect(prompts.log.warn).toHaveBeenCalledWith(
        expect.stringContaining(
          `Git hook dispatcher path "${path.join('.vite-hooks', '_')}" is unsafe`,
        ),
      );
      expect(fs.readFileSync(dispatcherDir, 'utf8')).toBe('preserve this file');
    });

    it.each([
      'echo vp config --no-hooks',
      'node setup.js vp config --no-hooks',
      'echo "build && vp config --no-hooks && done"',
      'cd tools && vp config',
      'VP_GIT_HOOKS=0 vp config',
      'echo "$BUILD_STATUS"',
    ])('preserves arguments in %j when adding a refresh command', (prepare) => {
      writePackageJson(tmpDir, { scripts: { prepare } });
      writeProjectHooks();

      expect(installGitHooks(tmpDir, true)).toBe(true);
      expect(readPackageJson(tmpDir).scripts?.prepare).toBe(`vp config && ${prepare}`);
      expect(installGitHooks(tmpDir, true)).toBe(true);
      expect(readPackageJson(tmpDir).scripts?.prepare).toBe(`vp config && ${prepare}`);
    });

    it.each([true, false])('respects --hooks=%s', async (hooks) => {
      writeProjectHooks();

      const plan = await collect({ interactive: true, hooks });

      expect(plan.shouldSetupHooks).toBe(hooks);
      expectNoHooksPrompt();
    });
  });

  describe('no existing hooks', () => {
    it('does not treat non-hook files in .vite-hooks/ as existing hooks', async () => {
      fs.mkdirSync(path.join(tmpDir, '.vite-hooks'));
      fs.writeFileSync(path.join(tmpDir, '.vite-hooks', 'README.md'), 'not a hook\n');
      mockConfirm.mockResolvedValueOnce(true);

      const plan = await collect({ interactive: true });

      expect(plan.shouldSetupHooks).toBe(true);
      expect(mockConfirm).toHaveBeenCalledTimes(1);
    });

    it.each([true, false])('asks the original confirm prompt (answer=%s)', async (answer) => {
      mockConfirm.mockResolvedValueOnce(answer);

      const plan = await collect({ interactive: true });

      expect(plan.shouldSetupHooks).toBe(answer);
      expect(mockConfirm).toHaveBeenCalledTimes(1);
      expect(mockSelect).not.toHaveBeenCalled();
    });

    it('sets up hooks by default in non-interactive mode', async () => {
      const plan = await collect({ interactive: false });

      expect(plan.shouldSetupHooks).toBe(true);
      expectNoHooksPrompt();
    });

    it.each([true, false])('respects --hooks=%s without prompting', async (hooks) => {
      const plan = await collect({ interactive: true, hooks });

      expect(plan.shouldSetupHooks).toBe(hooks);
      expectNoHooksPrompt();
    });
  });
});
