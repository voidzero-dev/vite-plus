import { existsSync, readFileSync, readdirSync, realpathSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

// This module runs before dependency installation in CI.
// Keep it independent of installed project dependencies, with only built-in imports.
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

function isPackageJson(value: unknown): value is {
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
  optionalDependencies?: Record<string, string>;
} {
  if (value === null || typeof value !== 'object') {
    return false;
  }
  const sections = [
    'dependencies' in value ? value.dependencies : undefined,
    'devDependencies' in value ? value.devDependencies : undefined,
    'optionalDependencies' in value ? value.optionalDependencies : undefined,
  ];
  return sections.every(
    (section) =>
      section === undefined ||
      (typeof section === 'object' &&
        section !== null &&
        !Array.isArray(section) &&
        Object.values(section).every((v: unknown) => typeof v === 'string')),
  );
}

export function alignVendoredVitestDependencies(rootDir: string, version: string): void {
  // Keep Vite's direct dependencies aligned with the root catalog so sync-remote
  // and CI use the same lockfile. Leave Rolldown's test dependencies to upstream.
  const packagesDir = join(rootDir, 'vite', 'packages');
  const dirs = [
    join(rootDir, 'vite'),
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
    const pkg: unknown = JSON.parse(source);
    if (!isPackageJson(pkg)) {
      throw new Error(`Invalid dependency declarations in ${file}`);
    }
    let changed = false;
    for (const field of ['dependencies', 'devDependencies', 'optionalDependencies'] as const) {
      const dependencies = pkg[field] ?? {};
      for (const [name, currentVersion] of Object.entries(dependencies)) {
        if (REMOVED_VITEST_PACKAGES.has(name)) {
          throw new Error(`Migrate removed ${name} use in ${file} before synchronizing Vitest`);
        }
        if (!VITEST_EXACT_VERSION_PACKAGES.has(name)) {
          continue;
        }
        // Only the default catalog is aligned during sync; named catalogs can still use v4.
        if (currentVersion !== version && currentVersion !== 'catalog:') {
          dependencies[name] = version;
          changed = true;
        }
      }
    }
    if (changed) {
      writeFileSync(file, `${JSON.stringify(pkg, null, 2)}\n`);
    }
  }
}

if (
  process.argv[1] &&
  existsSync(process.argv[1]) &&
  import.meta.filename === realpathSync(process.argv[1])
) {
  const rootDir = process.cwd();
  // upgrade-deps.ts keeps this exact runtime pin in sync with the root catalog.
  // Read the declaration without importing CLI code or an installed YAML parser.
  const constants = readFileSync(join(rootDir, 'packages/cli/src/utils/constants.ts'), 'utf8');
  const version = constants.match(/^export const VITEST_VERSION = '(5\.\d+\.\d+)';\r?$/m)?.[1];
  if (!version) {
    throw new Error('VITEST_VERSION must declare an exact stable v5 version');
  }
  alignVendoredVitestDependencies(rootDir, version);
}
