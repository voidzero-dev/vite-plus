import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

import { resolveViteRoot } from '../resolve-vite.ts';

describe('resolveViteRoot', () => {
  const cwd = resolve('workspace');

  it.each([
    [],
    ['--mode', 'production'],
    ['--outDir', 'dist', '--emptyOutDir'],
    ['--host', '127.0.0.1', '--port', '4173'],
    ['--sourcemap', 'inline'],
  ])('uses the dispatched cwd for %j', (...args) => {
    expect(resolveViteRoot({ cwd, args })).toBe(cwd);
  });

  it.each([
    ['packages/app'],
    ['packages/app', '--mode', 'production'],
    ['--mode', 'production', 'packages/app'],
    ['-m=production', 'packages/app'],
    ['--outDir=dist', 'packages/app'],
    ['--emptyOutDir', 'packages/app'],
    ['--no-clearScreen', 'packages/app'],
    ['-w', 'packages/app'],
    ['--strictPort', 'packages/app'],
    ['--host', '127.0.0.1', 'packages/app'],
    ['--sourcemap', 'inline', 'packages/app'],
    ['packages/app', '--', 'ignored'],
  ])('resolves the positional root in %j', (...args) => {
    expect(resolveViteRoot({ cwd, args })).toBe(resolve(cwd, 'packages/app'));
  });
});
