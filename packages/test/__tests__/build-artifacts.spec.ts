/**
 * Verify that the @voidzero-dev/vite-plus-test build output (dist/)
 * contains the expected files and that patches applied during the build
 * (in build.ts) produce correct artifacts.
 *
 * This is important because vite-plus re-packages vitest with custom
 * patches, and missing exports or incorrect patches can break
 * third-party integrations (e.g., @storybook/addon-vitest, #1086).
 */
import fs from 'node:fs';
import path from 'node:path';
import url from 'node:url';

import { describe, expect, it } from 'vitest';

const testPkgDir = path.resolve(path.dirname(url.fileURLToPath(import.meta.url)), '..');
const distDir = path.join(testPkgDir, 'dist');

describe('build artifacts', () => {
  describe('@vitest/browser/context.js', () => {
    const contextPath = path.join(distDir, '@vitest/browser/context.js');

    it('should exist', () => {
      expect(fs.existsSync(contextPath), `${contextPath} should exist`).toBe(true);
    });

    it('should export page, cdp, and utils', () => {
      const content = fs.readFileSync(contextPath, 'utf-8');
      expect(content).toMatch(/export\s*\{[^}]*page[^}]*\}/);
      expect(content).toMatch(/export\s*\{[^}]*cdp[^}]*\}/);
      expect(content).toMatch(/export\s*\{[^}]*utils[^}]*\}/);
    });
  });

  /**
   * The vitest:vendor-aliases plugin must NOT resolve @vitest/browser/context
   * to the static file. If it does, the BrowserContext plugin's virtual module
   * (which provides the `server` export) is bypassed.
   *
   * See: https://github.com/voidzero-dev/vite-plus/issues/1086
   */
  describe('vitest:vendor-aliases plugin (regression test for #1086)', () => {
    const browserIndexPath = path.join(distDir, '@vitest/browser/index.js');

    it('should not map @vitest/browser/context in vendorMap', () => {
      const content = fs.readFileSync(browserIndexPath, 'utf-8');
      // The vendorMap inside vitest:vendor-aliases should NOT contain
      // '@vitest/browser/context' — it must be left for BrowserContext
      // plugin to resolve as a virtual module.
      const vendorAliasesMatch = content.match(
        /name:\s*['"]vitest:vendor-aliases['"][\s\S]*?const vendorMap\s*=\s*\{([\s\S]*?)\}/,
      );
      expect(vendorAliasesMatch, 'vitest:vendor-aliases plugin should exist').toBeTruthy();
      const vendorMapContent = vendorAliasesMatch![1];
      expect(vendorMapContent).not.toContain("'@vitest/browser/context'");
    });
  });
});
