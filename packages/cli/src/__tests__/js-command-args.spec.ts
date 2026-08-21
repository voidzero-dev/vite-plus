import { describe, expect, it } from 'vitest';

import {
  parseConfigArgs,
  parseCreateArgs,
  parseHooksArgs,
  parseMigrateArgs,
  parseStagedArgs,
} from '../../binding/index.js';

function expectParsed<T>(outcome: { status: string; value?: T }): T {
  expect(outcome.status).toBe('ok');
  if (outcome.status !== 'ok' || outcome.value === undefined) {
    throw new Error(`The parser returned ${outcome.status}.`);
  }
  return outcome.value;
}

function expectParseError(outcome: { status: string; error?: { kind: string; message: string } }) {
  expect(outcome.status).toBe('error');
  if (outcome.status !== 'error' || outcome.error === undefined) {
    throw new Error(`The parser returned ${outcome.status}.`);
  }
  return outcome.error;
}

describe('JavaScript command NAPI arguments', () => {
  it('returns staged field names and only explicit values', () => {
    expect(
      expectParsed(
        parseStagedArgs([
          '--allow-empty',
          '--concurrent=2',
          '--diff-filter',
          'ACMR',
          '--no-stash',
        ]),
      ),
    ).toEqual({ allowEmpty: true, concurrent: 2, diffFilter: 'ACMR', stash: false });
    expect(expectParsed(parseStagedArgs([]))).toEqual({});
  });

  it('returns config field names and only explicit values', () => {
    expect(
      expectParsed(parseConfigArgs(['--hooks-dir', '.custom', '--hooks', '--no-agent'])),
    ).toEqual({ hooksDir: '.custom', hooks: true, agent: false });
    expect(expectParsed(parseConfigArgs([]))).toEqual({});
  });

  it('returns a typed hooks subcommand', () => {
    expect(expectParsed(parseHooksArgs(['enable', '--hooks-dir', '.custom']))).toEqual({
      command: 'enable',
      hooksDir: '.custom',
    });
  });

  it('keeps migrate runtime defaults out of Rust', () => {
    expect(expectParsed(parseMigrateArgs(['project', '--no-interactive', '--no-hooks']))).toEqual({
      path: 'project',
      hooks: false,
      interactive: false,
    });
    expect(expectParsed(parseMigrateArgs([]))).toEqual({});
  });

  it('returns validated create values and exact template arguments', () => {
    expect(
      expectParsed(
        parseCreateArgs([
          'vite',
          '--package-manager',
          'pnpm',
          '--no-agent',
          '--',
          '--template',
          'react-ts',
          '--',
        ]),
      ),
    ).toEqual({
      templateName: 'vite',
      agent: false,
      packageManager: 'pnpm',
      templateArgs: ['--template', 'react-ts', '--'],
    });
  });

  it.each([
    ['config', () => parseConfigArgs(['--no-hooks-dir'])],
    ['hooks', () => parseHooksArgs(['enable', '--no-hooks-dir'])],
    ['migrate', () => parseMigrateArgs(['--no-full'])],
    ['create', () => parseCreateArgs(['--all'])],
  ])('returns strict errors for %s', (_command, parse) => {
    expect(expectParseError(parse()).kind).toBe('unknown-argument');
  });
});
