import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import {
  canRegisterSyncHooks,
  isSupportedNodeVersion,
  rewriteHelperRequires,
  runtimeHelperSpecifier,
  SUPPORTED_NODE_RANGE,
} from '../script-hooks.ts';
import { detectScript, parseVpxArgs, ScriptError } from '../vpx-script.ts';

const dirs: string[] = [];

function project(files: Record<string, string>): string {
  const dir = mkdtempSync(path.join(tmpdir(), 'vpx-script-'));
  dirs.push(dir);
  for (const [name, content] of Object.entries(files)) {
    const file = path.join(dir, name);
    mkdirSync(path.dirname(file), { recursive: true });
    writeFileSync(file, content);
  }
  return dir;
}

afterEach(() => {
  for (const dir of dirs.splice(0)) {
    rmSync(dir, { recursive: true, force: true });
  }
});

/** String items of the list declared as `start`; lists end at `];` or `]);`. */
function extractList(text: string, start: string): string[] | undefined {
  return text
    .slice(text.indexOf(start))
    .split(/\]\)?;/, 1)[0]
    .match(/"[^"]+"|'[^']+'/g)
    ?.map((value) => value.slice(1, -1));
}

describe('detectScript', () => {
  it('runs an explicit path with a script extension', () => {
    const cwd = project({ 'scripts/seed.ts': '' });
    expect(detectScript(['./scripts/seed.ts', '--dry-run'], cwd)).toEqual({
      nodeArgs: ['./scripts/seed.ts', '--dry-run'],
    });
  });

  it('runs a bare relative path to an existing file', () => {
    const cwd = project({ 'scripts/seed.ts': '' });
    expect(detectScript(['scripts/seed.ts'], cwd)).toEqual({ nodeArgs: ['scripts/seed.ts'] });
  });

  it('reports a missing TypeScript file or explicit path instead of downloading', () => {
    const cwd = project({});
    expect(() => detectScript(['seed.ts'], cwd)).toThrow(
      new ScriptError('Script not found: seed.ts'),
    );
    expect(() => detectScript(['./missing.js'], cwd)).toThrow(ScriptError);
  });

  it('keeps package names in package mode', () => {
    const cwd = project({ 'deploy.sh': '#!/bin/sh\n' });
    expect(detectScript(['eslint', '.'], cwd)).toBeUndefined();
    expect(detectScript(['highlight.js'], cwd)).toBeUndefined();
    expect(detectScript(['@vue/cli@5.0.0'], cwd)).toBeUndefined();
    expect(detectScript(['./deploy.sh'], cwd)).toBeUndefined();
  });

  it('runs an extensionless file with a vpx shebang', () => {
    const cwd = project({ 'bin/tool': '#!/usr/bin/env -S vpx --tsconfig x.json\n' });
    const tool = path.join(cwd, 'bin/tool');
    expect(detectScript([tool], cwd)).toEqual({ nodeArgs: [tool] });
  });

  it('forwards leading Node.js options and skips their values', () => {
    const cwd = project({ 'main.ts': '' });
    expect(detectScript(['--import', './setup.ts', './main.ts'], cwd)).toEqual({
      nodeArgs: ['--import', './setup.ts', './main.ts'],
      tsconfig: undefined,
    });
    expect(() => detectScript(['--watch', './missing.ts'], cwd)).toThrow(ScriptError);
    expect(detectScript(['--eval', 'console.log(1)'], cwd)).toEqual({
      nodeArgs: ['--eval', 'console.log(1)'],
      tsconfig: undefined,
    });
  });

  it('extracts --tsconfig from the Node.js options only', () => {
    const cwd = project({ 'a.ts': '' });
    expect(
      detectScript(['--watch', '--tsconfig', 'x.json', './a.ts', '--tsconfig', 'y'], cwd),
    ).toEqual({
      nodeArgs: ['--watch', './a.ts', '--tsconfig', 'y'],
      tsconfig: 'x.json',
    });
  });

  it('matches the global CLI detection table', () => {
    // Keep crates/vp_global_cli/src/commands/vpx_script.rs and this module in sync.
    const rust = readFileSync(
      path.join(import.meta.dirname, '../../../../crates/vp_global_cli/src/commands/vpx_script.rs'),
      'utf8',
    );
    const source = readFileSync(path.join(import.meta.dirname, '../vpx-script.ts'), 'utf8');
    expect(extractList(rust, 'NODE_OPTIONS_WITH_VALUE')?.length).toBeGreaterThan(10);
    expect(extractList(rust, 'SCRIPT_EXTENSIONS')).toHaveLength(8);
    expect(extractList(source, 'NODE_OPTIONS_WITH_VALUE')).toEqual(
      extractList(rust, 'NODE_OPTIONS_WITH_VALUE'),
    );
    expect(extractList(source, 'SCRIPT_EXTENSIONS')).toEqual(
      extractList(rust, 'SCRIPT_EXTENSIONS'),
    );
  });
});

describe('parseVpxArgs', () => {
  it('stops at the first positional or unknown option', () => {
    expect(parseVpxArgs(['--tsconfig=t.json', '-s', '--watch', './a.ts'])).toEqual({
      flags: {
        packages: [],
        shellMode: false,
        silent: true,
        help: false,
        version: false,
        tsconfig: 't.json',
      },
      positional: ['--watch', './a.ts'],
    });
    expect(parseVpxArgs(['-p', 'cowsay', '-c', 'echo hi | cowsay']).flags).toMatchObject({
      packages: ['cowsay'],
      shellMode: true,
    });
    expect(parseVpxArgs(['-v']).flags.version).toBe(true);
    expect(parseVpxArgs(['./a.ts', '--version'])).toMatchObject({
      flags: { version: false },
      positional: ['./a.ts', '--version'],
    });
  });
});

describe('script hooks', () => {
  it('keeps the supported Node.js range in sync with engines.node', () => {
    const pkg = JSON.parse(
      readFileSync(path.join(import.meta.dirname, '../../package.json'), 'utf8'),
    );
    expect(SUPPORTED_NODE_RANGE).toBe(pkg.engines.node);
  });

  it('uses in-thread hooks where registerHooks() has the fixes the hooks need', () => {
    expect(canRegisterSyncHooks('22.18.0')).toBe(false);
    expect(canRegisterSyncHooks('22.22.2')).toBe(false);
    expect(canRegisterSyncHooks('22.22.3')).toBe(true);
    expect(canRegisterSyncHooks('22.23.0')).toBe(true);
    expect(canRegisterSyncHooks('24.11.0')).toBe(false);
    expect(canRegisterSyncHooks('24.11.1')).toBe(true);
    expect(canRegisterSyncHooks('24.12.0')).toBe(true);
    expect(canRegisterSyncHooks('26.0.0')).toBe(true);
  });

  it('checks the supported Node.js range', () => {
    expect(isSupportedNodeVersion('22.18.0')).toBe(true);
    expect(isSupportedNodeVersion('22.17.1')).toBe(false);
    expect(isSupportedNodeVersion('23.11.0')).toBe(false);
    expect(isSupportedNodeVersion('24.11.0')).toBe(true);
    expect(isSupportedNodeVersion('25.0.0')).toBe(false);
    expect(isSupportedNodeVersion('26.0.0')).toBe(true);
    expect(isSupportedNodeVersion('20.19.0')).toBe(false);
  });

  it('maps oxc-node runtime helpers to @oxc-project/runtime', () => {
    expect(runtimeHelperSpecifier('@oxc-node/core/helpers/defineProperty')).toBe(
      '@oxc-project/runtime/helpers/defineProperty',
    );
    expect(runtimeHelperSpecifier('@oxc-node/core')).toBeUndefined();
    expect(runtimeHelperSpecifier('./helpers/defineProperty')).toBeUndefined();
  });

  it('rewrites CommonJS helper requires so ES module evaluation can run them', () => {
    const code = rewriteHelperRequires(
      'var _defineProperty = require("@oxc-node/core/helpers/defineProperty");\nexport function f() {}',
    );
    expect(code).not.toContain('require(');
    expect(code).toContain(`process.getBuiltinModule("node:module").createRequire("file://`);
    expect(code).toContain('("@oxc-project/runtime/helpers/defineProperty");');
    expect(rewriteHelperRequires('require("./local")')).toBe('require("./local")');
  });
});
