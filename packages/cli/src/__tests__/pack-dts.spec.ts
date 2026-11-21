import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import {
  hasExplicitDtsGenerator,
  readIsolatedDeclarations,
  selectDtsGenerator,
  TSRS_EXPERIMENTAL_WARNING,
  withTsrsWarningSuppressed,
  type DtsGeneratorEnvironment,
} from '../pack-dts.js';

const TSRS_URL = 'file:///bundled/tsrs/lib/version.cjs';
const TSRS_NOTICE =
  'tsrs support is highly experimental and may change or break without notice. Some options will be unavailable.';

const tempDirs: string[] = [];

function createProject(files: Record<string, string>) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-pack-dts-'));
  tempDirs.push(dir);
  for (const [name, content] of Object.entries(files)) {
    const file = path.join(dir, name);
    fs.mkdirSync(path.dirname(file), { recursive: true });
    fs.writeFileSync(file, content);
  }
  return dir;
}

afterEach(() => {
  for (const dir of tempDirs.splice(0, tempDirs.length)) {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

function environment(typescriptPath?: string): DtsGeneratorEnvironment {
  return {
    resolveTypeScript: () => typescriptPath,
    tsrsModuleUrl: () => TSRS_URL,
  };
}

const noTypeScript = environment();
const PLAIN_TSCONFIG = JSON.stringify({ compilerOptions: { strict: true } });

function config(cwd: string, dts: unknown, tsconfig: false | string = 'tsconfig.json') {
  return { cwd, dts, tsconfig: tsconfig && path.join(cwd, tsconfig) };
}

describe('selectDtsGenerator', () => {
  it('selects bundled tsrs when typescript does not resolve', () => {
    const cwd = createProject({ 'tsconfig.json': PLAIN_TSCONFIG });

    expect(selectDtsGenerator(config(cwd, {}), noTypeScript)).toEqual({
      kind: 'tsrs',
      dts: { generator: 'tsrs', tsrs: { moduleUrl: TSRS_URL } },
    });
  });

  it('keeps other declaration options when it selects tsrs', () => {
    const cwd = createProject({ 'tsconfig.json': PLAIN_TSCONFIG });
    const dts = { sourcemap: true, tsrs: { vfs: true } };

    expect(selectDtsGenerator(config(cwd, dts), noTypeScript).dts).toEqual({
      generator: 'tsrs',
      sourcemap: true,
      tsrs: { vfs: true, moduleUrl: TSRS_URL },
    });
    expect(dts).toEqual({ sourcemap: true, tsrs: { vfs: true } });
  });

  it('keeps upstream selection when typescript resolves', () => {
    const cwd = createProject({ 'tsconfig.json': PLAIN_TSCONFIG });
    const dts = {};

    expect(selectDtsGenerator(config(cwd, dts), environment('/ts/package.json'))).toEqual({
      kind: 'unchanged',
      dts,
      typescriptPath: '/ts/package.json',
    });
  });

  it('leaves disabled declarations unchanged', () => {
    const cwd = createProject({ 'tsconfig.json': PLAIN_TSCONFIG });

    expect(selectDtsGenerator(config(cwd, false), noTypeScript)).toEqual({
      kind: 'unchanged',
      dts: false,
    });
  });

  it.each(['tsc', 'oxc', 'tsgo', { name: 'custom' }])(
    'keeps an explicit %s generator',
    (generator) => {
      const cwd = createProject({ 'tsconfig.json': PLAIN_TSCONFIG });
      const dts = { generator };

      expect(selectDtsGenerator(config(cwd, dts), noTypeScript)).toEqual({
        kind: 'unchanged',
        dts,
      });
    },
  );

  it('supplies the bundled tsrs to an explicit tsrs generator', () => {
    const cwd = createProject({ 'tsconfig.json': PLAIN_TSCONFIG });

    expect(
      selectDtsGenerator(config(cwd, { generator: 'tsrs' }), environment('/ts/package.json')),
    ).toEqual({
      kind: 'explicit-tsrs',
      dts: { generator: 'tsrs', tsrs: { moduleUrl: TSRS_URL } },
    });
  });

  it('keeps a user tsrs moduleUrl', () => {
    const cwd = createProject({ 'tsconfig.json': PLAIN_TSCONFIG });
    const dts = { generator: 'tsrs', tsrs: { moduleUrl: 'file:///custom/tsrs.cjs' } };

    expect(selectDtsGenerator(config(cwd, dts), noTypeScript).dts).toBe(dts);
  });

  it.each([
    ['tsc.vue', { tsc: { vue: true } }],
    ['the deprecated vue option', { vue: true }],
    ['custom languages', { customLanguages: [{}] }],
  ])('keeps upstream selection for %s', (_name, dts) => {
    const cwd = createProject({ 'tsconfig.json': PLAIN_TSCONFIG });

    expect(selectDtsGenerator(config(cwd, dts), noTypeScript).kind).toBe('unchanged');
  });

  it('keeps upstream selection when no tsconfig is used', () => {
    const cwd = createProject({ 'tsconfig.json': PLAIN_TSCONFIG });

    expect(selectDtsGenerator(config(cwd, {}, false), noTypeScript).kind).toBe('unchanged');
    expect(selectDtsGenerator(config(cwd, { tsconfig: false }), noTypeScript).kind).toBe(
      'unchanged',
    );
    expect(selectDtsGenerator(config(cwd, { tsconfig: 'missing.json' }), noTypeScript).kind).toBe(
      'unchanged',
    );
  });

  it('reads the tsconfig that dts.tsconfig selects, relative to dts.cwd', () => {
    const cwd = createProject({
      'tsconfig.json': PLAIN_TSCONFIG,
      'pkg/tsconfig.build.json': JSON.stringify({
        compilerOptions: { isolatedDeclarations: true },
      }),
    });
    const dts = { cwd: path.join(cwd, 'pkg'), tsconfig: 'tsconfig.build.json' };

    expect(selectDtsGenerator(config(cwd, dts), noTypeScript).kind).toBe('unchanged');
  });

  it('discovers the nearest tsconfig from dts.cwd when dts.tsconfig is true', () => {
    const cwd = createProject({
      'tsconfig.json': JSON.stringify({ compilerOptions: { isolatedDeclarations: true } }),
    });
    const dts = { cwd: path.join(cwd, 'src', 'nested'), tsconfig: true };
    fs.mkdirSync(dts.cwd, { recursive: true });

    expect(selectDtsGenerator(config(cwd, dts, false), noTypeScript).kind).toBe('unchanged');
  });

  it('keeps Oxc for isolated declarations', () => {
    const cwd = createProject({
      'tsconfig.json': JSON.stringify({ compilerOptions: { isolatedDeclarations: true } }),
    });

    expect(selectDtsGenerator(config(cwd, {}), noTypeScript).kind).toBe('unchanged');
  });

  it('keeps Oxc when dts.compilerOptions enables isolated declarations', () => {
    const cwd = createProject({ 'tsconfig.json': PLAIN_TSCONFIG });
    const dts = { compilerOptions: { isolatedDeclarations: true } };

    expect(selectDtsGenerator(config(cwd, dts), noTypeScript).kind).toBe('unchanged');
  });

  it('keeps upstream selection when the tsconfig cannot be read', () => {
    const cwd = createProject({
      'tsconfig.json': JSON.stringify({ extends: './missing.json' }),
    });

    expect(selectDtsGenerator(config(cwd, {}), noTypeScript).kind).toBe('unchanged');
  });

  it.each([
    ['dts.compilerOptions', { compilerOptions: { stripInternal: true } }],
    ['dts.tsconfigRaw', { tsconfigRaw: { include: ['src'] } }],
  ])('rejects %s without typescript', (name, dts) => {
    const cwd = createProject({ 'tsconfig.json': PLAIN_TSCONFIG });

    expect(() => selectDtsGenerator(config(cwd, dts), noTypeScript)).toThrow(
      `\`${name}\` cannot be used without \`typescript\`. The bundled tsrs declaration generator reads compiler options only from the tsconfig file. Move these options into ${path.join(cwd, 'tsconfig.json')}, or install \`typescript\`.`,
    );
  });

  it('accepts empty compiler option overrides', () => {
    const cwd = createProject({ 'tsconfig.json': PLAIN_TSCONFIG });

    expect(
      selectDtsGenerator(config(cwd, { compilerOptions: {}, tsconfigRaw: {} }), noTypeScript).kind,
    ).toBe('tsrs');
  });
});

describe('readIsolatedDeclarations', () => {
  it('reads JSONC', () => {
    const cwd = createProject({
      'tsconfig.json': `{
  // comment
  "compilerOptions": { "isolatedDeclarations": true, },
}`,
    });

    expect(readIsolatedDeclarations(path.join(cwd, 'tsconfig.json'))).toBe(true);
  });

  it('follows relative extends without the .json extension', () => {
    const cwd = createProject({
      'tsconfig.json': JSON.stringify({ extends: './configs/base' }),
      'configs/base.json': JSON.stringify({ compilerOptions: { isolatedDeclarations: true } }),
    });

    expect(readIsolatedDeclarations(path.join(cwd, 'tsconfig.json'))).toBe(true);
  });

  it('follows package extends', () => {
    const cwd = createProject({
      'tsconfig.json': JSON.stringify({ extends: '@scope/tsconfig' }),
      'node_modules/@scope/tsconfig/package.json': JSON.stringify({ name: '@scope/tsconfig' }),
      'node_modules/@scope/tsconfig/tsconfig.json': JSON.stringify({
        compilerOptions: { isolatedDeclarations: true },
      }),
    });

    expect(readIsolatedDeclarations(path.join(cwd, 'tsconfig.json'))).toBe(true);
  });

  it('lets later extends entries and the file itself override earlier values', () => {
    const cwd = createProject({
      'tsconfig.json': JSON.stringify({ extends: ['./on.json', './off.json'] }),
      'override.json': JSON.stringify({
        extends: './on.json',
        compilerOptions: { isolatedDeclarations: false },
      }),
      'reset.json': JSON.stringify({
        extends: './on.json',
        compilerOptions: { isolatedDeclarations: null },
      }),
      'on.json': JSON.stringify({ compilerOptions: { isolatedDeclarations: true } }),
      'off.json': JSON.stringify({ compilerOptions: { isolatedDeclarations: false } }),
    });

    expect(readIsolatedDeclarations(path.join(cwd, 'tsconfig.json'))).toBe(false);
    expect(readIsolatedDeclarations(path.join(cwd, 'override.json'))).toBe(false);
    expect(readIsolatedDeclarations(path.join(cwd, 'reset.json'))).toBe(false);
  });

  it('returns undefined for an unreadable chain', () => {
    const cwd = createProject({
      'invalid.json': '{',
      'cycle-a.json': JSON.stringify({ extends: './cycle-b.json' }),
      'cycle-b.json': JSON.stringify({ extends: './cycle-a.json' }),
      'missing-package.json': JSON.stringify({ extends: '@scope/missing' }),
    });

    for (const file of ['invalid.json', 'cycle-a.json', 'missing-package.json']) {
      expect(readIsolatedDeclarations(path.join(cwd, file)), file).toBeUndefined();
    }
  });
});

describe('warning suppression', () => {
  it('matches the declaration plugin notice for tsrs only', () => {
    expect(TSRS_EXPERIMENTAL_WARNING.test(TSRS_NOTICE)).toBe(true);
    expect(TSRS_EXPERIMENTAL_WARNING.test('TypeScript Go is experimental.')).toBe(false);
  });

  it('keeps user entries', () => {
    expect(withTsrsWarningSuppressed(undefined)).toBe(TSRS_EXPERIMENTAL_WARNING);
    expect(withTsrsWarningSuppressed('custom')).toEqual(['custom', TSRS_EXPERIMENTAL_WARNING]);
    expect(withTsrsWarningSuppressed([/a/, 'b'])).toEqual([/a/, 'b', TSRS_EXPERIMENTAL_WARNING]);

    const suppress = withTsrsWarningSuppressed((message) => message === 'custom');
    expect(typeof suppress).toBe('function');
    const matches = suppress as (message: string) => boolean;
    expect(matches('custom')).toBe(true);
    expect(matches(TSRS_NOTICE)).toBe(true);
    expect(matches('other')).toBe(false);
  });

  it('detects an explicit generator', () => {
    expect(hasExplicitDtsGenerator({ generator: 'tsc' })).toBe(true);
    expect(hasExplicitDtsGenerator({})).toBe(false);
    expect(hasExplicitDtsGenerator(true)).toBe(false);
    expect(hasExplicitDtsGenerator(undefined)).toBe(false);
  });
});
