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
});
