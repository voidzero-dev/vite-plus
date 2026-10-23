import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { prepareTsNodeRun } from '../migrator/ts-node-config.ts';

describe('prepareTsNodeRun', () => {
  let cwd: string;

  beforeEach(() => {
    cwd = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-ts-node-config-'));
    fs.mkdirSync(path.join(cwd, 'scripts'));
    fs.writeFileSync(path.join(cwd, 'scripts/a.ts'), 'export {};\n');
  });

  afterEach(() => {
    fs.rmSync(cwd, { recursive: true, force: true });
  });

  function config(value: unknown, file = 'tsconfig.json') {
    fs.writeFileSync(path.join(cwd, file), JSON.stringify(value));
  }

  function prepare(args: string[], environment: string[] = [], executable = 'ts-node') {
    return prepareTsNodeRun(args, {
      cwd,
      environment,
      executable,
      nodeLoader: executable === 'node',
    });
  }

  it.each(['-T', '--transpileOnly', '--transpile-only'])(
    'removes %s while leaving script arguments intact',
    (flag) => {
      expect(prepare([flag, 'scripts/a.ts', '--typeCheck'])).toEqual({ edits: [{ index: 0 }] });
    },
  );

  it('keeps commands that type-check by default, including the ESM binary', () => {
    for (const executable of ['ts-node', 'ts-node-esm']) {
      expect(prepare(['scripts/a.ts'], [], executable).keepReason).toContain('type-checks');
    }
    expect(prepare(['scripts/a.ts'], [], 'ts-node-transpile-only')).toEqual({ edits: [] });
  });

  it('reads transpile-only mode from the nearest entrypoint config', () => {
    config({ 'ts-node': { transpileOnly: false } });
    config({ 'ts-node': { transpileOnly: true } }, 'scripts/tsconfig.json');
    expect(prepare(['scripts/a.ts'])).toEqual({
      edits: [],
      prependArgs: ['--tsconfig=scripts/tsconfig.json'],
    });
    expect(prepare(['--esm', 'scripts/a.ts'])).toEqual({
      edits: [{ index: 0 }],
      prependArgs: ['--tsconfig=scripts/tsconfig.json'],
    });
  });

  it('reads JSONC and local inherited ts-node settings', () => {
    config({ 'ts-node': { transpileOnly: true, files: true } }, 'base.json');
    fs.writeFileSync(
      path.join(cwd, 'tsconfig.json'),
      '{ // project config\n"extends": "./base", "ts-node": { "esm": true, }, }',
    );
    expect(prepare(['scripts/a.ts'])).toEqual({
      edits: [],
      prependArgs: ['--tsconfig=tsconfig.json'],
    });
  });

  it('merges environment, inherited config, and CLI in order', () => {
    config({ 'ts-node': { transpileOnly: true } }, 'base.json');
    config({ extends: './base.json', 'ts-node': { transpileOnly: false } });
    expect(prepare(['scripts/a.ts'], ['TS_NODE_TRANSPILE_ONLY=true']).keepReason).toContain(
      'type-checks',
    );
    expect(prepare(['-T', 'scripts/a.ts'], ['TS_NODE_TRANSPILE_ONLY=false'])).toEqual({
      edits: [{ index: 0 }],
      prependArgs: ['--tsconfig=tsconfig.json'],
    });
  });

  it.each([
    'TS_NODE_TRANSPILE_ONLY=true',
    'TS_NODE_TRANSPILE_ONLY=1',
    'TS_NODE_TRANSPILE_ONLY=yes',
  ])('recognizes %s', (assignment) => {
    expect(prepare(['scripts/a.ts'], [assignment])).toEqual({ edits: [] });
  });

  it('keeps type checking even when transpile-only is also enabled', () => {
    expect(prepare(['-T', '--typeCheck', 'scripts/a.ts']).keepReason).toContain('type-checks');
    expect(prepare(['-T', 'scripts/a.ts'], ['TS_NODE_TYPE_CHECK=true']).keepReason).toContain(
      'type-checks',
    );
    config({ 'ts-node': { typeCheck: true } });
    expect(prepare(['-T', 'scripts/a.ts']).keepReason).toContain('type-checks');
    expect(
      prepare(['--require', 'ts-node/register/transpile-only', 'scripts/a.ts'], [], 'node')
        .keepReason,
    ).toContain('type-checks');
    expect(prepare(['-T', '--type-check=false', 'scripts/a.ts']).keepReason).toContain(
      'enabled flag',
    );
  });

  it.each([
    'ts-node/register/transpile-only',
    'ts-node/esm/transpile-only',
    'ts-node/esm/transpile-only.mjs',
  ])('recognizes the explicit transpile-only loader %s', (loader) => {
    expect(prepare(['--loader', loader, 'scripts/a.ts'], [], 'node')).toEqual({ edits: [] });
  });

  it('finds a Node preload configuration from cwd rather than the script directory', () => {
    config({ 'ts-node': { transpileOnly: true } });
    config({ 'ts-node': { typeCheck: true } }, 'scripts/tsconfig.json');
    expect(prepare(['--require=ts-node/register', 'scripts/a.ts'], [], 'node')).toEqual({
      edits: [],
      prependArgs: ['--tsconfig=tsconfig.json'],
    });
  });

  it.each(['-P', '--project', '--project='])('maps %s to a checked tsconfig file', (flag) => {
    config({ 'ts-node': { transpileOnly: true } }, 'scripts/config.json');
    const inline = flag.endsWith('=');
    const args = inline ? [`${flag}scripts/config.json`] : [flag, 'scripts/config.json'];
    const expected = [{ index: 0, value: '--tsconfig=scripts/config.json' }];
    expect(prepare([...args, 'scripts/a.ts'])).toEqual({
      edits: inline ? expected : [...expected, { index: 1 }],
    });
  });

  it('maps project directories and gives CLI project selection precedence over the environment', () => {
    config({ 'ts-node': { transpileOnly: true } }, 'scripts/tsconfig.json');
    expect(prepare(['-P', 'scripts', 'scripts/a.ts'], ['TS_NODE_PROJECT=missing.json'])).toEqual({
      edits: [{ index: 0, value: '--tsconfig=scripts/tsconfig.json' }, { index: 1 }],
    });
    expect(prepare(['scripts/a.ts'], ['TS_NODE_PROJECT=scripts/tsconfig.json'])).toEqual({
      edits: [],
    });
    expect(prepare(['scripts/a.ts'], ['TS_NODE_PROJECT=scripts']).keepReason).toContain(
      'names a directory',
    );
  });

  it('keeps relative project paths portable when the working directory contains spaces', () => {
    const spaced = path.join(cwd, 'my project');
    fs.mkdirSync(spaced);
    config({ 'ts-node': { transpileOnly: true } }, 'my project/ts config.json');
    const result = prepareTsNodeRun(['-P', './ts config.json', 'a.ts'], {
      cwd: spaced,
      environment: [],
      executable: 'ts-node',
      nodeLoader: false,
    });
    expect(result).toEqual({
      edits: [{ index: 0, value: '--tsconfig=./ts config.json' }, { index: 1 }],
    });
    expect(JSON.stringify(result)).not.toContain(cwd);
  });

  it('keeps repeated project flags instead of leaving one untranslated', () => {
    expect(prepare(['-T', '-P', 'one.json', '-P', 'two.json', 'a.ts']).keepReason).toContain(
      'more than once',
    );
  });

  it.each([
    ['--swc'],
    ['-C', 'ttypescript'],
    ['-O', '{"module":"CommonJS"}'],
    ['--transpiler', 'custom'],
    ['--skipProject'],
    ['--cwdMode'],
    ['--showConfig'],
    ['--unknown'],
  ])('keeps unsupported ts-node options %s', (...flags) => {
    expect(prepare(['-T', ...flags, 'scripts/a.ts']).keepReason).toContain('does not translate');
  });

  it.each([
    { compilerOptions: { module: 'CommonJS' } },
    { require: ['./setup.ts'] },
    { moduleTypes: { '**/*.ts': 'cjs' } },
    { swc: true },
    { transpiler: 'custom' },
    { experimentalResolver: true },
  ])('keeps ts-node-specific config %j', (settings) => {
    config({ 'ts-node': { transpileOnly: true, ...settings } });
    expect(prepare(['scripts/a.ts']).keepReason).toContain('does not translate');
  });

  it.each(['TS_NODE_COMPILER_OPTIONS={}', 'TS_NODE_REQUIRE=./setup.ts', 'TS_NODE_SKIP_PROJECT=1'])(
    'keeps unsupported environment setting %s',
    (setting) => {
      expect(prepare(['-T', 'scripts/a.ts'], [setting]).keepReason).toContain('does not translate');
    },
  );

  it('keeps config failures and unresolved inheritance instead of assuming type checking is off', () => {
    expect(prepare(['-T', '-P', 'missing.json', 'scripts/a.ts']).keepReason).toContain(
      'cannot be read',
    );
    config({ extends: 'ts-node/node16/tsconfig.json' });
    expect(prepare(['-T', 'scripts/a.ts']).keepReason).toContain('cannot resolve');
    config({ extends: './tsconfig.json' });
    expect(prepare(['-T', 'scripts/a.ts']).keepReason).toContain('circular');
    fs.writeFileSync(path.join(cwd, 'tsconfig.json'), '{ "ts-node":');
    expect(prepare(['-T', 'scripts/a.ts']).keepReason).toContain('cannot be parsed');
  });

  it('keeps commands when a shell state change makes their config unknown', () => {
    expect(
      prepareTsNodeRun(['-T', 'scripts/a.ts'], {
        environment: [],
        executable: 'ts-node',
        nodeLoader: false,
      }).keepReason,
    ).toContain('working directory is unknown');
  });

  it('preserves additional Node options in loader mode', () => {
    expect(
      prepare(
        ['--conditions', 'dev', '--require', 'ts-node/register/transpile-only', 'scripts/a.ts'],
        [],
        'node',
      ),
    ).toEqual({ edits: [] });
  });

  it.each(['--env-file', '--env-file-if-exists', '--env_file', '--env_file_if_exists'])(
    'keeps a Node loader with %s because the file can enable type checking',
    (flag) => {
      fs.writeFileSync(path.join(cwd, '.env'), 'TS_NODE_TYPE_CHECK=true\n');
      for (const envArgs of [[flag, '.env'], [`${flag}=.env`]]) {
        expect(
          prepare(
            ['--require', 'ts-node/register/transpile-only', ...envArgs, 'scripts/a.ts'],
            [],
            'node',
          ).keepReason,
        ).toContain('can configure ts-node');
      }
    },
  );

  it.each(['./setup.cjs', 'dotenv/config'])(
    'keeps the additional preload %s because it can enable type checking',
    (preload) => {
      fs.writeFileSync(path.join(cwd, 'setup.cjs'), "process.env.TS_NODE_TYPE_CHECK = 'true';\n");
      expect(
        prepare(
          ['-r', preload, '-r', 'ts-node/register/transpile-only', 'scripts/a.ts'],
          [],
          'node',
        ).keepReason,
      ).toContain('additional preload');
      expect(prepare(['-T', '-r', preload, 'scripts/a.ts']).keepReason).toContain(
        'additional preload',
      );
    },
  );

  it.each([
    '--config-file=node.config.json',
    '--experimental-config-file=node.config.json',
    '--experimental-default-config-file',
    '--experimental_config_file=node.config.json',
  ])('keeps Node configuration option %s because it can load preloads', (flag) => {
    expect(
      prepare([flag, '-r', 'ts-node/register/transpile-only', 'scripts/a.ts'], [], 'node')
        .keepReason,
    ).toContain('can load preloads');
  });

  it('retains noncanonical environment names instead of disabling POSIX type checking', () => {
    expect(prepare(['scripts/a.ts'], ['TS_NODE_transpile_only=true']).keepReason).toContain(
      'casing',
    );
  });

  it('follows entrypoint symlinks when deciding whether type checking is enabled', () => {
    fs.mkdirSync(path.join(cwd, 'actual'));
    config({ 'ts-node': { typeCheck: true } }, 'actual/tsconfig.json');
    config({ 'ts-node': { transpileOnly: true } }, 'scripts/tsconfig.json');
    fs.writeFileSync(path.join(cwd, 'actual/a.ts'), 'export {};\n');
    fs.rmSync(path.join(cwd, 'scripts/a.ts'));
    fs.symlinkSync(path.join(cwd, 'actual/a.ts'), path.join(cwd, 'scripts/a.ts'));
    expect(prepare(['-T', 'scripts/a.ts']).keepReason).toContain('type-checks');
  });

  it('retains unavailable and glob entrypoints instead of guessing their config', () => {
    for (const entry of ['scripts/missing.ts', 'scripts/*/*.ts']) {
      expect(prepare(['-T', entry]).keepReason).toContain('cannot be resolved');
    }
  });

  it('validates literal NODE_OPTIONS loader arguments before preserving Node.js', () => {
    const context = {
      cwd,
      environment: ['NODE_OPTIONS=--require ts-node/register/transpile-only'],
      executable: 'node',
      nodeLoader: true,
      publicRegister: true,
      nodeOptionsArgs: ['--require', 'ts-node/register/transpile-only'],
    };
    expect(prepareTsNodeRun(['scripts/a.ts'], context)).toEqual({ edits: [] });
    expect(
      prepareTsNodeRun(['scripts/a.ts'], { ...context, nodeOptionsArgs: undefined }).keepReason,
    ).toContain('cannot be checked');
    expect(
      prepareTsNodeRun(['scripts/a.ts'], {
        ...context,
        nodeOptionsArgs: ['-r', 'ts-node/register'],
      }).keepReason,
    ).toContain('type-checks');
    expect(
      prepareTsNodeRun(['scripts/a.ts'], {
        ...context,
        nodeOptionsArgs: [...context.nodeOptionsArgs, '-r', 'dotenv/config'],
      }).keepReason,
    ).toContain('additional preload');
  });

  it('does not pass an implicit tsconfig pin to Node.js', () => {
    config({ 'ts-node': { transpileOnly: true } });
    const context = {
      cwd,
      environment: [],
      executable: 'node',
      nodeLoader: true,
      publicRegister: true,
    };
    expect(
      prepareTsNodeRun(['-r', 'ts-node/register', 'scripts/a.ts'], context).keepReason,
    ).toContain('set TS_NODE_PROJECT');
    expect(
      prepareTsNodeRun(['-r', 'ts-node/register', 'scripts/a.ts'], {
        ...context,
        environment: ['TS_NODE_PROJECT=tsconfig.json'],
      }),
    ).toEqual({ edits: [] });
  });

  it.each(['VP_SCRIPT_TSCONFIG', 'TSX_TSCONFIG_PATH', 'OXC_TSCONFIG_PATH'])(
    'retains %s because the public preload gives it different precedence',
    (name) => {
      expect(
        prepareTsNodeRun(['-r', 'ts-node/register/transpile-only', 'scripts/a.ts'], {
          cwd,
          environment: [`${name}=other.json`],
          executable: 'node',
          nodeLoader: true,
          publicRegister: true,
        }).keepReason,
      ).toContain('different configuration');
    },
  );
});
