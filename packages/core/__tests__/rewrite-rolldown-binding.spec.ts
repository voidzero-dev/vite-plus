import fs from 'node:fs';
import path from 'node:path';
import url from 'node:url';

import { describe, expect, it } from 'vitest';

import cliPkgJson from '../../cli/package.json' with { type: 'json' };
import { rewriteRolldownBindingRequires } from '../build-support/rewrite-rolldown-binding.ts';
import corePkgJson from '../package.json' with { type: 'json' };

const fixturePath = path.join(
  path.dirname(url.fileURLToPath(import.meta.url)),
  'fixtures',
  'rolldown-binding-loader.txt',
);
const loaderFixture = fs.readFileSync(fixturePath, 'utf-8');

const PACKAGE_NAME = '@voidzero-dev/vite-plus';

// The suffixes the fixture's supported branches use; must stay a subset of the
// CLI's napi.targets-derived set the release build passes in.
const platformSuffixes = new Set([
  'darwin-arm64',
  'darwin-x64',
  'linux-arm64-gnu',
  'linux-arm64-musl',
  'linux-x64-gnu',
  'linux-x64-musl',
  'win32-arm64-msvc',
  'win32-x64-msvc',
]);

const VERSION = '9.9.9';

describe('rewriteRolldownBindingRequires', () => {
  const result = rewriteRolldownBindingRequires(loaderFixture, {
    packageName: PACKAGE_NAME,
    platformSuffixes,
    version: VERSION,
  });

  it('rewrites supported platform branches to Vite+ platform packages', () => {
    // Fixture contains three supported branches: darwin-x64, darwin-arm64,
    // linux-x64-musl. Each requires the package and its package.json.
    for (const suffix of ['darwin-x64', 'darwin-arm64', 'linux-x64-musl']) {
      expect(result.source).toContain(`__require("@voidzero-dev/vite-plus-${suffix}")`);
      expect(result.source).toContain(
        `__require("@voidzero-dev/vite-plus-${suffix}/package.json")`,
      );
      expect(result.source).not.toContain(`@rolldown/binding-${suffix}`);
    }
    expect(result.rewrittenSuffixes).toEqual(
      new Set(['darwin-x64', 'darwin-arm64', 'linux-x64-musl']),
    );
    expect(result.specifierRewrites).toBe(6);
  });

  it('rewrites version guards only for rewritten branches', () => {
    expect(result.guardRewrites).toBe(3);
    const guardMatches = result.source.match(/bindingPackageVersion !== "9\.9\.9"/g);
    expect(guardMatches).toHaveLength(3);
    const messageMatches = result.source.match(/expected 9\.9\.9 but got/g);
    expect(messageMatches).toHaveLength(3);
  });

  it('leaves unsupported platform branches on the Rolldown packages and version', () => {
    expect(result.source).toContain('__require("@rolldown/binding-freebsd-x64")');
    expect(result.source).toContain('__require("@rolldown/binding-freebsd-x64/package.json")');
    // The freebsd guard still compares against the Rolldown version.
    const freebsdGuard = result.source.slice(
      result.source.indexOf('@rolldown/binding-freebsd-x64/package.json'),
    );
    expect(freebsdGuard).toContain('bindingPackageVersion !== "1.2.1"');
    expect(freebsdGuard).toContain('expected 1.2.1 but got');
  });

  it('leaves the WASI and WebContainer fallbacks untouched', () => {
    expect(result.source).toContain('__napiWasiResolveCandidate("@rolldown/binding-wasm32-wasi"');
    expect(result.source).toContain('__require("@rolldown/binding-wasm32-wasi/package.json")');
    expect(result.source).toContain(
      '`${baseDir}/node_modules/@rolldown/binding-wasm32-wasi/rolldown-binding.wasi.cjs`',
    );
    expect(result.source).toContain('`@rolldown/binding-wasm32-wasi@${version}`');
  });

  it('leaves version strings outside rewritten guards untouched', () => {
    // Rolldown's public version stays its own; only guard comparisons of
    // rewritten branches change.
    const source = `const VERSION = "1.2.1";\nexport { VERSION };\n`;
    const rewritten = rewriteRolldownBindingRequires(source, {
      packageName: PACKAGE_NAME,
      platformSuffixes,
      version: VERSION,
    });
    expect(rewritten.source).toBe(source);
    expect(rewritten.specifierRewrites).toBe(0);
    expect(rewritten.guardRewrites).toBe(0);
  });

  it('handles single-quoted loader output', () => {
    const source = [
      `const binding = require('@rolldown/binding-darwin-arm64');`,
      `const bindingPackageVersion = require('@rolldown/binding-darwin-arm64/package.json').version;`,
      `if (bindingPackageVersion !== '1.2.1' && process.env.NAPI_RS_ENFORCE_VERSION_CHECK && process.env.NAPI_RS_ENFORCE_VERSION_CHECK !== '0') throw new Error(\`Native binding package version mismatch, expected 1.2.1 but got \${bindingPackageVersion}. You can reinstall dependencies to fix this issue.\`);`,
    ].join('\n');
    const rewritten = rewriteRolldownBindingRequires(source, {
      packageName: PACKAGE_NAME,
      platformSuffixes,
      version: VERSION,
    });
    expect(rewritten.source).toContain(`require('@voidzero-dev/vite-plus-darwin-arm64')`);
    expect(rewritten.source).toContain(`bindingPackageVersion !== '9.9.9'`);
    expect(rewritten.source).toContain('expected 9.9.9 but got');
    expect(rewritten.specifierRewrites).toBe(2);
    expect(rewritten.guardRewrites).toBe(1);
  });

  it('is stable when applied twice', () => {
    const second = rewriteRolldownBindingRequires(result.source, {
      packageName: PACKAGE_NAME,
      platformSuffixes,
      version: VERSION,
    });
    expect(second.source).toBe(result.source);
    expect(second.specifierRewrites).toBe(0);
  });

  it('covers every CLI napi target with a platform suffix', async () => {
    const { parseTriple } = await import('@napi-rs/cli');
    expect(cliPkgJson.napi.packageName).toBe(PACKAGE_NAME);
    const suffixes = cliPkgJson.napi.targets.map((target) => parseTriple(target).platformArchABI);
    expect(new Set(suffixes)).toEqual(platformSuffixes);
  });

  it('keeps the committed core package.json free of platform pins', () => {
    // Platform pins are injected at publish time
    // (packages/cli/publish-native-addons.ts), never committed.
    const committedNativePins = Object.keys(corePkgJson.optionalDependencies).filter((name) =>
      name.startsWith(`${PACKAGE_NAME}-`),
    );
    expect(committedNativePins).toEqual([]);
  });
});
