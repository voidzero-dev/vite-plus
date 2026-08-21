import { describe, expect, it } from 'vitest';

import { parseStagedArgs } from '../../../binding/index.js';

function expectParsed(argv: string[]) {
  const outcome = parseStagedArgs(argv);
  expect(outcome.status).toBe('ok');
  if (outcome.status !== 'ok') {
    throw new Error(`The parser returned ${outcome.status}. The test expected arguments.`);
  }
  return outcome.value;
}

function expectParseError(argv: string[]) {
  const outcome = parseStagedArgs(argv);
  expect(outcome.status).toBe('error');
  if (outcome.status !== 'error') {
    throw new Error(`The parser returned ${outcome.status}. The test expected an error.`);
  }
  return outcome.error;
}

describe('staged arguments', () => {
  it.each([
    { argv: ['--no-concurrent'], expected: false },
    { argv: ['--concurrent', 'false'], expected: false },
    { argv: ['--concurrent', 'true'], expected: true },
    { argv: ['--concurrent'], expected: true },
    { argv: ['--concurrent=1'], expected: 1 },
    { argv: ['-p', '2'], expected: 2 },
  ])('parses $argv as concurrent=$expected', ({ argv, expected }) => {
    expect(expectParsed(argv).concurrent).toBe(expected);
  });

  it.each([
    { argv: ['--concurrent=0'] },
    { argv: ['-p', '0'] },
    { argv: ['--concurrent=-1'] },
    { argv: ['--concurrent=1.5'] },
    { argv: ['--concurrent=NaN'] },
    { argv: ['--concurrent=4294967296'] },
  ])('rejects invalid concurrency $argv', ({ argv }) => {
    const error = expectParseError(argv);
    expect(error.kind).toBe('invalid-value');
    expect(error.message).toContain('use true, false, or an integer from 1 through 4294967295');
  });

  it.each([['--no-cwd'], ['--no-diff'], ['--no-diff-filter'], ['--stash'], ['--no-debug']])(
    'rejects unsupported option %s',
    (option) => {
      const error = expectParseError([option]);
      expect(error.kind).toBe('unknown-argument');
      expect(error.message).toContain(`unexpected argument '${option}'`);
    },
  );

  it('rejects positional arguments', () => {
    const error = expectParseError(['unexpected']);
    expect(error.kind).toBe('unknown-argument');
  });

  it('rejects missing and repeated values', () => {
    expect(expectParseError(['--cwd']).kind).toBe('invalid-value');
    expect(expectParseError(['--cwd', 'one', '--cwd', 'two']).kind).toBe('argument-conflict');
    expect(expectParseError(['--debug', '--debug']).kind).toBe('argument-conflict');
  });

  it('preserves valid string options', () => {
    expect(
      expectParsed(['--cwd', 'packages/app', '--diff=main...HEAD', '--diff-filter', 'ACMR']),
    ).toEqual({
      cwd: 'packages/app',
      diff: 'main...HEAD',
      diffFilter: 'ACMR',
    });
  });

  it('returns only explicit Boolean options', () => {
    expect(expectParsed(['--allow-empty', '--debug', '--no-stash'])).toEqual({
      allowEmpty: true,
      debug: true,
      stash: false,
    });
  });

});
