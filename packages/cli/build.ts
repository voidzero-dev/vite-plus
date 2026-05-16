/**
 * Build script for vite-plus CLI package
 *
 * This script performs the following main tasks:
 * 1. buildWithTsdown() - Bundles all CLI entry points via tsdown
 * 2. buildNapiBinding() - Builds the native Rust binding via NAPI
 * 3. syncCorePackageExports() - Creates shim files to re-export from @voidzero-dev/vite-plus-core
 * 4. syncTestPackageExports() - Creates shim files to re-export from vitest
 * 5. syncVersionsExport() - Generates ./versions module with bundled tool versions
 * 6. copyBundledDocs() - Copies docs into docs/ for bundled package access
 * 7. syncReadmeFromRoot() - Keeps package README in sync
 *
 * The sync functions allow this package to be a drop-in replacement for 'vite' by
 * re-exporting all the same subpaths (./client, ./types/*, etc.) while delegating
 * to the core package for actual implementation.
 *
 * IMPORTANT: The core package must be built before running this script.
 * Native binding is built first because TypeScript may depend on generated binding types.
 */

import { execSync } from 'node:child_process';
import { existsSync, readdirSync, statSync } from 'node:fs';
import { copyFile, cp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';

import { createBuildCommand, NapiCli } from '@napi-rs/cli';
import { format } from 'oxfmt';

import { generateLicenseFile } from '../../scripts/generate-license.js';
import corePkg from '../core/package.json' with { type: 'json' };

const projectDir = dirname(fileURLToPath(import.meta.url));
const TEST_PACKAGE_NAME = 'vitest';
const CORE_PACKAGE_NAME = '@voidzero-dev/vite-plus-core';

// Browser providers projected under ./test/* and ./test/browser/providers/* so the
// public surface matches what the deleted `@voidzero-dev/vite-plus-test` wrapper exposed.
// Each entry maps the upstream package name to the short provider name used in the
// `./test/browser/providers/<short>` alias path.
const BROWSER_PROVIDER_PACKAGES: ReadonlyArray<{ pkg: string; short: string }> = [
  { pkg: '@vitest/browser-playwright', short: 'playwright' },
  { pkg: '@vitest/browser-preview', short: 'preview' },
  { pkg: '@vitest/browser-webdriverio', short: 'webdriverio' },
];

/**
 * Vitest-related bare specifiers that appear in `@vitest/browser-*` d.ts files
 * and the sub-path under `dist/test/` whose shim re-exports the same module.
 * Longer prefixes are listed first so substring matches don't shadow them
 * (e.g. `vitest/internal/browser` before `vitest/browser`).
 */
const VITEST_TYPE_SPECIFIER_REWRITES: ReadonlyArray<readonly [string, string]> = [
  ['@vitest/browser/context', '_at-vitest-browser/context'],
  ['@vitest/browser', '_at-vitest-browser'],
  ['vitest/internal/browser', 'internal/browser'],
  ['vitest/browser', 'browser'],
  ['vitest/node', 'node'],
];

const {
  values: { ['skip-native']: skipNative, ['skip-ts']: skipTs },
} = parseArgs({
  options: {
    ['skip-native']: { type: 'boolean', default: false },
    ['skip-ts']: { type: 'boolean', default: false },
  },
  strict: false,
});

// Filter out custom flags before passing to NAPI CLI
const napiArgs = process.argv
  .slice(2)
  .filter((arg) => arg !== '--skip-native' && arg !== '--skip-ts');

if (!skipTs) {
  buildWithTsdown();
  generateLicenseFile({
    title: 'Vite-Plus CLI license',
    packageName: 'Vite-Plus',
    outputPath: join(projectDir, 'LICENSE'),
    coreLicensePath: join(projectDir, '..', '..', 'LICENSE'),
    bundledPaths: [join(projectDir, 'dist')],
    resolveFrom: [projectDir],
  });
  if (!existsSync(join(projectDir, 'LICENSE'))) {
    throw new Error('LICENSE was not generated during build');
  }
}
// Build native first - TypeScript may depend on the generated binding types
if (!skipNative) {
  await buildNapiBinding();
}
if (!skipTs) {
  await syncCorePackageExports();
  await syncTestPackageExports();
  await syncVersionsExport();
}
await copyBundledDocs();
await syncReadmeFromRoot();

async function buildNapiBinding() {
  const buildCommand = createBuildCommand(napiArgs);
  const passedInOptions = buildCommand.getOptions();

  const cli = new NapiCli();

  const dtsHeader = process.env.RELEASE_BUILD
    ? (await import('../../rolldown/packages/rolldown/package.json', { with: { type: 'json' } }))
        .default.napi.dtsHeader
    : '';

  if (dtsHeader) {
    passedInOptions.dtsHeader = `type BindingErrorsOr<T> = T | BindingErrors;\ntype FxHashSet<T> = Set<T>;\ntype FxHashMap<K, V> = Map<K, V>;\n${dtsHeader}`;
  }

  const { task } = await cli.build({
    ...passedInOptions,
    packageJsonPath: '../package.json',
    cwd: 'binding',
    platform: true,
    jsBinding: 'index.cjs',
    dts: 'index.d.cts',
    release: process.env.VP_CLI_DEBUG !== '1',
    features: process.env.RELEASE_BUILD ? ['rolldown'] : void 0,
  });

  const outputs = await task;
  const viteConfig = await import('../../vite.config.js');
  for (const output of outputs) {
    if (output.kind !== 'node') {
      const { code, errors } = await format(output.path, await readFile(output.path, 'utf8'), {
        ...viteConfig.default.fmt,
        embeddedCode: true,
      });
      if (errors.length > 0) {
        for (const error of errors) {
          console.error(error);
        }
        process.exit(1);
      }
      await writeFile(output.path, code);
    }
  }
}

function buildWithTsdown() {
  execSync('npx tsdown', {
    cwd: projectDir,
    stdio: 'inherit',
  });
}

/**
 * Sync Vite core exports from @voidzero-dev/vite-plus-core to vite-plus
 *
 * Creates shim files that re-export from the core package, enabling imports like:
 * - `import type { ... } from 'vite-plus/types/importGlob.d.ts'`
 * - `import { ... } from 'vite-plus/module-runner'`
 *
 * Export paths created:
 * - ./client - Triple-slash reference (ambient type declarations for CSS, assets, etc.)
 * - ./module-runner - Re-exports both JS and types
 * - ./internal - Re-exports both JS and types
 * - ./dist/client/* - Re-exports client runtime files (.mjs, .cjs)
 * - ./types/* - Type-only re-exports using `export type *`
 *
 * Note: In package.json exports, ./types/internal/* must come BEFORE ./types/*
 * for correct precedence (more specific patterns must precede wildcards).
 *
 * @throws Error if core package is not built (missing dist directories)
 */
async function syncCorePackageExports() {
  console.log('\nSyncing core package exports...');

  const distDir = join(projectDir, 'dist');
  const clientDir = join(distDir, 'client');
  const typesDir = join(distDir, 'types');

  // Clean up previous build
  await rm(clientDir, { recursive: true, force: true });
  await rm(typesDir, { recursive: true, force: true });
  await mkdir(clientDir, { recursive: true });
  await mkdir(typesDir, { recursive: true });

  // Create ./client shim (types only) - uses triple-slash reference since client.d.ts is ambient
  console.log('  Creating ./client');
  await writeFile(
    join(distDir, 'client.d.ts'),
    `/// <reference types="${CORE_PACKAGE_NAME}/client" />\n`,
  );

  // Create ./pack/client shim (types only) - ambient type declarations for tsdown bundler features
  console.log('  Creating ./pack/client');
  await writeFile(
    join(distDir, 'pack-client.d.ts'),
    `/// <reference types="${CORE_PACKAGE_NAME}/pack/client" />\n`,
  );

  // Create ./module-runner shim
  console.log('  Creating ./module-runner');
  await writeFile(
    join(distDir, 'module-runner.js'),
    `export * from '${CORE_PACKAGE_NAME}/module-runner';\n`,
  );
  await writeFile(
    join(distDir, 'module-runner.d.ts'),
    `export * from '${CORE_PACKAGE_NAME}/module-runner';\n`,
  );

  // Create ./internal shim
  console.log('  Creating ./internal');
  await writeFile(join(distDir, 'internal.js'), `export * from '${CORE_PACKAGE_NAME}/internal';\n`);
  await writeFile(
    join(distDir, 'internal.d.ts'),
    `export * from '${CORE_PACKAGE_NAME}/internal';\n`,
  );

  // Create ./dist/client/* shims by reading core's dist/vite/client files
  console.log('  Creating ./dist/client/*');
  const coreClientDir = join(projectDir, '../core/dist/vite/client');
  if (!existsSync(coreClientDir)) {
    throw new Error(
      `Core client artifacts not found at "${coreClientDir}". ` +
        `Make sure ${CORE_PACKAGE_NAME} is built before building the CLI.`,
    );
  }
  for (const file of readdirSync(coreClientDir)) {
    const srcPath = join(coreClientDir, file);
    const shimPath = join(clientDir, file);
    // Skip directories
    if (statSync(srcPath).isDirectory()) {
      continue;
    }
    if (file.endsWith('.js') || file.endsWith('.mjs') || file.endsWith('.cjs')) {
      await writeFile(shimPath, `export * from '${CORE_PACKAGE_NAME}/dist/client/${file}';\n`);
    } else if (file.endsWith('.d.ts') || file.endsWith('.d.mts') || file.endsWith('.d.cts')) {
      const baseFile = file.replace(/\.d\.[mc]?ts$/, '');
      await writeFile(shimPath, `export * from '${CORE_PACKAGE_NAME}/dist/client/${baseFile}';\n`);
    } else {
      // Copy non-JS/TS files directly (e.g., CSS, source maps)
      await copyFile(srcPath, shimPath);
    }
  }

  // Create ./types/* shims by reading core's dist/vite/types files
  console.log('  Creating ./types/*');
  const coreTypesDir = join(projectDir, '../core/dist/vite/types');
  if (!existsSync(coreTypesDir)) {
    throw new Error(
      `Core type definitions not found at "${coreTypesDir}". ` +
        `Make sure ${CORE_PACKAGE_NAME} is built before building the CLI.`,
    );
  }
  await syncTypesDir(coreTypesDir, typesDir, '');

  console.log('\nSynced core package exports');
}

/**
 * Recursively sync type definition files from core to CLI package
 *
 * Creates shim .d.ts files that re-export types from the core package.
 * Uses `export type * from` syntax which is valid in TypeScript 5.0+.
 *
 * @param srcDir - Source directory containing .d.ts files
 * @param destDir - Destination directory for shim files
 * @param relativePath - Current path relative to types root (empty string at top level)
 *
 * Special handling:
 * - Skips top-level 'internal' directory (blocked by ./types/internal/* export)
 * - Supports .d.ts, .d.mts, and .d.cts extensions
 * - Preserves directory structure recursively
 */
async function syncTypesDir(srcDir: string, destDir: string, relativePath: string) {
  const entries = readdirSync(srcDir);

  for (const entry of entries) {
    const srcPath = join(srcDir, entry);
    const destPath = join(destDir, entry);
    const entryRelPath = relativePath ? `${relativePath}/${entry}` : entry;

    if (statSync(srcPath).isDirectory()) {
      // Skip top-level internal directory - it's blocked by ./types/internal/* export
      if (entry === 'internal' && relativePath === '') {
        continue;
      }

      await mkdir(destPath, { recursive: true });
      await syncTypesDir(srcPath, destPath, entryRelPath);
    } else if (/\.d\.[mc]?ts$/.test(entry)) {
      // Create shim that re-exports from core - must include extension for wildcard exports
      // Use 'export type *' since we're re-exporting from a .d.ts file
      await writeFile(
        destPath,
        `export type * from '${CORE_PACKAGE_NAME}/types/${entryRelPath}';\n`,
      );
    }
  }
}

/**
 * Sync exports from vitest to vite-plus
 *
 * This function reads vitest's package.json exports and creates shim files that
 * re-export everything under the ./test/* subpath. This allows users to import
 * from vite-plus/test/* instead of vitest/*.
 */
async function syncTestPackageExports() {
  console.log('\nSyncing test package exports...');

  // Resolve vitest's package.json via Node's resolver so we always read the
  // currently installed copy — packages/test/ no longer exists.
  const require = createRequire(import.meta.url);
  const testPkgPath = require.resolve(`${TEST_PACKAGE_NAME}/package.json`, {
    paths: [projectDir],
  });
  const cliPkgPath = join(projectDir, 'package.json');
  const testDistDir = join(projectDir, 'dist/test');

  // Read test package.json
  const testPkg = JSON.parse(await readFile(testPkgPath, 'utf-8'));
  const testExports = testPkg.exports as Record<string, unknown>;

  // Clean up previous build
  await rm(testDistDir, { recursive: true, force: true });
  await mkdir(testDistDir, { recursive: true });

  const generatedExports: Record<string, unknown> = {};

  for (const [exportPath, exportValue] of Object.entries(testExports)) {
    // Skip package.json export and wildcard exports
    if (exportPath === './package.json' || exportPath.includes('*')) {
      continue;
    }

    // Convert ./foo to ./test/foo, . to ./test
    const cliExportPath = exportPath === '.' ? './test' : `./test${exportPath.slice(1)}`;
    const shimBaseName = exportPath === '.' ? 'index' : exportPath.slice(2);
    const importSpecifier =
      exportPath === '.' ? TEST_PACKAGE_NAME : `${TEST_PACKAGE_NAME}${exportPath.slice(1)}`;

    // Create shim files and build export entry
    const shimExport = await createShimForExport(
      shimBaseName,
      exportValue,
      importSpecifier,
      testDistDir,
    );
    if (shimExport) {
      generatedExports[cliExportPath] = shimExport;
      console.log(`  Created ${cliExportPath}`);
    }
  }

  // Private shims for `@vitest/browser` and `@vitest/browser/context`. These
  // are referenced as relative paths from the inlined browser-provider d.ts
  // shims so that `@vitest/browser` resolves through vite-plus's own pnpm-edge
  // (same one that owns the `vitest` direct dep) — preventing the two-vitest
  // type-identity split that breaks user `provider: playwright()` typechecks.
  await writePrivateAtVitestBrowserShims(testDistDir);

  // Mirror upstream @vitest/browser-* provider packages under ./test/<provider> and
  // ./test/browser/providers/<short>. Existing vite-plus user code imports from these
  // paths (e.g., `vite-plus/test/browser-playwright`) and must keep resolving after
  // the bundled `@voidzero-dev/vite-plus-test` wrapper was removed.
  for (const { pkg, short } of BROWSER_PROVIDER_PACKAGES) {
    let providerPkgPath: string;
    try {
      providerPkgPath = require.resolve(`${pkg}/package.json`, { paths: [projectDir] });
    } catch (err) {
      console.warn(`  Skipping ${pkg} — not installed: ${(err as Error).message}`);
      continue;
    }
    const providerPkg = JSON.parse(await readFile(providerPkgPath, 'utf-8'));
    const providerPkgRoot = dirname(providerPkgPath);
    const providerExports = (providerPkg.exports ?? {}) as Record<string, unknown>;

    for (const [providerExportPath, providerExportValue] of Object.entries(providerExports)) {
      if (providerExportPath === './package.json' || providerExportPath.includes('*')) {
        continue;
      }

      const providerSubPath = providerExportPath === '.' ? '' : providerExportPath.slice(1);
      // Two CLI surfaces that map to the same provider shim:
      //   ./test/<pkgShortName>           → e.g. ./test/browser-playwright
      //   ./test/browser/providers/<short> → e.g. ./test/browser/providers/playwright
      const pkgShortName = pkg.startsWith('@vitest/') ? pkg.slice('@vitest/'.length) : pkg;
      const surfaces = [
        {
          cliPath: `./test/${pkgShortName}${providerSubPath}`,
          baseName: `${pkgShortName}${providerSubPath}`,
        },
        {
          cliPath: `./test/browser/providers/${short}${providerSubPath}`,
          baseName: `browser/providers/${short}${providerSubPath}`,
        },
      ];
      const importSpecifier =
        providerExportPath === '.' ? pkg : `${pkg}${providerExportPath.slice(1)}`;

      for (const { cliPath, baseName } of surfaces) {
        const shimBaseName = baseName.replace(/^\//, '');
        const shimExport = await createShimForExport(
          shimBaseName,
          providerExportValue,
          importSpecifier,
          testDistDir,
          { providerPkgRoot },
        );
        if (shimExport) {
          generatedExports[cliPath] = shimExport;
          console.log(`  Created ${cliPath}`);
        }
      }
    }
  }

  // Update CLI package.json
  await updateCliPackageJson(cliPkgPath, generatedExports);

  console.log(`\nSynced ${Object.keys(generatedExports).length} exports from test package`);
}

/**
 * Read version from a dependency's package.json in node_modules.
 * Uses readFile because these packages don't export ./package.json.
 *
 * TODO: Once https://github.com/oxc-project/oxc/pull/20784 lands and oxlint/oxfmt/oxlint-tsgolint
 * export ./package.json, this function can be removed and replaced with static imports:
 * ```js
 * import oxlintPkg from 'oxlint/package.json' with { type: 'json' };
 * import oxfmtPkg from 'oxfmt/package.json' with { type: 'json' };
 * import oxlintTsgolintPkg from 'oxlint-tsgolint/package.json' with { type: 'json' };
 * ```
 */
async function readDepVersion(packageName: string): Promise<string | null> {
  try {
    const pkgPath = join(projectDir, 'node_modules', packageName, 'package.json');
    const pkg = JSON.parse(await readFile(pkgPath, 'utf-8'));
    return pkg.version ?? null;
  } catch {
    return null;
  }
}

/**
 * Generate ./versions export module with bundled tool versions.
 *
 * Collects versions from:
 * - core package.json bundledVersions (vite, rolldown, tsdown)
 * - CLI dependency package.json (oxlint, oxfmt, oxlint-tsgolint, vitest)
 *
 * Generates dist/versions.js and dist/versions.d.ts with inlined constants.
 */
async function syncVersionsExport() {
  console.log('\nSyncing versions export...');
  const distDir = join(projectDir, 'dist');

  // Collect bundled versions from the core package
  const versions: Record<string, string> = {
    ...(corePkg as Record<string, any>).bundledVersions,
  };

  // Read versions from CLI dependencies' installed package.json files
  // (these packages don't export ./package.json, so node_modules is the source of truth)
  const depTools = ['oxlint', 'oxfmt', 'oxlint-tsgolint', 'vitest'] as const;
  for (const name of depTools) {
    const version = await readDepVersion(name);
    if (version) {
      versions[name] = version;
    }
  }

  // dist/versions.js — inlined constants (no runtime I/O)
  await writeFile(
    join(distDir, 'versions.js'),
    `export const versions = ${JSON.stringify(versions, null, 2)};\n`,
  );

  // dist/versions.d.ts — type declarations
  const typeFields = Object.keys(versions)
    .map((k) => `  readonly '${k}': string;`)
    .join('\n');
  await writeFile(
    join(distDir, 'versions.d.ts'),
    `export declare const versions: {\n${typeFields}\n};\n`,
  );

  console.log(`  Created ./versions (${Object.keys(versions).length} tools)`);
}

/**
 * Copy the docs source tree into docs/, preserving relative paths.
 * Generated VitePress output and installed dependencies are excluded so the package
 * only ships authoring sources and referenced assets.
 */
async function copyBundledDocs() {
  console.log('\nCopying bundled docs...');

  const docsSourceDir = join(projectDir, '..', '..', 'docs');
  const docsTargetDir = join(projectDir, 'docs');

  if (!existsSync(docsSourceDir)) {
    console.log('  Docs source directory not found, skipping docs copy');
    return;
  }

  const skipPrefixes = ['node_modules', '.vitepress/cache', '.vitepress/dist'];
  await rm(docsTargetDir, { recursive: true, force: true });
  await cp(docsSourceDir, docsTargetDir, {
    recursive: true,
    filter: (src) => {
      const rel = relative(docsSourceDir, src).replaceAll('\\', '/');
      return !skipPrefixes.some((prefix) => rel === prefix || rel.startsWith(`${prefix}/`));
    },
  });

  console.log('  Copied docs to docs/ (with paths preserved)');
}

async function syncReadmeFromRoot() {
  const rootReadmePath = join(projectDir, '..', '..', 'README.md');
  const packageReadmePath = join(projectDir, 'README.md');
  const [rootReadme, packageReadme] = await Promise.all([
    readFile(rootReadmePath, 'utf8'),
    readFile(packageReadmePath, 'utf8'),
  ]);

  const { suffix: rootSuffix } = splitReadme(rootReadme, rootReadmePath);
  const { prefix: packagePrefix } = splitReadme(packageReadme, packageReadmePath);
  const nextReadme = `${packagePrefix}\n\n${rootSuffix}\n`;

  if (nextReadme !== packageReadme) {
    await writeFile(packageReadmePath, nextReadme);
  }
}

function splitReadme(content: string, label: string) {
  const match = /^---\s*$/m.exec(content);
  if (!match || match.index === undefined) {
    throw new Error(`Expected ${label} to include a '---' separator.`);
  }

  const delimiterStart = match.index;
  const delimiterEnd = delimiterStart + match[0].length;
  const afterDelimiter = content.slice(delimiterEnd);
  const newlineMatch = /^\r?\n/.exec(afterDelimiter);
  const delimiterWithNewlineEnd = delimiterEnd + (newlineMatch ? newlineMatch[0].length : 0);

  return {
    prefix: content.slice(0, delimiterWithNewlineEnd).trim(),
    suffix: content.slice(delimiterWithNewlineEnd).trim(),
  };
}

type ExportValue =
  | string
  | {
      types?: string;
      default?: string;
      import?: ExportValue;
      require?: ExportValue;
      node?: string;
    };

/**
 * Write private shims at `dist/test/_at-vitest-browser{.d.ts,/context.d.ts}`
 * that re-export the `@vitest/browser` package. These are referenced by the
 * inlined browser-provider d.ts shims via relative paths so all of
 * `@vitest/browser`, `vitest/node`, etc. resolve through vite-plus's own
 * pnpm-edge — the same edge that owns vite-plus's `vitest` direct dep.
 * The underscore prefix marks them as private; they are not surfaced in the
 * package.json `exports` map (TS resolves the relative paths directly).
 */
async function writePrivateAtVitestBrowserShims(testDistDir: string): Promise<void> {
  await mkdir(join(testDistDir, '_at-vitest-browser'), { recursive: true });
  await writeFile(
    join(testDistDir, '_at-vitest-browser.d.ts'),
    `import '@vitest/browser';\nexport * from '@vitest/browser';\n`,
  );
  await writeFile(
    join(testDistDir, '_at-vitest-browser/context.d.ts'),
    `import '@vitest/browser/context';\nexport * from '@vitest/browser/context';\n`,
  );
}

/**
 * Inline-copy a browser-provider's upstream `.d.ts` file into `outDtsPath` and
 * rewrite vitest-related bare specifiers to relative paths inside the
 * vite-plus test shim tree. See `VITEST_TYPE_SPECIFIER_REWRITES` and
 * `writePrivateAtVitestBrowserShims` for the rationale.
 *
 * Specifiers that are user peer dependencies (`playwright`, `webdriverio`,
 * `tinyrainbow`, etc.) and the `@vitest/browser-*` self-import are left bare.
 */
async function writeInlinedProviderDts(
  outDtsPath: string,
  upstreamDtsPath: string,
  testDistDir: string,
): Promise<void> {
  const upstream = await readFile(upstreamDtsPath, 'utf-8');
  const outDir = dirname(outDtsPath);
  // Resolve to the file basename appended to the relative dir, never to the
  // bare directory. `relative('dist/test/x/', 'dist/test/y')` returns `'../y'`
  // but `relative('dist/test/x/', 'dist/test/y/')` returns `'..'` — TS would
  // then look for `dist/test/y/index.d.ts` instead of `dist/test/y.d.ts`. We
  // always emit `<relDir>/<basename>` so the basename lookup hits the file.
  const relToShim = (sub: string): string => {
    const r = relative(outDir, testDistDir).replaceAll('\\', '/');
    const prefix = r === '' ? '.' : r.startsWith('.') ? r : `./${r}`;
    return `${prefix}/${sub}`;
  };
  let result = upstream;
  for (const [bare, sub] of VITEST_TYPE_SPECIFIER_REWRITES) {
    const escaped = bare.replaceAll('/', '\\/');
    const pattern = new RegExp(`(['"])${escaped}\\1`, 'g');
    result = result.replaceAll(pattern, `'${relToShim(sub)}'`);
  }
  await mkdir(outDir, { recursive: true });
  await writeFile(outDtsPath, result);
}

/**
 * Resolve the upstream `.d.ts` path for a given export value. Returns null
 * when the export does not declare a types file (runtime-only exports).
 */
function resolveUpstreamDtsPath(
  providerPkgRoot: string,
  exportValue: unknown,
  condition: 'types' | 'require-types' = 'types',
): string | null {
  if (typeof exportValue === 'string') {
    return exportValue.endsWith('.d.ts') || exportValue.endsWith('.d.cts')
      ? join(providerPkgRoot, exportValue)
      : null;
  }
  if (typeof exportValue !== 'object' || exportValue === null) {
    return null;
  }
  const value = exportValue as Record<string, unknown>;
  if (condition === 'types') {
    if (typeof value.types === 'string') {
      return join(providerPkgRoot, value.types);
    }
    if (typeof value.import === 'object' && value.import !== null) {
      const types = (value.import as Record<string, unknown>).types;
      if (typeof types === 'string') {
        return join(providerPkgRoot, types);
      }
    }
  } else {
    if (typeof value.require === 'object' && value.require !== null) {
      const types = (value.require as Record<string, unknown>).types;
      if (typeof types === 'string') {
        return join(providerPkgRoot, types);
      }
    }
  }
  return null;
}

async function writeShimDts(
  outDtsPath: string,
  importSpecifier: string,
  upstreamDtsPath: string | null,
  testDistDir: string,
): Promise<void> {
  if (upstreamDtsPath) {
    await writeInlinedProviderDts(outDtsPath, upstreamDtsPath, testDistDir);
    return;
  }
  // Include side-effect import to preserve module augmentations (e.g., toMatchSnapshot on Assertion)
  await writeFile(
    outDtsPath,
    `import '${importSpecifier}';\nexport * from '${importSpecifier}';\n`,
  );
}

/**
 * Create shim file(s) for a single export and return the export entry for package.json.
 *
 * @param shimBaseName Path under dist/test/ (e.g. 'index', 'config', 'browser-playwright/context').
 * @param exportValue  The upstream package's export value for this entry.
 * @param testImportSpecifier The bare import specifier the shim should re-export from
 *   (e.g. 'vitest', 'vitest/node', '@vitest/browser-playwright').
 * @param distDir      Output dist/test directory.
 * @param opts         Optional shim context. Pass `providerPkgRoot` for browser-provider
 *                     packages to inline-copy their upstream d.ts content with specifier rewrites.
 */
async function createShimForExport(
  shimBaseName: string,
  exportValue: unknown,
  testImportSpecifier: string,
  distDir: string,
  opts: { providerPkgRoot?: string } = {},
): Promise<ExportValue | null> {
  const shimDir = join(distDir, dirname(shimBaseName));
  await mkdir(shimDir, { recursive: true });

  const baseFileName = shimBaseName.includes('/') ? shimBaseName.split('/').pop()! : shimBaseName;
  const shimDirForFile = shimBaseName.includes('/') ? shimDir : distDir;

  // Handle different export value formats
  if (typeof exportValue === 'string') {
    // Simple string export: "./browser-compat": "./dist/browser-compat.js"
    // Check if it's a type-only export
    if (exportValue.endsWith('.d.ts')) {
      const dtsPath = join(shimDirForFile, `${baseFileName}.d.ts`);
      const upstream = opts.providerPkgRoot
        ? resolveUpstreamDtsPath(opts.providerPkgRoot, exportValue, 'types')
        : null;
      await writeShimDts(dtsPath, testImportSpecifier, upstream, distDir);
      return { types: `./dist/test/${shimBaseName}.d.ts` };
    }

    const jsPath = join(shimDirForFile, `${baseFileName}.js`);
    await writeFile(jsPath, `export * from '${testImportSpecifier}';\n`);
    return { default: `./dist/test/${shimBaseName}.js` };
  }

  if (typeof exportValue === 'object' && exportValue !== null) {
    const value = exportValue as Record<string, unknown>;

    // Check if it has import/require conditions (complex conditional export)
    if ('import' in value || 'require' in value) {
      return await createConditionalShim(
        value,
        testImportSpecifier,
        shimDirForFile,
        baseFileName,
        shimBaseName,
        distDir,
        opts,
      );
    }

    // Simple object with types/default
    const result: ExportValue = {};

    if (value.types && typeof value.types === 'string') {
      const dtsPath = join(shimDirForFile, `${baseFileName}.d.ts`);
      const upstream = opts.providerPkgRoot
        ? resolveUpstreamDtsPath(opts.providerPkgRoot, value, 'types')
        : null;
      await writeShimDts(dtsPath, testImportSpecifier, upstream, distDir);
      (result as Record<string, string>).types = `./dist/test/${shimBaseName}.d.ts`;
    }

    if (value.default && typeof value.default === 'string') {
      const jsPath = join(shimDirForFile, `${baseFileName}.js`);
      await writeFile(jsPath, `export * from '${testImportSpecifier}';\n`);
      (result as Record<string, string>).default = `./dist/test/${shimBaseName}.js`;
    }

    return Object.keys(result).length > 0 ? result : null;
  }

  return null;
}

/**
 * Handle complex conditional exports with import/require/node conditions
 *
 * Handles both nested structures like:
 *   { import: { types, node, default }, require: { types, default } }
 * And flat structures like:
 *   { types, require, default }
 *
 * Insertion order matters: Node.js package-exports conditions are order-sensitive.
 * For dual-condition entries, `require` MUST come before `default` so that
 * `require('vite-plus/test/config')` resolves to the `.cjs` shim instead of
 * matching the catch-all `default` (which would point at the ESM file).
 */
async function createConditionalShim(
  value: Record<string, unknown>,
  testImportSpecifier: string,
  shimDir: string,
  baseFileName: string,
  shimBaseName: string,
  distDir: string,
  opts: { providerPkgRoot?: string } = {},
): Promise<ExportValue> {
  // Build entries as an array of tuples so we control insertion order explicitly.
  // Final order for flat entries: types, import (if present), require, default.
  // `require` MUST come before `default` — `default` matches everything, so
  // putting it first makes the `require` branch unreachable for CJS consumers.
  const entries: Array<[string, ExportValue]> = [];

  // Handle top-level types (flat structure like { types, require, default })
  if (value.types && typeof value.types === 'string' && !value.import) {
    const dtsPath = join(shimDir, `${baseFileName}.d.ts`);
    const upstream = opts.providerPkgRoot
      ? resolveUpstreamDtsPath(opts.providerPkgRoot, value, 'types')
      : null;
    await writeShimDts(dtsPath, testImportSpecifier, upstream, distDir);
    entries.push(['types', `./dist/test/${shimBaseName}.d.ts`]);
  }

  // Handle import condition
  if (value.import) {
    const importValue = value.import as Record<string, unknown>;

    if (typeof importValue === 'string') {
      const jsPath = join(shimDir, `${baseFileName}.js`);
      await writeFile(jsPath, `export * from '${testImportSpecifier}';\n`);
      entries.push(['import', `./dist/test/${shimBaseName}.js`]);
    } else if (typeof importValue === 'object' && importValue !== null) {
      const importResult: Record<string, string> = {};

      if (importValue.types && typeof importValue.types === 'string') {
        const dtsPath = join(shimDir, `${baseFileName}.d.ts`);
        const upstream = opts.providerPkgRoot
          ? resolveUpstreamDtsPath(opts.providerPkgRoot, value, 'types')
          : null;
        await writeShimDts(dtsPath, testImportSpecifier, upstream, distDir);
        importResult.types = `./dist/test/${shimBaseName}.d.ts`;
      }

      // Create main JS shim - used for both 'node' and 'default' conditions
      const jsPath = join(shimDir, `${baseFileName}.js`);
      await writeFile(jsPath, `export * from '${testImportSpecifier}';\n`);

      if (importValue.node) {
        importResult.node = `./dist/test/${shimBaseName}.js`;
      }
      if (importValue.default) {
        importResult.default = `./dist/test/${shimBaseName}.js`;
      }

      entries.push(['import', importResult]);
    }
  }

  // Handle require condition — emitted BEFORE `default` so CJS resolution
  // picks the `.cjs` shim instead of the catch-all `default` entry.
  if (value.require) {
    const requireValue = value.require as Record<string, unknown>;

    if (typeof requireValue === 'string') {
      const cjsPath = join(shimDir, `${baseFileName}.cjs`);
      await writeFile(cjsPath, `module.exports = require('${testImportSpecifier}');\n`);
      entries.push(['require', `./dist/test/${shimBaseName}.cjs`]);
    } else if (typeof requireValue === 'object' && requireValue !== null) {
      const requireResult: Record<string, string> = {};

      if (requireValue.types && typeof requireValue.types === 'string') {
        const dctsPath = join(shimDir, `${baseFileName}.d.cts`);
        const upstream = opts.providerPkgRoot
          ? resolveUpstreamDtsPath(opts.providerPkgRoot, value, 'require-types')
          : null;
        await writeShimDts(dctsPath, testImportSpecifier, upstream, distDir);
        requireResult.types = `./dist/test/${shimBaseName}.d.cts`;
      }

      if (requireValue.default && typeof requireValue.default === 'string') {
        const cjsPath = join(shimDir, `${baseFileName}.cjs`);
        await writeFile(cjsPath, `module.exports = require('${testImportSpecifier}');\n`);
        requireResult.default = `./dist/test/${shimBaseName}.cjs`;
      }

      entries.push(['require', requireResult]);
    }
  }

  // Handle top-level default (flat structure, only when no import condition).
  // Emitted LAST among siblings so `require` (and any specific condition)
  // wins resolution against the catch-all `default`.
  if (value.default && typeof value.default === 'string' && !value.import) {
    const jsPath = join(shimDir, `${baseFileName}.js`);
    await writeFile(jsPath, `export * from '${testImportSpecifier}';\n`);
    entries.push(['default', `./dist/test/${shimBaseName}.js`]);
  }

  return Object.fromEntries(entries) as ExportValue;
}

/**
 * Update CLI package.json with the generated exports
 */
async function updateCliPackageJson(pkgPath: string, generatedExports: Record<string, unknown>) {
  const pkg = JSON.parse(await readFile(pkgPath, 'utf-8'));

  // Remove old ./test/* exports (if any) to ensure clean sync
  if (pkg.exports) {
    for (const key of Object.keys(pkg.exports)) {
      if (key.startsWith('./test')) {
        delete pkg.exports[key];
      }
    }
  }

  // Add new exports
  pkg.exports = {
    ...pkg.exports,
    ...generatedExports,
  };

  // Ensure dist/test is included in files
  if (!pkg.files.includes('dist/test')) {
    pkg.files.push('dist/test');
  }

  const { code, errors } = await format(pkgPath, JSON.stringify(pkg, null, 2) + '\n', {
    sortPackageJson: true,
  });
  if (errors.length > 0) {
    for (const error of errors) {
      console.error(error);
    }
    process.exit(1);
  }

  await writeFile(pkgPath, code);
}
