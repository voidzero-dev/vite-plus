import { describe, expect, it } from 'vitest';

import { parseLiteralCommands } from '../../../binding/index.js';
import { getCommandStartIndex, literalArgv, literalCommands } from '../shell-command.ts';

describe('native literal command analysis', () => {
  it('returns UTF-16 ranges into the original source, including quoted words', () => {
    const source = `  echo '你好🦀' && MODE="café" pnpm exec v"p" config "--no-hooks" || echo 'e\u0301'`;
    const commands = parseLiteralCommands(source, true)!;
    expect(commands.map((tokens) => tokens.map(({ value }) => value))).toEqual([
      ['echo', '你好🦀'],
      ['MODE=café', 'pnpm', 'exec', 'vp', 'config', '--no-hooks'],
      ['echo', 'e\u0301'],
    ]);
    expect(
      commands.map((tokens) => tokens.map(({ start, end }) => source.slice(start, end))),
    ).toEqual([
      ['echo', "'你好🦀'"],
      ['MODE="café"', 'pnpm', 'exec', 'v"p"', 'config', '"--no-hooks"'],
      ['echo', "'e\u0301'"],
    ]);
    const flag = commands[1].at(-1)!;
    expect(source.slice(0, flag.start) + '--hooks' + source.slice(flag.end)).toBe(
      `  echo '你好🦀' && MODE="café" pnpm exec v"p" config --hooks || echo 'e\u0301'`,
    );
  });

  it.each(['pnpm exec', 'npm exec --', 'yarn', 'bun x', 'npx', 'bunx'])(
    'keeps the %s wrapper policy in the caller',
    (wrapper) => {
      const tokens = literalArgv(`${wrapper} vp config`)!;
      const values = tokens.map(({ value }) => value);
      expect(values.slice(getCommandStartIndex(values))).toEqual(['vp', 'config']);
    },
  );

  it('keeps environment assignments and stateful commands for caller policies', () => {
    expect(
      literalCommands(`MODE='unit && extra' vitest; cd child && vp test`)?.map((tokens) =>
        tokens.map(({ value }) => value),
      ),
    ).toEqual([
      ['MODE=unit && extra', 'vitest'],
      ['cd', 'child'],
      ['vp', 'test'],
    ]);
  });

  it('does not interpret command names inside arguments', () => {
    expect(literalArgv('echo vp config --no-hooks')?.map(({ value }) => value)).toEqual([
      'echo',
      'vp',
      'config',
      '--no-hooks',
    ]);
    expect(literalArgv(`vp config "" '$FLAGS; && # literal'`)?.map(({ value }) => value)).toEqual([
      'vp',
      'config',
      '',
      '$FLAGS; && # literal',
    ]);
  });

  it.each([
    'vp config "$FLAGS"',
    'MODE=$(node env.js) vp config',
    'vp config | tee log',
    'vp config > log',
    'vp config &',
    'vp config # comment',
    'vp config\nvp config',
    'vp config &&',
    'vp config;',
    'if true; then vp config; fi',
    'vp config "unterminated',
  ])('rejects unsupported syntax in %s', (source) => {
    expect(parseLiteralCommands(source, true)).toBeNull();
    expect(literalCommands(source)).toBeUndefined();
    expect(literalArgv(source)).toBeUndefined();
  });

  it('keeps single-command and command-list contracts distinct', () => {
    expect(literalArgv(' \t')).toEqual([]);
    expect(literalCommands(' \t')).toBeUndefined();
    expect(literalArgv('vp config && echo done')).toBeUndefined();
    expect(literalCommands('vp config && echo done')).toHaveLength(2);
  });
});
