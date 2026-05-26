import path from 'node:path';

import { describe, expect, test } from '@voidzero-dev/vite-plus-test';

import { getPnpmInvocation } from '../local-cli.ts';

describe('getPnpmInvocation()', () => {
  test('falls back to pnpm binary when npm_execpath is unset', () => {
    const expectedCommand = process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm';

    expect(getPnpmInvocation(undefined)).toEqual({
      command: expectedCommand,
      args: [],
    });
  });

  test('runs pnpm through node when npm_execpath points to a JS script', () => {
    const execPath = path.join('/tmp', 'pnpm.cjs');

    expect(getPnpmInvocation(execPath)).toEqual({
      command: process.execPath,
      args: [execPath],
    });
  });

  test('runs pnpm directly when npm_execpath points to a native binary', () => {
    const execPath = path.join('/home/runner/setup-pnpm/node_modules/.bin/store', 'pnpm');

    expect(getPnpmInvocation(execPath)).toEqual({
      command: execPath,
      args: [],
    });
  });
});
