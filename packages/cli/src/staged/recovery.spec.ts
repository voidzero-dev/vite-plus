import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import lintStaged from 'lint-staged';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test';

import { createStagedLogger } from './logger.ts';

const restoredMessage = 'Original state was restored from the backup stash.';
const patchMessage = 'Unstaged changes have been kept back in a patch file:';

function createLogger() {
  return { log: vi.fn(), warn: vi.fn(), error: vi.fn(), debug: vi.fn() };
}

describe('staged recovery', () => {
  let cwd: string;
  const git = (...args: string[]) => execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();

  beforeEach(() => {
    cwd = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-staged-recovery-'));
    git('init', '--quiet');
    git('config', 'user.name', 'Vite Plus');
    git('config', 'user.email', 'vite-plus@example.com');
    git('config', 'commit.gpgsign', 'false');
    git('config', 'core.autocrlf', 'false');
    fs.writeFileSync(path.join(cwd, 'file.txt'), 'a\nb\nc\n');
    fs.writeFileSync(
      path.join(cwd, 'task.cjs'),
      "require('node:fs').writeFileSync('file.txt', 'a\\nc\\n');\n",
    );
    git('add', '.');
    git('-c', 'core.hooksPath=', 'commit', '--quiet', '-m', 'init');
    fs.writeFileSync(path.join(cwd, 'file.txt'), 'a\nstaged\nc\n');
    git('add', 'file.txt');
    fs.writeFileSync(path.join(cwd, 'file.txt'), 'a\nunstaged\nc\n');
  });

  afterEach(() => {
    vi.restoreAllMocks();
    fs.rmSync(cwd, { recursive: true, force: true });
  });

  function expectOriginalState() {
    expect(git('show', ':file.txt')).toBe('a\nstaged\nc');
    expect(fs.readFileSync(path.join(cwd, 'file.txt'), 'utf8')).toBe('a\nunstaged\nc\n');
  }

  it.each([false, true])('reports successful rollback with debug=%s', async (debug) => {
    const logger = createLogger();
    const success = await lintStaged(
      { cwd, config: { '*.txt': 'node task.cjs' }, debug, color: false },
      debug ? logger : createStagedLogger(logger),
    );

    expect(success).toBe(false);
    expect(logger.warn.mock.calls).toEqual([[restoredMessage]]);
    expectOriginalState();
    expect(git('stash', 'list')).toBe('');
  });

  it.each([{ revert: false }, { stash: false }])(
    'keeps the recovery patch with %o',
    async (options) => {
      const logger = createLogger();
      const success = await lintStaged(
        { cwd, config: { '*.txt': 'node task.cjs' }, color: false, ...options },
        createStagedLogger(logger),
      );

      expect(success).toBe(false);
      expect(logger.warn).toHaveBeenCalledWith(patchMessage);
      expect(logger.warn).not.toHaveBeenCalledWith(restoredMessage);
      const patchPath = logger.warn.mock.calls.at(-1)![0] as string;
      expect(fs.readFileSync(patchPath, 'utf8')).toContain('+unstaged');
    },
  );

  it('keeps the recovery patch and backup stash when rollback fails', async () => {
    // Fail only the rollback's stash apply; all other Git operations are real.
    const { GitWorkflow } = await import(
      new URL('./gitWorkflow.js', import.meta.resolve('lint-staged')).href
    );
    const restoreOriginalState = GitWorkflow.prototype.restoreOriginalState;
    vi.spyOn(GitWorkflow.prototype, 'restoreOriginalState').mockImplementation(async function (
      this: { execGit: (args: string[]) => Promise<string> },
      ctx: unknown,
    ) {
      const execGit = this.execGit;
      this.execGit = async (args) => {
        if (args[0] === 'stash' && args[1] === 'apply') {
          throw new Error('Simulated stash apply failure');
        }
        return execGit(args);
      };
      await restoreOriginalState.call(this, ctx);
    });
    const logger = createLogger();
    const success = await lintStaged(
      { cwd, config: { '*.txt': 'node task.cjs' }, color: false },
      createStagedLogger(logger),
    );

    expect(success).toBe(false);
    expect(logger.error).toHaveBeenCalledWith('✖ Failed to revert to original state!');
    expect(logger.warn).toHaveBeenCalledWith(patchMessage);
    expect(logger.warn).not.toHaveBeenCalledWith(restoredMessage);
    const patchPath = logger.warn.mock.calls.at(-1)![0] as string;
    expect(fs.readFileSync(patchPath, 'utf8')).toContain('+unstaged');
    expect(git('stash', 'list')).toContain('lint-staged automatic backup');
    git('stash', 'apply', '--index');
    expectOriginalState();
  });

  it('preserves both diffs without recovery warnings when tasks succeed', async () => {
    const logger = createLogger();
    const success = await lintStaged(
      { cwd, config: { '*.txt': 'node --version' }, color: false },
      createStagedLogger(logger),
    );

    expect(success).toBe(true);
    expect(logger.warn).not.toHaveBeenCalled();
    expectOriginalState();
  });
});
