import fs from 'node:fs';
import path from 'node:path';
import url from 'node:url';

import { describe, expect, it } from 'vitest';

import cliPkgJson from '../../cli/package.json' with { type: 'json' };
import {
  getNativePlatformPackageName,
  getNativePlatformPackageNames,
} from '../build-support/native-platform-packages.ts';
import corePkgJson from '../package.json' with { type: 'json' };

const coreDir = path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), '..');
const distDir = path.join(coreDir, 'dist');

describe('build artifacts', () => {
  it('should include esm-shims.js in dist for tsdown shims support', () => {
    const shimsPath = path.join(distDir, 'esm-shims.js');
    expect(fs.existsSync(shimsPath), `${shimsPath} should exist`).toBe(true);

    const content = fs.readFileSync(shimsPath, 'utf8');
    expect(content).toContain('__dirname');
    expect(content).toContain('__filename');
  });

  it('should include tsdown client.d.ts in dist/tsdown for pack/client support', () => {
    const clientPath = path.join(distDir, 'tsdown/client.d.ts');
    expect(fs.existsSync(clientPath), `${clientPath} should exist`).toBe(true);

    const content = fs.readFileSync(clientPath, 'utf8');
    expect(content).toContain('ImportMeta');
    expect(content).toContain('glob');
  });

  it('maps CLI NAPI targets to Vite+ native platform packages', () => {
    expect(getNativePlatformPackageNames(cliPkgJson.napi.targets)).toEqual([
      '@voidzero-dev/vite-plus-darwin-arm64',
      '@voidzero-dev/vite-plus-darwin-x64',
      '@voidzero-dev/vite-plus-linux-arm64-gnu',
      '@voidzero-dev/vite-plus-linux-arm64-musl',
      '@voidzero-dev/vite-plus-linux-x64-gnu',
      '@voidzero-dev/vite-plus-linux-x64-musl',
      '@voidzero-dev/vite-plus-win32-x64-msvc',
      '@voidzero-dev/vite-plus-win32-arm64-msvc',
    ]);
  });

  it('declares only generated Vite+ native packages as core optional dependencies', () => {
    const packageNames = getNativePlatformPackageNames(cliPkgJson.napi.targets);
    const nativeOptionalDependencyNames = Object.keys(corePkgJson.optionalDependencies).filter(
      (name) => name.startsWith('@voidzero-dev/vite-plus-'),
    );

    expect(nativeOptionalDependencyNames.toSorted()).toEqual(packageNames.toSorted());
    for (const packageName of packageNames) {
      expect(corePkgJson.optionalDependencies).toHaveProperty(packageName, corePkgJson.version);
    }

    expect(corePkgJson.peerDependencies).not.toHaveProperty('vite-plus');
  });

  it('rejects unsupported NAPI targets', () => {
    expect(() => getNativePlatformPackageName('wasm32-unknown-unknown')).toThrow(
      'Unsupported NAPI target architecture',
    );
  });
});
