/**
 * Verify that the vite-plus/versions export works correctly.
 *
 * Tests run against the already-built dist/ directory, ensuring
 * that syncVersionsExport() produces correct artifacts.
 */
import fs from 'node:fs';
import path from 'node:path';
import url from 'node:url';

import { describe, expect, it } from '@voidzero-dev/vite-plus-test';

const cliPkgDir = path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), '../..');
const distDir = path.join(cliPkgDir, 'dist');
const corePkgPath = path.join(cliPkgDir, '../core/package.json');
const testPkgPath = path.join(cliPkgDir, '../test/package.json');

describe('versions export', () => {
  describe('build artifacts', () => {
    it('dist/versions.js should exist', () => {
      expect(fs.existsSync(path.join(distDir, 'versions.js'))).toBe(true);
    });

    it('dist/versions.d.ts should exist', () => {
      expect(fs.existsSync(path.join(distDir, 'versions.d.ts'))).toBe(true);
    });

    it('dist/versions.js should export a versions object', () => {
      const content = fs.readFileSync(path.join(distDir, 'versions.js'), 'utf-8');
      expect(content).toContain('export const versions');
    });

    it('dist/versions.d.ts should declare a versions type', () => {
      const content = fs.readFileSync(path.join(distDir, 'versions.d.ts'), 'utf-8');
      expect(content).toContain('export declare const versions');
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

    it('should contain all test bundledVersions', async () => {
      const testPkg = JSON.parse(fs.readFileSync(testPkgPath, 'utf-8'));
      const mod = await import('../../dist/versions.js');
      const versions = mod.versions as Record<string, string>;
      for (const [key, value] of Object.entries(
        testPkg.bundledVersions as Record<string, string>,
      )) {
        expect(versions[key], `versions.${key} should match test bundledVersions`).toBe(value);
      }
    });
  });

  describe('dependency tool versions', () => {
    it('should contain oxlint version', async () => {
      const mod = await import('../../dist/versions.js');
      const versions = mod.versions as Record<string, string>;
      expect(versions.oxlint).toBeTypeOf('string');
    });

    it('should contain oxfmt version', async () => {
      const mod = await import('../../dist/versions.js');
      const versions = mod.versions as Record<string, string>;
      expect(versions.oxfmt).toBeTypeOf('string');
    });

    it('should contain oxlint-tsgolint version', async () => {
      const mod = await import('../../dist/versions.js');
      const versions = mod.versions as Record<string, string>;
      expect(versions['oxlint-tsgolint']).toBeTypeOf('string');
    });
  });

  describe('type declarations', () => {
    it('should have type fields for all bundled tools', () => {
      const content = fs.readFileSync(path.join(distDir, 'versions.d.ts'), 'utf-8');
      const expectedKeys = [
        'vite',
        'rolldown',
        'tsdown',
        'vitest',
        'oxlint',
        'oxfmt',
        'oxlint-tsgolint',
      ];
      for (const key of expectedKeys) {
        expect(content).toContain(key);
      }
    });

    it('should declare all fields as readonly string', () => {
      const content = fs.readFileSync(path.join(distDir, 'versions.d.ts'), 'utf-8');
      const fieldMatches = content.match(/readonly [\w'-]+: string;/g);
      expect(fieldMatches).not.toBeNull();
      expect(fieldMatches!.length).toBeGreaterThanOrEqual(7);
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

    it('should have valid semver-like versions', async () => {
      const { versions } = await import('../../dist/versions.js');
      const semverPattern = /^\d+\.\d+\.\d+/;
      for (const [key, value] of Object.entries(versions as Record<string, string>)) {
        expect(value, `${key} should be a valid version`).toMatch(semverPattern);
      }
    });
  });
});
