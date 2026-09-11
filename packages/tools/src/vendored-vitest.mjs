import { existsSync, readFileSync, readdirSync, realpathSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

// This module runs before dependency installation and Node.js setup in CI.
// Keep it as plain JavaScript with only built-in imports.
// These official packages share the runner version selected by upgrade-deps.ts.
// Community packages, such as browser-webdriverio, have independent versions.
export const VITEST_EXACT_VERSION_PACKAGES = new Set([
  'vitest',
  '@vitest/browser',
  '@vitest/browser-playwright',
  '@vitest/browser-preview',
  '@vitest/coverage-v8',
  '@vitest/coverage-istanbul',
  '@vitest/mocker',
  '@vitest/pretty-format',
  '@vitest/snapshot',
  '@vitest/spy',
  '@vitest/ui',
  '@vitest/utils',
  '@vitest/web-worker',
]);
export const REMOVED_VITEST_PACKAGES = new Set(['@vitest/runner', '@vitest/expect']);

/**
 * @param {string} rootDir
 * @param {string} version
 */
export function alignVendoredVitestDependencies(rootDir, version) {
  for (const vendor of ['vite', 'rolldown']) {
    const packagesDir = join(rootDir, vendor, 'packages');
    const dirs = [
      join(rootDir, vendor),
      ...readdirSync(packagesDir, { withFileTypes: true })
        .filter((entry) => entry.isDirectory())
        .map((entry) => join(packagesDir, entry.name)),
    ];
    for (const dir of dirs) {
      const file = join(dir, 'package.json');
      if (!existsSync(file)) {
        continue;
      }
      const source = readFileSync(file, 'utf8');
      const pkg = JSON.parse(source);
      let changed = false;
      for (const field of ['dependencies', 'devDependencies', 'optionalDependencies']) {
        for (const name of Object.keys(pkg[field] ?? {})) {
          if (REMOVED_VITEST_PACKAGES.has(name)) {
            throw new Error(`Migrate removed ${name} use in ${file} before synchronizing Vitest`);
          }
          if (!VITEST_EXACT_VERSION_PACKAGES.has(name)) {
            continue;
          }
          // Only the default catalog is aligned during sync; named catalogs can still use v4.
          if (pkg[field][name] !== version && pkg[field][name] !== 'catalog:') {
            pkg[field][name] = version;
            changed = true;
          }
        }
      }
      if (changed) {
        writeFileSync(file, `${JSON.stringify(pkg, null, 2)}\n`);
      }
    }
  }
}

if (
  process.argv[1] &&
  existsSync(process.argv[1]) &&
  fileURLToPath(import.meta.url) === realpathSync(process.argv[1])
) {
  const rootDir = process.cwd();
  // upgrade-deps.ts keeps this exact runtime pin in sync with the root catalog.
  // Read the declaration without loading TypeScript or an installed YAML parser.
  const constants = readFileSync(join(rootDir, 'packages/cli/src/utils/constants.ts'), 'utf8');
  const version = constants.match(/^export const VITEST_VERSION = '(5\.\d+\.\d+)';\r?$/m)?.[1];
  if (!version) {
    throw new Error('VITEST_VERSION must declare an exact stable v5 version');
  }
  alignVendoredVitestDependencies(rootDir, version);
}
