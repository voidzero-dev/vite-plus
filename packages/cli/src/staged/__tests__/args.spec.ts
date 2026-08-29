import { describe, expect, it } from 'vitest';

import { normalizeStagedArgs, parseStagedArgs } from '../args.js';

describe('staged arguments', () => {
  it.each([
    { argv: ['--no-concurrent'], expected: false },
    { argv: ['--concurrent', 'false'], expected: false },
    { argv: ['--concurrent', 'true'], expected: true },
    { argv: ['--concurrent'], expected: true },
    { argv: ['--concurrent=1'], expected: 1 },
    { argv: ['-p', '2'], expected: 2 },
  ])('normalizes $argv to concurrent=$expected', ({ argv, expected }) => {
    expect(normalizeStagedArgs(parseStagedArgs(argv)).concurrent).toBe(expected);
  });

  it.each([
    { argv: ['--concurrent=0'] },
    { argv: ['-p', '0'] },
    { argv: ['--concurrent=-1'] },
    { argv: ['--concurrent=NaN'] },
  ])('rejects invalid concurrency $argv', ({ argv }) => {
    expect(() => normalizeStagedArgs(parseStagedArgs(argv))).toThrow(
      'Option "--concurrent" must be true, false, or a number greater than 0.',
    );
  });

  it.each([
    { argv: ['--no-cwd'], option: 'cwd', value: 'path' },
    { argv: ['--no-diff'], option: 'diff', value: 'string' },
    { argv: ['--no-diff-filter'], option: 'diff-filter', value: 'string' },
  ])('rejects the negated string option --no-$option', ({ argv, option, value }) => {
    expect(() => normalizeStagedArgs(parseStagedArgs(argv))).toThrow(
      `Option "--no-${option}" is not supported. Use "--${option} <${value}>".`,
    );
  });

  it('preserves valid string options', () => {
    expect(
      normalizeStagedArgs(
        parseStagedArgs(['--cwd', 'packages/app', '--diff=main...HEAD', '--diff-filter', 'ACMR']),
      ),
    ).toEqual({
      concurrent: undefined,
      cwd: 'packages/app',
      diff: 'main...HEAD',
      diffFilter: 'ACMR',
    });
  });
});
