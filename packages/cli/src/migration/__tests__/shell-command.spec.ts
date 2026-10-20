import { describe, expect, it } from 'vitest';

import { parseLiteralCommands } from '../../../binding/index.js';
import { getCommandStartIndex } from '../migrator/shared.ts';

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
      const tokens = parseLiteralCommands(`${wrapper} vp config`, false)![0];
      const values = tokens.map(({ value }) => value);
      expect(values.slice(getCommandStartIndex(values))).toEqual(['vp', 'config']);
    },
  );

  it('keeps environment assignments and stateful commands for caller policies', () => {
    expect(
      parseLiteralCommands(`MODE='unit && extra' vitest; cd child && vp test`, true)?.map(
        (tokens) => tokens.map(({ value }) => value),
      ),
    ).toEqual([
      ['MODE=unit && extra', 'vitest'],
      ['cd', 'child'],
      ['vp', 'test'],
    ]);
  });

  it('does not interpret command names inside arguments', () => {
    expect(
      parseLiteralCommands('echo vp config --no-hooks', false)?.[0].map(({ value }) => value),
    ).toEqual(['echo', 'vp', 'config', '--no-hooks']);
    expect(
      parseLiteralCommands(`vp config "" '$FLAGS; && # literal'`, false)?.[0].map(
        ({ value }) => value,
      ),
    ).toEqual(['vp', 'config', '', '$FLAGS; && # literal']);
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
    expect(parseLiteralCommands(source, false)).toBeNull();
  });

  it('keeps single-command and command-list contracts distinct', () => {
    expect(parseLiteralCommands(' \t', false)).toEqual([[]]);
    expect(parseLiteralCommands(' \t', true)).toBeNull();
    expect(parseLiteralCommands('vp config && echo done', false)).toBeNull();
    expect(parseLiteralCommands('vp config && echo done', true)).toHaveLength(2);
  });
});
