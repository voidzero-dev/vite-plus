import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import type { ResolvedConfig } from '@voidzero-dev/vite-plus-core/pack';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { configureTypeScript7Dts } from '../utils/typescript-dts.js';

function writeJson(file: string, value: unknown): void {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, JSON.stringify(value));
}

describe('configureTypeScript7Dts', () => {
  let cwd: string;

  beforeEach(() => {
    cwd = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-typescript-dts-'));
    fs.writeFileSync(path.join(cwd, 'package.json'), '{}');
  });

  afterEach(() => {
    fs.rmSync(cwd, { recursive: true, force: true });
  });

  function installTypeScript(version: string, includeCompiler = true): string {
    writeJson(path.join(cwd, 'node_modules/typescript/package.json'), {
      name: 'typescript',
      version,
      exports: { './package.json': './package.json' },
    });

    const platformPackage = `typescript-${process.platform}-${process.arch}`;
    const platformRoot = path.join(cwd, 'node_modules/@typescript', platformPackage);
    if (includeCompiler) {
      writeJson(path.join(platformRoot, 'package.json'), {
        name: `@typescript/${platformPackage}`,
        version,
        exports: { './package.json': './package.json' },
      });
      const executable = path.join(
        platformRoot,
        'lib',
        process.platform === 'win32' ? 'tsc.exe' : 'tsc',
      );
      fs.mkdirSync(path.dirname(executable), { recursive: true });
      fs.writeFileSync(executable, '');
      return executable;
    }
    return path.join(platformRoot, 'lib', 'tsc');
  }

  function config(dts: ResolvedConfig['dts']): ResolvedConfig {
    return { cwd, dts } as ResolvedConfig;
  }

  it('uses the native compiler shipped with TypeScript 7', () => {
    const executable = installTypeScript('7.0.2');
    const resolved = config({});

    configureTypeScript7Dts(resolved);

    expect(resolved.dts && resolved.dts.tsgo).toEqual({ path: fs.realpathSync(executable) });
  });

  it('preserves a custom native compiler path', () => {
    installTypeScript('7.0.2', false);
    const resolved = config({ tsgo: { path: '/custom/tsc' } });

    configureTypeScript7Dts(resolved);

    expect(resolved.dts && resolved.dts.tsgo).toEqual({ path: '/custom/tsc' });
  });

  it('leaves the Oxc declaration pipeline unchanged', () => {
    installTypeScript('7.0.2', false);
    const resolved = config({ oxc: true });

    configureTypeScript7Dts(resolved);

    expect(resolved.dts).toEqual({ oxc: true });
  });

  it('rejects an explicitly disabled native declaration pipeline', () => {
    installTypeScript('7.0.2', false);

    expect(() => configureTypeScript7Dts(config({ tsgo: false }))).toThrow(
      'TypeScript 7 declaration generation requires',
    );
  });

  it('keeps the legacy pipeline for TypeScript 6', () => {
    installTypeScript('6.0.2', false);
    const resolved = config({});

    configureTypeScript7Dts(resolved);

    expect(resolved.dts).toEqual({});
  });
});
