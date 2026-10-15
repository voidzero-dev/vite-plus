import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { satisfies } from 'verkit';
import { afterEach, describe, expect, it } from 'vitest';

import { oxcNode } from '../../binding/index.js';
import {
  canRegisterSyncHooks,
  HELPER_MODULE_NAME,
  isSupportedNodeVersion,
  rewriteHelperRequires,
  runtimeHelperSpecifier,
  SUPPORTED_NODE_RANGE,
} from '../script-hooks.ts';
import {
  commandName,
  detectScript,
  hasVersionSpec,
  NODE_OPTIONS_WITH_VALUE,
  packageInvocation,
  parseVpxArgs,
  SCRIPT_EXTENSIONS,
  ScriptError,
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
  expect: 'package' | { error: string } | { script: string[]; tsconfig?: string };
}

const detectionTable: {
  scriptExtensions: string[];
  nodeOptionsWithValue: string[];
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
      expect({ nodeArgs: result?.nodeArgs, tsconfig: result?.tsconfig }).toEqual({
        nodeArgs: fill(expected.script),
        tsconfig: expected.tsconfig,
      });
    }
  });

  it('declares the lists the global CLI declares', () => {
    // vpx_script_cases.json holds the lists; vpx_script.rs checks its own against it too.
    expect(SCRIPT_EXTENSIONS).toEqual(detectionTable.scriptExtensions);
    expect([...NODE_OPTIONS_WITH_VALUE]).toEqual(detectionTable.nodeOptionsWithValue);
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

function runPackage(args: string[], cwd: string) {
  const { flags, positional } = parseVpxArgs(args);
  return packageInvocation(flags, positional as [string, ...string[]], cwd);
}

describe('packageInvocation', () => {
  const bin =
    process.platform === 'win32' ? 'node_modules/.bin/eslint.cmd' : 'node_modules/.bin/eslint';

  it('parses package specs the way the global CLI does', () => {
    expect(['eslint', 'eslint@9', '@vue/cli', '@vue/cli@5', '@scope'].map(hasVersionSpec)).toEqual([
      false,
      true,
      false,
      true,
      false,
    ]);
    expect(['eslint@9', '@vue/cli@5.0.0', 'create-vue'].map(commandName)).toEqual([
      'eslint',
      'cli',
      'create-vue',
    ]);
  });

  it('runs a binary installed in node_modules/.bin, from a parent directory too', () => {
    const dir = project({ [bin]: '', 'packages/app/.keep': '' });
    expect(runPackage(['eslint', '.'], path.join(dir, 'packages/app'))).toEqual({
      command: 'exec',
      args: ['eslint', '.'],
    });
  });

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

  it('runs an existing file with exec and a local package directory with dlx', () => {
    const dir = project({ 'deploy.sh': '', 'tools/package.json': '{}' });
    expect(runPackage(['./deploy.sh', 'x'], dir)).toEqual({
      command: 'exec',
      args: ['./deploy.sh', 'x'],
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

  it('rewrites CommonJS helper requires so ES module evaluation can run them', () => {
    const code = rewriteHelperRequires(
      `var _defineProperty = require("${HELPER_MODULE_NAME}/helpers/defineProperty");\nexport function f() {}`,
    );
    expect(code).not.toContain('require(');
    expect(code).toContain(`process.getBuiltinModule("node:module").createRequire("file://`);
    expect(code).toContain('("@oxc-project/runtime/helpers/defineProperty");');
    expect(rewriteHelperRequires('require("./local")')).toBe('require("./local")');
  });
});
