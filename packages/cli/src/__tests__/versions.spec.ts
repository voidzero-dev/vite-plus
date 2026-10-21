/**
 * Verify that the vite-plus/versions export works correctly.
 *
 * Tests run against the already-built dist/ directory, ensuring
 * that syncToolchainExports() produces correct artifacts.
 */
import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import url from 'node:url';

import { globSync } from 'tinyglobby';
import { describe, expect, it } from 'vitest';

const cliPkgDir = path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), '../..');
const distDir = path.join(cliPkgDir, 'dist');
const corePkgPath = path.join(cliPkgDir, '../core/package.json');
const vitestPkgPath = createRequire(import.meta.url).resolve('vitest/package.json');

describe('versions export', () => {
  describe('build artifacts', () => {
    it('dist/versions.js should export a versions object', () => {
      const content = fs.readFileSync(path.join(distDir, 'versions.js'), 'utf-8');
      expect(content).toContain('export const versions');
    });

    it('dist/versions.d.ts should declare a versions type', () => {
      const content = fs.readFileSync(path.join(distDir, 'versions.d.ts'), 'utf-8');
      expect(content).toContain('export declare const versions');
    });
  });

  describe('bundled dynamic imports resolve', () => {
    // The ESLint migrator (`src/migration/migrator/eslint.ts`) lazily does
    // `import('../versions.js')`. The tsdown `fix-versions-path` plugin rewrites
    // that specifier to `./versions.js` (external) and the code lands in a
    // dist-ROOT shared chunk, so it must resolve to `dist/versions.js`. Guard
    // against a regression where a chunk emits a relative `versions.js` import
    // that does not resolve relative to that chunk (e.g. if the migrator code
    // were inlined into `dist/migration/bin.js`, `./versions.js` would point at
    // the non-existent `dist/migration/versions.js`).
    //
    // Static relative dynamic imports ending in `versions.js`, e.g.
    // `import("./versions.js")`. Template-literal forms (used by the `vp
    // version` command) are intentionally excluded — they resolve at runtime.
    const RELATIVE_VERSIONS_IMPORT = /import\(\s*(['"])(\.\.?\/[^'"]*versions\.js)\1\s*\)/g;

    it('every relative `import(... versions.js)` in dist resolves to an existing file', () => {
      const offenders: string[] = [];
      let matchCount = 0;
      for (const file of globSync('**/*.js', { cwd: distDir, absolute: true })) {
        const content = fs.readFileSync(file, 'utf-8');
        for (const match of content.matchAll(RELATIVE_VERSIONS_IMPORT)) {
          matchCount++;
          const target = path.resolve(path.dirname(file), match[2]);
          if (!fs.existsSync(target)) {
            offenders.push(
              `${path.relative(distDir, file)} imports "${match[2]}" → missing ${path.relative(distDir, target)}`,
            );
          }
        }
      }
      // Sanity: the ESLint-migrator import must actually be present in the bundle,
      // otherwise this guard would silently pass without checking anything.
      expect(
        matchCount,
        'expected at least one bundled versions.js dynamic import',
      ).toBeGreaterThan(0);
      expect(offenders, offenders.join('\n')).toEqual([]);
    });
  });

  describe('bundledVersions consistency', () => {
    it('should contain all core bundledVersions', async () => {
      const corePkg = JSON.parse(fs.readFileSync(corePkgPath, 'utf-8'));
      const mod = await import('../../dist/versions.js');
      const versions = mod.versions as Record<string, string>;
      for (const [key, value] of Object.entries(
        corePkg.bundledVersions as Record<string, string>,
      )) {
        expect(versions[key], `versions.${key} should match core bundledVersions`).toBe(value);
      }
    });

    it('should contain vitest version matching installed package', async () => {
      const vitestPkg = JSON.parse(fs.readFileSync(vitestPkgPath, 'utf-8'));
      const mod = await import('../../dist/versions.js');
      const versions = mod.versions as Record<string, string>;
      expect(versions.vitest, 'versions.vitest should match installed vitest version').toBe(
        vitestPkg.version,
      );
    });
  });

  describe('type declarations', () => {
    it('should declare every runtime field as a readonly string', async () => {
      const content = fs.readFileSync(path.join(distDir, 'versions.d.ts'), 'utf-8');
      const { versions } = await import('../../dist/versions.js');
      const fields = Array.from(
        content.matchAll(/readonly '([^']+)': string;/g),
        (match) => match[1],
      );
      expect(fields).toEqual(Object.keys(versions));
    });
  });

  describe('runtime import', () => {
    it('should be importable and return an object with expected keys', async () => {
      const { versions } = await import('../../dist/versions.js');
      expect(versions).toBeDefined();
      expect(typeof versions).toBe('object');
      expect(versions.vite).toBeTypeOf('string');
      expect(versions.rolldown).toBeTypeOf('string');
      expect(versions.tsdown).toBeTypeOf('string');
      expect(versions.vitest).toBeTypeOf('string');
      expect(versions.oxlint).toBeTypeOf('string');
      expect(versions.oxfmt).toBeTypeOf('string');
      expect(versions['oxlint-tsgolint']).toBeTypeOf('string');
    });
  });
});
