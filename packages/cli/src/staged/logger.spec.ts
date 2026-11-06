import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import type { Logger } from 'lint-staged';
import { describe, expect, it, vi, type Mocked } from 'vite-plus/test';

import { createStagedLogger } from './logger.ts';

function setup(): { output: Mocked<Logger>; logger: Logger } {
  const output = { log: vi.fn(), warn: vi.fn(), error: vi.fn(), debug: vi.fn() };
  return { output, logger: createStagedLogger(output) };
}

describe('staged logger', () => {
  it('prints one result for seven files checked by one command', () => {
    const { output, logger } = setup();
    for (const message of [
      '⋯ Backing up original state…',
      '✔ Done backing up original state (9c3632d)!',
      '⋯ Running tasks for staged files…',
      '    * — 7 files',
      '      ⋯ vp check --fix',
      '',
      '✔ vp check --fix',
      '',
      '✔ Done running tasks for staged files!',
      '⋯ Staging changes from tasks…',
      '✔ Done staging changes from tasks!',
      '⋯ Cleaning up temporary files…',
      '✔ Done cleaning up temporary files!',
    ]) {
      logger.log(message);
    }
    expect(output.log.mock.calls).toEqual([['✔ vp check --fix']]);
  });

  it('filters colored progress and preserves colored command results', () => {
    const { output, logger } = setup();
    logger.log('\x1b[2m⋯ Backing up original state…\x1b[22m');
    logger.log('\x1b[32m✔\x1b[39m Done backing up original state (abc1234)!');
    logger.log('\x1b[2m    *.ts — 7 files\x1b[22m');
    logger.log('      \x1b[34m⋯\x1b[39m vp check --fix');
    const result = '\x1b[32m✔\x1b[39m vp check --fix';
    logger.log(result);
    expect(output.log.mock.calls).toEqual([[result]]);
  });

  it('preserves diagnostics, rollback results, and no-op messages', () => {
    const { output, logger } = setup();
    logger.error('✖ vp check --fix');
    logger.error('\n✖ vp check --fix:\nLint failed');
    logger.warn('↓ skipped command');
    logger.log('✖ Failed to stage changes from tasks!');
    logger.log('✔ Done reverting to original state!');
    logger.log('→ No staged files found.');
    expect(output.error.mock.calls).toEqual([
      ['✖ vp check --fix'],
      ['\n✖ vp check --fix:\nLint failed'],
    ]);
    expect(output.warn).toHaveBeenCalledWith('↓ skipped command');
    expect(output.log.mock.calls).toEqual([
      ['✖ Failed to stage changes from tasks!'],
      ['✔ Done reverting to original state!'],
      ['→ No staged files found.'],
    ]);
  });

  it('preserves verbose task output and messages with format arguments', () => {
    const { output, logger } = setup();
    const taskOutput =
      '\n→ task:\n⋯ progress\n    indented output\n✔ Done cleaning up temporary files!';
    logger.log(taskOutput);
    logger.log('    %s', 'task output');
    expect(output.log.mock.calls).toEqual([[taskOutput], ['    %s', 'task output']]);
  });

  it('keeps the lint-staged recovery message when the patch file exists', () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-staged-patch-'));
    const patchPath = path.join(tmpDir, 'unstaged.patch');
    fs.writeFileSync(patchPath, 'patch content');
    const { output, logger } = setup();

    logger.warn('Unstaged changes have been kept back in a patch file:');
    logger.warn(patchPath);

    expect(output.warn.mock.calls).toEqual([
      ['Unstaged changes have been kept back in a patch file:'],
      [patchPath],
    ]);

    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('replaces the stale recovery message when the patch file is already gone', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const { output, logger } = setup();
    const patchPath = path.join(os.tmpdir(), 'vp-staged-missing', 'unstaged.patch');
    // Ensure the file does not exist (parent directory may or may not exist).
    fs.rmSync(patchPath, { force: true });

    logger.warn('Unstaged changes have been kept back in a patch file:');
    logger.warn(patchPath);

    // The original label/path pair is suppressed; a truthful Vite+ warning is emitted instead.
    expect(output.warn.mock.calls).toEqual([]);
    expect(consoleErrorSpy.mock.calls.length).toBe(1);
    expect(consoleErrorSpy.mock.calls[0][1]).toBe(
      'Unstaged changes were restored from the backup stash. No recovery patch remains.',
    );

    consoleErrorSpy.mockRestore();
  });

  it('flushes a buffered recovery label if the next warning is not a patch path', () => {
    const { output, logger } = setup();
    logger.warn('Unstaged changes have been kept back in a patch file:');
    logger.warn('Some unrelated warning');

    expect(output.warn.mock.calls).toEqual([
      ['Unstaged changes have been kept back in a patch file:'],
      ['Some unrelated warning'],
    ]);
  });
});
