import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { satisfies } from 'verkit';
import { afterEach, describe, expect, it } from 'vitest';

import { oxcNode } from '../../binding/index.js';
import {
  canRegisterSyncHooks,
  HELPER_MODULE_NAME,
  isSupportedNodeVersion,
  routeHelperRequires,
  withRequireAsyncModuleCode,
  runtimeHelperSpecifier,
  SUPPORTED_NODE_RANGE,
} from '../script-hooks.ts';
import {
  canExecDirectly,
  checkExplicitProgram,
  commandName,
  detectScript,
  hasVersionSpec,
  NODE_OPTIONS_WITH_VALUE,
  NODE_OPTIONS_WITHOUT_ENTRY,
  packageInvocation,
  parseVpxArgs,
  SCRIPT_EXTENSIONS,
  ScriptError,
  WATCH_OPTIONS_WITH_VALUE,
} from '../vpx-script.ts';

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

interface DetectionCase {
  name: string;
  files?: Record<string, string>;
  args: string[];
  expect: 'package' | { error: string } | { script: string[]; tsconfig?: string; watch?: true };
}

const detectionTable: {
  scriptExtensions: string[];
  nodeOptionsWithValue: string[];
  nodeOptionsWithoutEntry: string[];
  watchOptionsWithValue: string[];
  packageSpecs: { spec: string; versioned: boolean; command: string }[];
  explicitPrograms: {
    name: string;
    files?: Record<string, string>;
    executable?: string[];
    token: string;
    unixOnly?: boolean;
    expect: 'ok' | { error: string; note?: string };
  }[];
  cases: DetectionCase[];
} = JSON.parse(
  readFileSync(
    path.join(
      import.meta.dirname,
      '../../../../crates/vp_global_cli/src/commands/vpx_script_cases.json',
    ),
    'utf8',
  ),
);

describe('detectScript', () => {
  // The global CLI runs the same cases (`vpx_script.rs`), so both decide the same way.
  it.each(detectionTable.cases)('$name', ({ files = {}, args, expect: expected }) => {
    const cwd = project(files);
    const fill = (values: string[]) => values.map((value) => value.replaceAll('{cwd}', cwd));
    if (expected === 'package') {
      expect(detectScript(fill(args), cwd)).toBeUndefined();
    } else if ('error' in expected) {
      expect(() => detectScript(fill(args), cwd)).toThrow(new ScriptError(expected.error));
    } else {
      const result = detectScript(fill(args), cwd);
      expect({
        nodeArgs: result?.nodeArgs,
        tsconfig: result?.tsconfig,
        watch: result?.watch ?? false,
      }).toEqual({
        nodeArgs: fill(expected.script),
        tsconfig: expected.tsconfig,
        watch: expected.watch ?? false,
      });
    }
  });

  it('declares the lists the global CLI declares', () => {
    // vpx_script_cases.json holds the lists; vpx_script.rs checks its own against it too.
    expect(SCRIPT_EXTENSIONS).toEqual(detectionTable.scriptExtensions);
    expect([...NODE_OPTIONS_WITH_VALUE]).toEqual(detectionTable.nodeOptionsWithValue);
    expect([...NODE_OPTIONS_WITHOUT_ENTRY]).toEqual(detectionTable.nodeOptionsWithoutEntry);
    expect([...WATCH_OPTIONS_WITH_VALUE]).toEqual(detectionTable.watchOptionsWithValue);
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
    expect(parseVpxArgs(['--tsconfig'])).toMatchObject({ positional: ['--tsconfig'] });
    expect(parseVpxArgs(['--tsconfig']).flags.tsconfig).toBeUndefined();
    expect(parseVpxArgs(['./a.ts', '--version'])).toMatchObject({
      flags: { version: false },
      positional: ['./a.ts', '--version'],
    });
  });
});

function runPackage(args: string[], cwd: string) {
  const { flags, positional } = parseVpxArgs(args);
  return packageInvocation(flags, positional as [string, ...string[]], cwd);
}

describe('checkExplicitProgram', () => {
  // The global CLI runs the same cases (`vpx_script.rs`).
  it.each(detectionTable.explicitPrograms)(
    '$name',
    ({ files = {}, executable = [], token, unixOnly, expect: expected }) => {
      if (unixOnly && process.platform === 'win32') {
        return;
      }
      const cwd = project(files);
      for (const file of executable) {
        chmodSync(path.join(cwd, file), 0o755);
      }
      expect(checkExplicitProgram(token, cwd)).toEqual(expected === 'ok' ? undefined : expected);
    },
  );
});

describe.skipIf(process.platform === 'win32')('canExecDirectly', () => {
  it('runs a native binary or a script with a native interpreter without the shell', () => {
    const dir = project({
      'sh.sh': '#!/bin/sh\nexit 0\n',
      'env.js': '#!/usr/bin/env node\n',
      'spaced.sh': '#! /bin/sh -e\n',
      'plain.js': 'console.log(1);\n',
      'missing.sh': '#!/missing/interpreter\n',
      'relative.sh': '#!sh\n',
      'crlf.sh': '#!/bin/sh\r\nexit 0\r\n',
      'mode.sh': '#!/bin/sh\n',
    });
    writeFileSync(path.join(dir, 'nested.sh'), `#!${path.join(dir, 'sh.sh')}\n`);
    const file = (name: string) => path.join(dir, name);
    // Every file but `mode.sh` has the execute bit.
    for (const name of [
      'sh.sh',
      'env.js',
      'spaced.sh',
      'plain.js',
      'missing.sh',
      'relative.sh',
      'crlf.sh',
      'nested.sh',
    ]) {
      chmodSync(file(name), 0o755);
    }

    expect(canExecDirectly(process.execPath)).toBe(true);
    expect(['sh.sh', 'env.js', 'spaced.sh'].filter((name) => canExecDirectly(file(name)))).toEqual([
      'sh.sh',
      'env.js',
      'spaced.sh',
    ]);
    // The shell reports these, as a terminal does: no `#!` line, an interpreter that does
    // not exist, is relative, keeps the `\r` the kernel reads, or is itself a script,
    // and a file without the execute bit.
    const viaShell = ['plain.js', 'missing.sh', 'relative.sh', 'crlf.sh', 'nested.sh', 'mode.sh'];
    expect(viaShell.filter((name) => canExecDirectly(file(name)))).toEqual([]);
  });
});

describe('packageInvocation', () => {
  const bin =
    process.platform === 'win32' ? 'node_modules/.bin/eslint.cmd' : 'node_modules/.bin/eslint';

  it.each(detectionTable.packageSpecs)(
    'parses $spec the way the global CLI does',
    ({ spec, versioned, command }) => {
      expect(hasVersionSpec(spec)).toBe(versioned);
      expect(commandName(spec)).toBe(command);
    },
  );

  it('runs the nearest binary in node_modules/.bin, from a parent directory too', () => {
    const dir = project({ [bin]: '', 'packages/app/src/.keep': '', [`packages/lib/${bin}`]: '' });
    chmodSync(path.join(dir, bin), 0o755);
    chmodSync(path.join(dir, 'packages/lib', bin), 0o755);
    expect(runPackage(['eslint', '.'], path.join(dir, 'packages/app/src'))).toEqual({
      command: 'run',
      file: path.join(dir, bin),
      args: ['.'],
    });
    // `packages/lib` has no package.json, so `vp exec eslint` would not search it.
    expect(runPackage(['eslint'], path.join(dir, 'packages/lib'))).toEqual({
      command: 'run',
      file: path.join(dir, 'packages/lib', bin),
      args: [],
    });
  });

  it.skipIf(process.platform === 'win32')(
    'reports an installed binary that is not executable',
    () => {
      expect(runPackage(['eslint'], project({ [bin]: '' }))).toEqual({
        error: `'${bin}' is not executable`,
        note: `Reinstall dependencies with \`vp install\`, or make it executable with \`chmod +x ${bin}\`.`,
      });
    },
  );

  it('reports a binary that is not installed instead of downloading it', () => {
    expect(runPackage(['eslint', '.'], project({}))).toEqual({
      error: "'eslint' is not installed in node_modules",
      note: 'Add it to devDependencies, or run `vp dlx eslint` to download and run it.',
    });
  });

  it('downloads a versioned package, and anything with --package or --shell-mode', () => {
    const dir = project({ [bin]: '' });
    expect(runPackage(['eslint@9', '.'], dir)).toEqual({ command: 'dlx', args: ['eslint@9', '.'] });
    expect(runPackage(['-s', '-p', 'cowsay', '-c', 'echo hi | cowsay'], dir)).toEqual({
      command: 'dlx',
      args: ['--package', 'cowsay', '--shell-mode', '--silent', 'echo hi | cowsay'],
    });
  });

  it('runs an existing executable file and hands a local package directory to dlx', () => {
    const dir = project({ 'deploy.sh': '', 'tools/package.json': '{}' });
    chmodSync(path.join(dir, 'deploy.sh'), 0o755);
    expect(runPackage(['./deploy.sh', 'x'], dir)).toEqual({
      command: 'run',
      file: path.join(dir, 'deploy.sh'),
      args: ['x'],
    });
    expect(runPackage(['./tools'], dir)).toEqual({ command: 'dlx', args: ['./tools'] });
  });
});

describe('script hooks', () => {
  it('keeps the supported Node.js range in sync with engines.node', () => {
    const pkg = JSON.parse(
      readFileSync(path.join(import.meta.dirname, '../../package.json'), 'utf8'),
    );
    expect(SUPPORTED_NODE_RANGE).toBe(pkg.engines.node);
    // The startup check is hand-written to keep a range parser out of the preload.
    for (const major of [20, 21, 22, 23, 24, 25, 26, 27]) {
      for (const minor of [0, 10, 11, 17, 18, 19, 22, 23]) {
        const version = `${major}.${minor}.0`;
        expect(isSupportedNodeVersion(version), version).toBe(satisfies(version, pkg.engines.node));
      }
    }
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

  it('maps oxc-node runtime helpers to @oxc-project/runtime', () => {
    expect(runtimeHelperSpecifier(`${HELPER_MODULE_NAME}/helpers/defineProperty`)).toBe(
      '@oxc-project/runtime/helpers/defineProperty',
    );
    // Helper imports another oxc-node copy emitted keep their own resolution.
    expect(runtimeHelperSpecifier('@oxc-node/core/helpers/defineProperty')).toBeUndefined();
    expect(runtimeHelperSpecifier(HELPER_MODULE_NAME)).toBeUndefined();
    expect(runtimeHelperSpecifier('./helpers/defineProperty')).toBeUndefined();
  });

  it('keeps `using` native where Node.js runs it, which the binding detects when it loads', () => {
    // `detect_runtime_syntax` runs from the binding's `module_exports` hook; without it,
    // `using` would be lowered to a helper on every version.
    const transformer = new oxcNode.OxcTransformer(process.cwd(), HELPER_MODULE_NAME);
    const { source } = transformer.transform(
      path.join(tmpdir(), 'using.ts'),
      '{ using x = { [Symbol.dispose]() {} }; }',
    );
    const major = Number(process.versions.node.split('.')[0]);
    expect(source.includes(`${HELPER_MODULE_NAME}/helpers/usingCtx`)).toBe(major < 24);
  });

  it('resolves helper requires from this package and leaves other requests alone', () => {
    /* oxlint-disable no-underscore-dangle -- Node.js' CommonJS resolver */
    const Module = { _resolveFilename: (request: string) => `original:${request}` };
    routeHelperRequires(Module);
    const helper = Module._resolveFilename(`${HELPER_MODULE_NAME}/helpers/defineProperty`);
    expect(helper).toMatch(/@oxc-project[\\/]runtime[\\/]/);
    expect(path.isAbsolute(helper)).toBe(true);
    expect(Module._resolveFilename('./local')).toBe('original:./local');
    expect(Module._resolveFilename('@oxc-node/core/helpers/defineProperty')).toBe(
      'original:@oxc-node/core/helpers/defineProperty',
    );
    /* oxlint-enable no-underscore-dangle */
  });

  it("gives require() of a module with top-level await Node.js' error code", () => {
    const tla = new Error(
      'require() cannot be used on an ESM graph with top-level await. Use import() instead.\n  /x/dep.mts:1:1',
    );
    expect(withRequireAsyncModuleCode(tla)).toMatchObject({ code: 'ERR_REQUIRE_ASYNC_MODULE' });
    const other = new Error('Failed to parse /x/a.ts');
    expect((withRequireAsyncModuleCode(other) as NodeJS.ErrnoException).code).toBeUndefined();
  });
});
