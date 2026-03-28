// Build Script for @voidzero-dev/vite-plus-test
//
// Bundles vitest and @vitest/* dependencies with browser/Node.js separation.
//
// ┌─────────────────────────────────────────────────────────────────────┐
// │                          BUILD FLOW                                 │
// ├─────────────────────────────────────────────────────────────────────┤
// │  1. bundleVitest()           Copy vitest-dev → dist/                │
// │  2. copyVitestPackages()     Copy @vitest/* → dist/@vitest/         │
// │  3. collectLeafDependencies() Parse imports with oxc-parser         │
// │  4. bundleLeafDeps()         Bundle chai, pathe, etc → dist/vendor/ │
// │  5. rewriteVitestImports()   Rewrite @vitest/*, vitest/*, vite      │
// │  6. patchVitestPkgRootPaths() Fix distRoot for relocated files      │
// │  7. patchVitestBrowserPackage() Inject vendor-aliases plugin        │
// │  8. patchBrowserProviderLocators() Fix browser-safe imports         │
// │  9. Post-processing:                                                │
// │     - patchVendorPaths()                                            │
// │     - createBrowserCompatShim()                                     │
// │     - createModuleRunnerStub()   Browser-safe stub                  │
// │     - createNodeEntry()          index-node.js with browser-provider│
// │     - copyBrowserClientFiles()                                      │
// │     - createPluginExports()      dist/plugins/* for pnpm overrides  │
// │     - mergePackageJson()                                            │
// │     - validateExternalDeps()                                        │
// └─────────────────────────────────────────────────────────────────────┘
//
// Output Structure:
//   dist/@vitest/*     - Copied packages (browser/Node.js safe)
//   dist/vendor/*      - Bundled leaf dependencies
//   dist/plugins/*     - Shims for pnpm overrides
//   dist/index.js      - Browser-safe entry
//   dist/index-node.js - Node.js entry (includes browser-provider)
//
// Key Design:
//   - COPY @vitest/* to preserve browser/Node.js separation
//   - BUNDLE only leaf deps (chai, etc.) to reduce install size
//   - Separate entries prevent __vite__injectQuery errors in browser

import { existsSync } from 'node:fs';
import {
  copyFile,
  glob as fsGlob,
  mkdir,
  readFile,
  readdir,
  rm,
  stat,
  writeFile,
} from 'node:fs/promises';
import { builtinModules } from 'node:module';
import { basename, join, parse, resolve, dirname, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

import { parseSync } from 'oxc-parser';
import { format } from 'oxfmt';
import { build } from 'rolldown';
import { dts } from 'rolldown-plugin-dts';

import { generateLicenseFile } from '../../scripts/generate-license.ts';
import pkg from './package.json' with { type: 'json' };

const projectDir = dirname(fileURLToPath(import.meta.url));
const vitestSourceDir = resolve(projectDir, 'node_modules/vitest-dev');
const distDir = resolve(projectDir, 'dist');
const vendorDir = resolve(distDir, 'vendor');

const CORE_PACKAGE_NAME = '@voidzero-dev/vite-plus-core';
const TEST_PACKAGE_NAME = '@voidzero-dev/vite-plus-test';

// @vitest/* packages to copy (not bundle) to preserve browser/Node.js separation
// These are copied from node_modules to dist/@vitest/ to avoid shared chunks
// that mix Node.js-only code with browser code
const VITEST_PACKAGES_TO_COPY = [
  '@vitest/runner',
  '@vitest/utils',
  '@vitest/spy',
  '@vitest/expect',
  '@vitest/snapshot',
  '@vitest/mocker',
  '@vitest/pretty-format',
  '@vitest/browser',
  '@vitest/browser-playwright',
  '@vitest/browser-webdriverio',
  '@vitest/browser-preview',
] as const;

// Mapping from @vitest/* package specifiers to their paths within dist/@vitest/
// Used for import rewriting and vendor-aliases plugin
const VITEST_PACKAGE_TO_PATH: Record<string, string> = {
  // @vitest/runner
  '@vitest/runner': '@vitest/runner/index.js',
  '@vitest/runner/utils': '@vitest/runner/utils.js',
  '@vitest/runner/types': '@vitest/runner/types.js',
  // @vitest/utils
  '@vitest/utils': '@vitest/utils/index.js',
  '@vitest/utils/source-map': '@vitest/utils/source-map.js',
  '@vitest/utils/source-map/node': '@vitest/utils/source-map/node.js',
  '@vitest/utils/error': '@vitest/utils/error.js',
  '@vitest/utils/helpers': '@vitest/utils/helpers.js',
  '@vitest/utils/display': '@vitest/utils/display.js',
  '@vitest/utils/timers': '@vitest/utils/timers.js',
  '@vitest/utils/highlight': '@vitest/utils/highlight.js',
  '@vitest/utils/offset': '@vitest/utils/offset.js',
  '@vitest/utils/resolver': '@vitest/utils/resolver.js',
  '@vitest/utils/serialize': '@vitest/utils/serialize.js',
  '@vitest/utils/constants': '@vitest/utils/constants.js',
  '@vitest/utils/diff': '@vitest/utils/diff.js',
  // @vitest/spy
  '@vitest/spy': '@vitest/spy/index.js',
  // @vitest/expect
  '@vitest/expect': '@vitest/expect/index.js',
  // @vitest/snapshot
  '@vitest/snapshot': '@vitest/snapshot/index.js',
  '@vitest/snapshot/environment': '@vitest/snapshot/environment.js',
  '@vitest/snapshot/manager': '@vitest/snapshot/manager.js',
  // @vitest/mocker
  '@vitest/mocker': '@vitest/mocker/index.js',
  '@vitest/mocker/node': '@vitest/mocker/node.js',
  '@vitest/mocker/browser': '@vitest/mocker/browser.js',
  '@vitest/mocker/redirect': '@vitest/mocker/redirect.js',
  '@vitest/mocker/transforms': '@vitest/mocker/transforms.js',
  '@vitest/mocker/automock': '@vitest/mocker/automock.js',
  '@vitest/mocker/register': '@vitest/mocker/register.js',
  // @vitest/pretty-format
  '@vitest/pretty-format': '@vitest/pretty-format/index.js',
  // @vitest/browser
  '@vitest/browser': '@vitest/browser/index.js',
  '@vitest/browser/context': '@vitest/browser/context.js',
  '@vitest/browser/client': '@vitest/browser/client.js',
  '@vitest/browser/locators': '@vitest/browser/locators.js',
  // @vitest/browser-playwright
  '@vitest/browser-playwright': '@vitest/browser-playwright/index.js',
  // @vitest/browser-webdriverio
  '@vitest/browser-webdriverio': '@vitest/browser-webdriverio/index.js',
  // @vitest/browser-preview
  '@vitest/browser-preview': '@vitest/browser-preview/index.js',
};

// Packages that should NOT be bundled into dist/vendor/ (remain external at runtime)
// There are two categories:
// 1. Runtime deps (also in package.json dependencies) - installed with the package, not bundled
// 2. Peer/optional deps (also in peerDependencies) - users must install themselves
const EXTERNAL_BLOCKLIST = new Set([
  // Our own packages - resolved at runtime
  CORE_PACKAGE_NAME,
  `${CORE_PACKAGE_NAME}/module-runner`,
  'vite',
  'vitest',

  // Peer dependencies - consumers must provide these
  '@edge-runtime/vm',
  '@opentelemetry/api',
  '@standard-schema/spec', // Types-only import from @vitest/expect
  'happy-dom',
  'jsdom',

  // Optional dependencies with bundling issues or native bindings
  'debug', // environment detection broken when bundled
  'playwright', // native bindings
  'webdriverio', // native bindings

  // Runtime deps (in package.json dependencies) - not bundled, resolved at install time
  'sirv',
  'ws',
  'pixelmatch',
  'pngjs',

  // MSW (Mock Service Worker) - optional peer dep of @vitest/mocker
  'msw',
  'msw/browser',
  'msw/core/http',
]);

// CJS packages that need their default export destructured to named exports
const CJS_REEXPORT_PACKAGES = new Set(['expect-type']);

// Node built-in modules (including node: prefix variants)
const NODE_BUILTINS = new Set([...builtinModules, ...builtinModules.map((m) => `node:${m}`)]);

// Step 1: Copy vitest-dev dist files (rewriting vite -> core package)
await bundleVitest();

// Step 1.5: Rebrand vitest CLI output as "vp test" with vite-plus version
await brandVitest();

// Step 2: Copy @vitest/* packages from node_modules to dist/@vitest/
// This preserves the original file structure to maintain browser/Node.js separation
await copyVitestPackages();

// Step 2.5: Convert tabs to spaces in all copied JS files for consistent formatting
await convertTabsToSpaces();

// Step 3: Collect leaf dependencies from copied @vitest/* files
// These are external packages like tinyrainbow, pathe, chai, etc.
const leafDeps = await collectLeafDependencies();

// Step 4: Bundle only leaf dependencies into dist/vendor/
// Unlike bundling @vitest/* directly, this avoids shared chunks that mix browser/Node.js code
const leafDepToVendorPath = await bundleLeafDeps(leafDeps);

// Step 5: Rewrite imports in copied @vitest/* and vitest-dev files
// - @vitest/* -> relative paths to dist/@vitest/
// - leaf deps -> relative paths to dist/vendor/
// - vite -> @voidzero-dev/vite-plus-core
await rewriteVitestImports(leafDepToVendorPath);

// Step 6: Fix pkgRoot resolution in all @vitest/* packages
// Files are now at dist/@vitest/*/index.js, so "../.." needs to become "../../.."
await patchVitestPkgRootPaths();

// Step 7: Patch @vitest/browser package (vendor-aliases plugin, exclude list)
await patchVitestBrowserPackage();

// Step 8: Patch browser provider locators.js files for browser-safe imports
await patchBrowserProviderLocators();

// Step 9: Post-processing
await patchVendorPaths();
await patchVitestCoreResolver();
await createBrowserCompatShim();
await createModuleRunnerStub();
await createNodeEntry();
await copyBrowserClientFiles();
await createBrowserEntryFiles();
await patchModuleAugmentations();
await patchChaiTypeReference();
await patchMockerHoistedModule();
await patchServerDepsInline();
const pluginExports = await createPluginExports();
await mergePackageJson(pluginExports);
generateLicenseFile({
  title: 'Vite-Plus test license',
  packageName: 'Vite-Plus',
  outputPath: join(projectDir, 'LICENSE.md'),
  coreLicensePath: join(projectDir, '..', '..', 'LICENSE'),
  bundledPaths: [distDir],
  resolveFrom: [projectDir, join(projectDir, '..', '..')],
  extraPackages: [
    { packageDir: vitestSourceDir },
    ...VITEST_PACKAGES_TO_COPY.map((packageName) => ({
      packageDir: resolve(projectDir, 'node_modules', packageName),
    })),
  ],
});
if (!existsSync(join(projectDir, 'LICENSE.md'))) {
  throw new Error('LICENSE.md was not generated during build');
}
await syncLicenseFromRoot();
await validateExternalDeps();

async function mergePackageJson(pluginExports: Array<{ exportPath: string; shimFile: string }>) {
  const vitestPackageJsonPath = join(vitestSourceDir, 'package.json');
  const destPackageJsonPath = resolve(projectDir, 'package.json');

  const vitestPkg = JSON.parse(await readFile(vitestPackageJsonPath, 'utf-8'));
  const destPkg = JSON.parse(await readFile(destPackageJsonPath, 'utf-8'));

  // Fields to merge from vitest-dev package.json (excluding dependencies since we bundle them)
  const fieldsToMerge = [
    'imports',
    'exports',
    'main',
    'module',
    'types',
    'engines',
    'peerDependencies',
    'peerDependenciesMeta',
  ] as const;

  for (const field of fieldsToMerge) {
    if (vitestPkg[field] !== undefined) {
      destPkg[field] = vitestPkg[field];
    }
  }

  // Remove bundled @vitest/* packages from peerDependencies
  // These browser provider packages are now bundled, so users don't need to install them
  const bundledPeerDeps = [
    '@vitest/browser-playwright',
    '@vitest/browser-webdriverio',
    '@vitest/browser-preview',
  ];
  if (destPkg.peerDependencies) {
    for (const dep of bundledPeerDeps) {
      delete destPkg.peerDependencies[dep];
    }
  }
  if (destPkg.peerDependenciesMeta) {
    for (const dep of bundledPeerDeps) {
      delete destPkg.peerDependenciesMeta[dep];
    }
  }

  destPkg.bundledVersions = {
    ...destPkg.bundledVersions,
    vitest: vitestPkg.version,
  };

  // Add @vitest/browser compatible export (for when this package overrides @vitest/browser)
  // The main "." export is what's used when code imports from @vitest/browser
  if (destPkg.exports) {
    // Add conditional Node.js export to the main entry
    // Node.js code (like @vitest/browser-playwright) uses index-node.js which includes
    // browser-provider exports. Browser code uses index.js which is safe.
    // This separation prevents Node.js-only code (like __vite__injectQuery) from being
    // loaded in the browser, which would cause "Identifier already declared" errors.
    //
    // IMPORTANT: The 'browser' condition must come BEFORE 'node' because vitest passes
    // custom --conditions (like 'browser') to worker processes when frameworks like Nuxt
    // set edge/cloudflare presets. Without the 'browser' condition here, Node.js would
    // match 'node' first, loading index-node.js which imports @vitest/browser/index.js,
    // which imports 'ws'. With --conditions browser active, 'ws' resolves to its browser
    // stub (ws/browser.js) that doesn't export WebSocketServer, causing a SyntaxError.
    // See: https://github.com/voidzero-dev/vite-plus/issues/831
    if (destPkg.exports['.'] && destPkg.exports['.'].import) {
      destPkg.exports['.'].import = {
        types: destPkg.exports['.'].import.types,
        browser: destPkg.exports['.'].import.default,
        node: './dist/index-node.js',
        default: destPkg.exports['.'].import.default,
      };
    }

    destPkg.exports['./browser-compat'] = {
      default: './dist/browser-compat.js',
    };

    // Add @vitest/browser-compatible subpath exports
    // These are needed when this package is used as a pnpm override for @vitest/browser
    // Files are copied to dist/ (not dist/vendor/) to match path resolution in bundled code
    destPkg.exports['./client'] = {
      default: './dist/client.js',
    };
    // Point to @vitest/browser/context.js so that tests and init scripts share the same module
    // This is critical: the init script (locators.js) calls page.extend() on this module,
    // and tests must use the SAME module instance to see the extended methods
    destPkg.exports['./context'] = {
      types: './browser/context.d.ts',
      default: './dist/@vitest/browser/context.js',
    };
    // Also export ./browser/context for users importing vite-plus/test/browser/context
    destPkg.exports['./browser/context'] = {
      types: './browser/context.d.ts',
      default: './dist/@vitest/browser/context.js',
    };
    destPkg.exports['./locators'] = {
      default: './dist/locators.js',
    };
    destPkg.exports['./matchers'] = {
      default: './dist/dummy.js', // Placeholder
    };
    destPkg.exports['./utils'] = {
      default: './dist/dummy.js', // Placeholder
    };

    // Add @vitest/browser-playwright compatible export
    // Users can import { playwright } from 'vitest/browser-playwright'
    destPkg.exports['./browser-playwright'] = {
      types: './dist/@vitest/browser-playwright/index.d.ts',
      default: './dist/@vitest/browser-playwright/index.js',
    };

    // Add @vitest/browser-webdriverio compatible export
    // Users can import { webdriverio } from 'vitest/browser-webdriverio'
    destPkg.exports['./browser-webdriverio'] = {
      types: './dist/@vitest/browser-webdriverio/index.d.ts',
      default: './dist/@vitest/browser-webdriverio/index.js',
    };

    // Add @vitest/browser-preview compatible export
    // Users can import { preview } from 'vitest/browser-preview'
    destPkg.exports['./browser-preview'] = {
      types: './dist/@vitest/browser-preview/index.d.ts',
      default: './dist/@vitest/browser-preview/index.js',
    };

    // Add browser/providers/* alias exports for compatibility
    // Some vitest examples use the nested path format
    destPkg.exports['./browser/providers/playwright'] = {
      types: './dist/@vitest/browser-playwright/index.d.ts',
      default: './dist/@vitest/browser-playwright/index.js',
    };
    destPkg.exports['./browser/providers/webdriverio'] = {
      types: './dist/@vitest/browser-webdriverio/index.d.ts',
      default: './dist/@vitest/browser-webdriverio/index.js',
    };
    destPkg.exports['./browser/providers/preview'] = {
      types: './dist/@vitest/browser-preview/index.d.ts',
      default: './dist/@vitest/browser-preview/index.js',
    };

    // Add plugin exports for all bundled @vitest/* packages
    // This allows pnpm overrides to redirect: @vitest/runner -> vitest/plugins/runner
    for (const { exportPath, shimFile } of pluginExports) {
      destPkg.exports[exportPath] = {
        default: shimFile,
      };
    }
  }

  // Merge vitest dependencies into devDependencies (since we bundle them)
  // Skip packages that are already in dependencies (runtime deps)
  if (vitestPkg.dependencies) {
    destPkg.devDependencies = destPkg.devDependencies || {};
    for (const [dep, version] of Object.entries(vitestPkg.dependencies)) {
      // Skip vite - we use our own core package
      if (dep === 'vite') {
        continue;
      }
      // Skip packages already in dependencies (they're runtime deps, not dev-only)
      if (destPkg.dependencies && destPkg.dependencies[dep]) {
        continue;
      }
      // Don't override existing devDependencies
      if (!destPkg.devDependencies[dep]) {
        destPkg.devDependencies[dep] = version;
      }
    }
  }

  const { code, errors } = await format(
    destPackageJsonPath,
    JSON.stringify(destPkg, null, 2) + '\n',
    {
      experimentalSortPackageJson: true,
    },
  );
  if (errors.length > 0) {
    for (const error of errors) {
      console.error(error);
    }
    process.exit(1);
  }
  await writeFile(destPackageJsonPath, code);
}

async function syncLicenseFromRoot() {
  const rootLicensePath = join(projectDir, '..', '..', 'LICENSE');
  const packageLicensePath = join(projectDir, 'LICENSE');
  await copyFile(rootLicensePath, packageLicensePath);
}

async function bundleVitest() {
  const vitestDestDir = projectDir;

  await mkdir(vitestDestDir, { recursive: true });

  // Get all vitest files excluding node_modules and package.json
  const vitestFiles = fsGlob(join(vitestSourceDir, '**/*'), {
    exclude: [
      join(vitestSourceDir, 'node_modules/**'),
      join(vitestSourceDir, 'package.json'),
      join(vitestSourceDir, 'README.md'),
    ],
  });

  for await (const file of vitestFiles) {
    const stats = await stat(file);
    if (!stats.isFile()) {
      continue;
    }

    const relativePath = file.replace(vitestSourceDir, '');
    const destPath = join(vitestDestDir, relativePath);

    await mkdir(parse(destPath).dir, { recursive: true });

    // Rewrite vite imports in .js, .mjs, and .cjs files
    if (
      file.endsWith('.js') ||
      file.endsWith('.mjs') ||
      file.endsWith('.cjs') ||
      file.endsWith('.d.ts') ||
      file.endsWith('.d.cts')
    ) {
      let content = await readFile(file, 'utf-8');
      content = content
        .replaceAll(/from ['"]vite['"]/g, `from '${CORE_PACKAGE_NAME}'`)
        .replaceAll(/import\(['"]vite['"]\)/g, `import('${CORE_PACKAGE_NAME}')`)
        .replaceAll(/require\(['"]vite['"]\)/g, `require('${CORE_PACKAGE_NAME}')`)
        .replaceAll(/require\("vite"\)/g, `require("${CORE_PACKAGE_NAME}")`)
        .replaceAll(`import 'vite';`, `import '${CORE_PACKAGE_NAME}';`)
        .replaceAll(`'vite/module-runner'`, `'${CORE_PACKAGE_NAME}/module-runner'`)
        .replaceAll(`declare module "vite"`, `declare module "${CORE_PACKAGE_NAME}"`)
        .replaceAll(/import\(['"]vitest['"]\)/g, `import('${TEST_PACKAGE_NAME}')`);
      console.log(`Replaced vite imports in ${destPath}`);
      await writeFile(destPath, content, 'utf-8');
    } else {
      await copyFile(file, destPath);
    }
  }
}

/**
 * Rebrand vitest CLI output as "vp test" with Vite+ banner styling.
 * Patches bundled chunks to replace vitest branding and align banner output.
 */
async function brandVitest() {
  const chunksDir = resolve(projectDir, 'dist/chunks');
  const cacFiles: string[] = [];
  for await (const file of fsGlob(join(chunksDir, 'cac.*.js'))) {
    cacFiles.push(file);
  }
  if (cacFiles.length === 0) {
    throw new Error('brandVitest: no cac chunk found in dist/chunks/');
  }
  for (const cacFile of cacFiles) {
    let content = await readFile(cacFile, 'utf-8');

    function patchString(label: string, search: string | RegExp, replacement: string) {
      const before = content;
      content =
        typeof search === 'string'
          ? content.replace(search, replacement)
          : content.replace(search, replacement);
      if (content === before) {
        throw new Error(
          `brandVitest: failed to patch "${label}" — pattern not found in ${cacFile}`,
        );
      }
    }

    // 1. CLI name: cac("vitest") → cac("vp test")
    patchString('cac name', 'cac("vitest")', 'cac("vp test")');

    // 2. Version: var version = "<semver>" → use VITE_PLUS_VERSION env var with fallback
    patchString(
      'version',
      /var version = "(\d+\.\d+\.\d+[^"]*)"/,
      'var version = process.env.VITE_PLUS_VERSION || "$1"',
    );

    // 3. Banner regex: /^vitest\/\d+\.\d+\.\d+$/ → /^vp test\/[\d.]+$/
    patchString('banner regex', '/^vitest\\/\\d+\\.\\d+\\.\\d+$/', '/^vp test\\/[\\d.]+$/');

    // 4. Help text: $ vitest --help → $ vp test --help
    patchString('help text', '$ vitest --help --expand-help', '$ vp test --help --expand-help');

    await writeFile(cacFile, content, 'utf-8');
    console.log(`Branded vitest → vp test in ${cacFile}`);
  }

  const cliApiFiles: string[] = [];
  for await (const file of fsGlob(join(chunksDir, 'cli-api.*.js'))) {
    cliApiFiles.push(file);
  }
  if (cliApiFiles.length === 0) {
    throw new Error('brandVitest: no cli-api chunk found in dist/chunks/');
  }

  for (const cliApiFile of cliApiFiles) {
    let content = await readFile(cliApiFile, 'utf-8');

    function patchString(label: string, search: string | RegExp, replacement: string) {
      const before = content;
      content =
        typeof search === 'string'
          ? content.replace(search, replacement)
          : content.replace(search, replacement);
      if (content === before) {
        throw new Error(
          `brandVitest: failed to patch "${label}" — pattern not found in ${cliApiFile}`,
        );
      }
    }

    // Remove one extra leading newline before DEV/RUN banner.
    patchString(
      'banner leading newline',
      /printBanner\(\) \{\n\t\tthis\.log\(\);\n/,
      'printBanner() {\n',
    );

    // Use a blue badge for both DEV and RUN.
    patchString(
      'banner color',
      /const color = this\.ctx\.config\.watch \? "blue" : "[a-z]+";\n\t\tconst mode = this\.ctx\.config\.watch \? "DEV" : "RUN";/,
      'const mode = this.ctx.config.watch ? "DEV" : "RUN";\n\t\tconst label = c.bold(c.inverse(c.blue(` ${mode} `)));',
    );

    // Remove the version from the banner line and render a high-contrast label.
    patchString(
      'banner version text',
      /this\.log\(withLabel\(color, mode, (?:""|`v\$\{this\.ctx\.version\} `)\) \+ c\.gray\(this\.ctx\.config\.root\)\);/,
      'this.log(`${label} ${c.gray(this.ctx.config.root)}`);',
    );

    await writeFile(cliApiFile, content, 'utf-8');
    console.log(`Branded vitest banner in ${cliApiFile}`);
  }
}

/**
 * Copy @vitest/* packages from node_modules to dist/@vitest/
 * This preserves the original file structure to maintain browser/Node.js separation.
 * Unlike bundling with Rolldown, copying avoids creating shared chunks that mix
 * Node.js-only code with browser code.
 */
async function copyVitestPackages() {
  console.log('\nCopying @vitest/* packages to dist/@vitest/...');

  const vitestDir = resolve(distDir, '@vitest');
  await rm(vitestDir, { recursive: true, force: true });
  await mkdir(vitestDir, { recursive: true });

  let totalCopied = 0;

  for (const pkg of VITEST_PACKAGES_TO_COPY) {
    const pkgName = pkg.replace('@vitest/', '');
    const srcDir = resolve(projectDir, `node_modules/${pkg}/dist`);
    const destPkgDir = resolve(vitestDir, pkgName);

    try {
      await stat(srcDir);
    } catch {
      console.log(`  Warning: ${pkg} not installed, skipping`);
      continue;
    }

    console.log(`  Copying ${pkg}...`);
    const copied = await copyDirRecursive(srcDir, destPkgDir);
    totalCopied += copied;
    console.log(`    -> ${copied} files`);

    // Copy root .d.ts files from @vitest/browser package directory.
    // These are type definitions that live at the package root (not in dist/),
    // e.g. context.d.ts, matchers.d.ts, aria-role.d.ts, utils.d.ts.
    // Dynamically scan instead of hardcoding to handle future upstream additions.
    if (pkg === '@vitest/browser') {
      const pkgRoot = resolve(projectDir, `node_modules/${pkg}`);
      try {
        const pkgEntries = await readdir(pkgRoot);
        for (const entry of pkgEntries) {
          if (entry.endsWith('.d.ts')) {
            await copyFile(join(pkgRoot, entry), join(destPkgDir, entry));
            console.log(`    + copied ${entry}`);
            totalCopied++;
          }
        }
      } catch {
        // Package root not readable, skip
      }
    }
  }

  console.log(`\nCopied ${totalCopied} files to dist/@vitest/`);
}

/**
 * Recursively copy a directory
 */
async function copyDirRecursive(srcDir: string, destDir: string): Promise<number> {
  await mkdir(destDir, { recursive: true });
  const entries = await readdir(srcDir, { withFileTypes: true });
  let count = 0;

  for (const entry of entries) {
    const srcPath = join(srcDir, entry.name);
    const destPath = join(destDir, entry.name);

    if (entry.isDirectory()) {
      count += await copyDirRecursive(srcPath, destPath);
    } else if (entry.isFile()) {
      await copyFile(srcPath, destPath);
      count++;
    }
  }

  return count;
}

/**
 * Collect leaf dependencies from copied @vitest/* files AND vitest core dist files.
 * These are external packages that should be bundled (tinyrainbow, pathe, chai, expect-type, etc.)
 * but NOT @vitest/*, vitest/*, vite/*, node built-ins, or blocklisted packages.
 */
async function collectLeafDependencies(): Promise<Set<string>> {
  console.log('\nCollecting leaf dependencies from dist/...');

  const leafDeps = new Set<string>();
  const vitestDir = resolve(distDir, '@vitest');

  // Scan both @vitest/* packages AND vitest core dist files
  const jsFiles = fsGlob([
    join(vitestDir, '**/*.js'),
    join(distDir, '*.js'),
    join(distDir, 'chunks/*.js'),
  ]);

  for await (const file of jsFiles) {
    const content = await readFile(file, 'utf-8');
    const result = parseSync(file, content, { sourceType: 'module' });

    // Collect ESM static imports
    for (const imp of result.module.staticImports) {
      const specifier = imp.moduleRequest.value;
      if (isLeafDependency(specifier)) {
        leafDeps.add(specifier);
      }
    }

    // Collect ESM static exports (re-exports)
    for (const exp of result.module.staticExports) {
      for (const entry of exp.entries) {
        if (entry.moduleRequest) {
          const specifier = entry.moduleRequest.value;
          if (isLeafDependency(specifier)) {
            leafDeps.add(specifier);
          }
        }
      }
    }

    // Collect dynamic imports (only string literals)
    for (const dynImp of result.module.dynamicImports) {
      const rawText = content.slice(dynImp.moduleRequest.start, dynImp.moduleRequest.end);
      if (
        (rawText.startsWith("'") && rawText.endsWith("'")) ||
        (rawText.startsWith('"') && rawText.endsWith('"'))
      ) {
        const specifier = rawText.slice(1, -1);
        if (isLeafDependency(specifier)) {
          leafDeps.add(specifier);
        }
      }
    }
  }

  console.log(`Found ${leafDeps.size} leaf dependencies:`);
  for (const dep of leafDeps) {
    console.log(`  - ${dep}`);
  }

  return leafDeps;
}

/**
 * Check if a specifier is a leaf dependency that should be bundled.
 * Leaf deps are external packages that are NOT:
 * - @vitest/* (we copy these)
 * - vitest or vitest/* (we copy vitest-dev)
 * - vite or vite/* (we use our core package)
 * - Node.js built-ins
 * - Blocklisted packages
 * - Relative paths
 */
function isLeafDependency(specifier: string): boolean {
  // Relative paths
  if (specifier.startsWith('.') || specifier.startsWith('/')) {
    return false;
  }
  // @vitest/* packages (we copy these)
  if (specifier.startsWith('@vitest/')) {
    return false;
  }
  // vitest or vitest/* (we copy vitest-dev)
  if (specifier === 'vitest' || specifier.startsWith('vitest/')) {
    return false;
  }
  // vite or vite/* (we use our core package)
  if (specifier === 'vite' || specifier.startsWith('vite/')) {
    return false;
  }
  // Node.js built-ins
  if (NODE_BUILTINS.has(specifier)) {
    return false;
  }
  // Blocklisted packages
  if (EXTERNAL_BLOCKLIST.has(specifier)) {
    return false;
  }
  // Node.js subpath imports (#module-evaluator, etc.)
  if (specifier.startsWith('#')) {
    return false;
  }
  // Invalid specifiers
  if (!/^(@[a-z0-9-~][a-z0-9-._~]*\/)?[a-z0-9-~][a-z0-9-._~]*/.test(specifier)) {
    return false;
  }
  return true;
}

/**
 * Bundle only leaf dependencies into dist/vendor/.
 * Only bundles non-@vitest deps (tinyrainbow, pathe, chai, etc.)
 * to avoid shared chunks that mix Node.js and browser code.
 */
async function bundleLeafDeps(leafDeps: Set<string>): Promise<Map<string, string>> {
  console.log('\nBundling leaf dependencies...');

  await rm(vendorDir, { recursive: true, force: true });
  await mkdir(vendorDir, { recursive: true });

  const specifierToVendorPath = new Map<string, string>();

  if (leafDeps.size === 0) {
    console.log('  No leaf dependencies to bundle.');
    return specifierToVendorPath;
  }

  // Build input object with all leaf deps
  const input: Record<string, string> = {};
  for (const dep of leafDeps) {
    const safeName = safeFileName(dep);
    input[safeName] = dep;
  }

  try {
    await build({
      input,
      output: {
        dir: vendorDir,
        format: 'esm',
        entryFileNames: '[name].mjs',
        chunkFileNames: 'shared-[hash].mjs',
      },
      platform: 'neutral',
      treeshake: false,
      external: [
        // Keep node built-ins external
        ...NODE_BUILTINS,
        // Keep blocklisted packages external
        ...EXTERNAL_BLOCKLIST,
        // Keep @vitest/* external (we copy them)
        /@vitest\//,
        // Keep vitest external (we copy it)
        /^vitest(\/.*)?$/,
        // Keep vite external (we use core package)
        /^vite(\/.*)?$/,
      ],
      resolve: {
        conditionNames: ['node', 'import', 'default'],
        mainFields: ['module', 'main'],
      },
      logLevel: 'warn',
    });

    const dtsInput = { ...input };

    for (const name of Object.keys(dtsInput)) {
      const vendorDtsPath = join(vendorDir, `vendor_${name}.d.ts`);
      dtsInput[name] = vendorDtsPath;
      await writeFile(vendorDtsPath, `export * from '${name}';`, 'utf-8');
    }

    await build({
      input: dtsInput,
      output: {
        dir: vendorDir,
        format: 'esm',
        entryFileNames: '[name].mts',
      },
      plugins: [
        dts({
          dtsInput: true,
          oxc: true,
          resolver: 'oxc',
          emitDtsOnly: true,
          tsconfig: false,
        }),
      ],
    });

    for (const p of Object.values(dtsInput)) {
      await rm(p);
    }

    // Register all specifiers
    for (const dep of leafDeps) {
      const safeName = safeFileName(dep);
      const vendorFilePath = join(vendorDir, `${safeName}.mjs`);
      specifierToVendorPath.set(dep, vendorFilePath);
      console.log(`  -> vendor/${safeName}.mjs`);

      // Fix CJS packages that need named exports extracted from default
      if (CJS_REEXPORT_PACKAGES.has(dep)) {
        await fixCjsNamedExports(vendorFilePath, dep);
      }
    }
  } catch (error) {
    console.error('Failed to bundle leaf dependencies:', error);
    throw error;
  }

  console.log(`\nBundled ${specifierToVendorPath.size} leaf dependencies.`);
  return specifierToVendorPath;
}

/**
 * Rewrite imports in all copied @vitest/* files and vitest-dev dist files.
 * This handles:
 * - @vitest/* -> relative paths to dist/@vitest/
 * - vitest/* -> relative paths to dist/
 * - vite -> @voidzero-dev/vite-plus-core
 * - leaf deps -> relative paths to dist/vendor/
 */
async function rewriteVitestImports(leafDepToVendorPath: Map<string, string>) {
  console.log('\nRewriting imports in @vitest/* and vitest core files...');

  const vitestDir = resolve(distDir, '@vitest');
  let rewrittenCount = 0;

  // Scan both @vitest/* packages AND vitest core dist files
  // Include .d.ts files so TypeScript type imports also get rewritten
  const jsFiles = fsGlob([
    join(vitestDir, '**/*.js'),
    join(vitestDir, '**/*.d.ts'),
    join(distDir, '*.js'),
    join(distDir, '*.d.ts'),
    join(distDir, 'chunks/*.js'),
    join(distDir, 'chunks/*.d.ts'),
  ]);

  for await (const file of jsFiles) {
    let content = await readFile(file, 'utf-8');
    const fileDir = dirname(file);

    // Build specifier map for this file
    const specifierMap = new Map<string, string>();

    // Add @vitest/* mappings (relative paths)
    for (const [pkg, destPath] of Object.entries(VITEST_PACKAGE_TO_PATH)) {
      const absoluteDest = resolve(distDir, destPath);
      let relativePath = relative(fileDir, absoluteDest);
      relativePath = relativePath.split('\\').join('/'); // Windows fix
      if (!relativePath.startsWith('.')) {
        relativePath = './' + relativePath;
      }
      specifierMap.set(pkg, absoluteDest);
    }

    // Add vitest/* mappings (relative to dist/)
    const vitestSubpathRewrites: Record<string, string> = {
      vitest: resolve(distDir, 'index.js'),
      'vitest/node': resolve(distDir, 'node.js'),
      'vitest/config': resolve(distDir, 'config.js'),
      // vitest/browser exports page, server, CDPSession, BrowserCommands, etc from @vitest/browser/context
      // This matches vitest's own package.json exports: "./browser" -> "./browser/context.d.ts"
      'vitest/browser': resolve(distDir, '@vitest/browser/context.js'),
      // vitest/internal/browser exports browser-safe __INTERNAL and stringify (NOT @vitest/browser/index.js which has Node.js code)
      'vitest/internal/browser': resolve(distDir, 'browser.js'),
      'vitest/runners': resolve(distDir, 'runners.js'),
      'vitest/suite': resolve(distDir, 'suite.js'),
      'vitest/environments': resolve(distDir, 'environments.js'),
      'vitest/coverage': resolve(distDir, 'coverage.js'),
      'vitest/reporters': resolve(distDir, 'reporters.js'),
      'vitest/snapshot': resolve(distDir, 'snapshot.js'),
      'vitest/mocker': resolve(distDir, 'mocker.js'),
    };
    for (const [specifier, absolutePath] of Object.entries(vitestSubpathRewrites)) {
      specifierMap.set(specifier, absolutePath);
    }

    // Add leaf dep mappings (relative to vendor/)
    for (const [specifier, vendorPath] of leafDepToVendorPath) {
      specifierMap.set(specifier, vendorPath);
    }

    // For files inside @vitest/browser/, preserve 'vitest/browser' as a bare specifier.
    // These files run in browser context where the vitest:vendor-aliases plugin
    // resolves 'vitest/browser' to the virtual module '\0vitest/browser',
    // which provides browser-safe context API (page, server, userEvent, utils).
    // Without this, 'vitest/browser' gets rewritten to './index.js' which resolves
    // to the Node.js server file (~9000 lines of node:fs, ws, etc.)
    if (file.includes('@vitest/browser') || file.includes('@vitest\\browser')) {
      specifierMap.delete('vitest/browser');
    }

    // Rewrite using AST
    const rewritten = rewriteImportsWithAst(content, file, false, specifierMap);

    // Also rewrite vite -> core package (simple string replacement since it's a package name)
    let finalContent = rewritten
      .replaceAll(/from ['"]vite['"]/g, `from '${CORE_PACKAGE_NAME}'`)
      .replaceAll(/import\(['"]vite['"]\)/g, `import('${CORE_PACKAGE_NAME}')`)
      .replaceAll(`'vite/module-runner'`, `'${CORE_PACKAGE_NAME}/module-runner'`);

    // Special handling for @vitest/mocker entry files that have redundant side-effect imports
    // The original files have: import 'magic-string'; export {...} from './chunk-automock.js'; import 'estree-walker';
    // This is problematic because:
    // 1. Side-effect imports are redundant (chunk files already import what they need)
    // 2. Having imports after exports can confuse some module parsers
    // Fix: Remove redundant side-effect imports from vendor deps in entry files
    if (file.includes('@vitest/mocker') || file.includes('@vitest\\mocker')) {
      // Get the base filename
      const baseName = file.split(/[/\\]/).pop();
      // Only process entry files (not chunk files)
      if (baseName && !baseName.startsWith('chunk-')) {
        // Remove side-effect imports from vendor deps (these are redundant since chunk files import them)
        finalContent = finalContent.replace(/import\s*['"][^'"]*vendor[^'"]*\.mjs['"];?\s*/g, '');
      }
    }

    if (finalContent !== content) {
      await writeFile(file, finalContent, 'utf-8');
      rewrittenCount++;
    }
  }

  console.log(`  Rewrote imports in ${rewrittenCount} files`);

  // Also rewrite imports in the main vitest-dev dist files
  console.log('\nRewriting imports in vitest-dev dist files...');
  let mainRewrittenCount = 0;

  const mainJsFiles = fsGlob(join(distDir, '*.js'));
  const chunksJsFiles = fsGlob(join(distDir, 'chunks', '*.js'));
  const workersJsFiles = fsGlob(join(distDir, 'workers', '*.js'));

  for await (const file of mainJsFiles) {
    const rewritten = await rewriteDistFile(file, leafDepToVendorPath);
    if (rewritten) {
      mainRewrittenCount++;
    }
  }
  for await (const file of chunksJsFiles) {
    const rewritten = await rewriteDistFile(file, leafDepToVendorPath);
    if (rewritten) {
      mainRewrittenCount++;
    }
  }
  for await (const file of workersJsFiles) {
    const rewritten = await rewriteDistFile(file, leafDepToVendorPath);
    if (rewritten) {
      mainRewrittenCount++;
    }
  }

  console.log(`  Rewrote imports in ${mainRewrittenCount} dist files`);
}

/**
 * Rewrite imports in a vitest-dev dist file.
 * Returns true if the file was modified.
 */
async function rewriteDistFile(
  file: string,
  leafDepToVendorPath: Map<string, string>,
): Promise<boolean> {
  let content = await readFile(file, 'utf-8');

  // Build specifier map
  const specifierMap = new Map<string, string>();

  // Add @vitest/* mappings
  for (const [pkg, destPath] of Object.entries(VITEST_PACKAGE_TO_PATH)) {
    const absoluteDest = resolve(distDir, destPath);
    specifierMap.set(pkg, absoluteDest);
  }

  // Add leaf dep mappings
  for (const [specifier, vendorPath] of leafDepToVendorPath) {
    specifierMap.set(specifier, vendorPath);
  }

  // Add vitest/* subpath mappings
  // NOTE: Do NOT include 'vitest/browser' - it must be handled by
  // the vitest:browser:virtual-module:context plugin at runtime
  const vitestSubpathRewrites: Record<string, string> = {
    vitest: resolve(distDir, 'index.js'),
    'vitest/node': resolve(distDir, 'node.js'),
    'vitest/config': resolve(distDir, 'config.js'),
    // 'vitest/browser' - intentionally omitted, handled by virtual module plugin
    'vitest/internal/browser': resolve(distDir, 'browser.js'),
    'vitest/runners': resolve(distDir, 'runners.js'),
    'vitest/suite': resolve(distDir, 'suite.js'),
    'vitest/environments': resolve(distDir, 'environments.js'),
    'vitest/coverage': resolve(distDir, 'coverage.js'),
    'vitest/reporters': resolve(distDir, 'reporters.js'),
    'vitest/snapshot': resolve(distDir, 'snapshot.js'),
    'vitest/mocker': resolve(distDir, 'mocker.js'),
  };
  for (const [specifier, absolutePath] of Object.entries(vitestSubpathRewrites)) {
    specifierMap.set(specifier, absolutePath);
  }

  // Add mappings for ./vendor/vitest_*.mjs relative imports
  // These are vitest-dev's bundled @vitest/* packages that we've copied to dist/@vitest/
  const vendorToVitest: Record<string, string> = {
    './vendor/vitest_runner.mjs': resolve(distDir, '@vitest/runner/index.js'),
    './vendor/vitest_runners.mjs': resolve(distDir, 'runners.js'),
    './vendor/vitest_browser.mjs': resolve(distDir, '@vitest/browser/context.js'),
    './vendor/vitest_internal_browser.mjs': resolve(distDir, 'browser.js'),
    './vendor/vitest_utils.mjs': resolve(distDir, '@vitest/utils/index.js'),
    './vendor/vitest_spy.mjs': resolve(distDir, '@vitest/spy/index.js'),
    './vendor/vitest_snapshot.mjs': resolve(distDir, '@vitest/snapshot/index.js'),
    './vendor/vitest_expect.mjs': resolve(distDir, '@vitest/expect/index.js'),
  };
  for (const [vendorPath, destPath] of Object.entries(vendorToVitest)) {
    specifierMap.set(vendorPath, destPath);
  }

  let rewritten = rewriteImportsWithAst(content, file, false, specifierMap);

  // Strip module-runner side-effect import from index.js
  // This import is Node.js-only and causes browser tests to hang when vitest/index.js
  // is loaded in browser context (to get describe, it, expect, etc.)
  // The module-runner contains Node.js code (process.platform, etc.) that browsers can't execute
  if (basename(file) === 'index.js') {
    rewritten = rewritten.replace(
      /import\s*['"]@voidzero-dev\/vite-plus-core\/module-runner['"];?\s*/g,
      '',
    );
  }

  if (rewritten !== content) {
    await writeFile(file, rewritten, 'utf-8');
    return true;
  }
  return false;
}

/**
 * Rewrite imports using oxc-parser AST for precise replacements
 */
function rewriteImportsWithAst(
  content: string,
  filePath: string,
  isCjs: boolean,
  specifierToVendorPath: Map<string, string>,
): string {
  // Use Map to deduplicate replacements by start position
  const replacementMap = new Map<number, [number, number, string]>();

  // Helper to get relative path for a specifier
  const getRelativePath = (specifier: string): string | null => {
    const vendorPath = specifierToVendorPath.get(specifier);
    if (!vendorPath) {
      return null;
    }
    let relativePath = relative(dirname(filePath), vendorPath);
    // Normalize to forward slashes for ES module imports (Windows uses backslashes)
    relativePath = relativePath.split('\\').join('/');
    if (!relativePath.startsWith('.')) {
      relativePath = './' + relativePath;
    }
    return relativePath;
  };

  // Helper to add replacement (deduplicates by start position)
  const addReplacement = (start: number, end: number, newValue: string) => {
    if (!replacementMap.has(start)) {
      replacementMap.set(start, [start, end, newValue]);
    }
  };

  // Parse with oxc-parser
  const result = parseSync(filePath, content, {
    sourceType: isCjs ? 'script' : 'module',
  });

  // Collect ESM static imports
  for (const imp of result.module.staticImports) {
    const specifier = imp.moduleRequest.value;
    const relativePath = getRelativePath(specifier);
    if (relativePath) {
      // Replace the module request (including quotes)
      addReplacement(imp.moduleRequest.start, imp.moduleRequest.end, `'${relativePath}'`);
    }
  }

  // Collect ESM static exports (re-exports)
  for (const exp of result.module.staticExports) {
    for (const entry of exp.entries) {
      if (entry.moduleRequest) {
        const specifier = entry.moduleRequest.value;
        const relativePath = getRelativePath(specifier);
        if (relativePath) {
          addReplacement(entry.moduleRequest.start, entry.moduleRequest.end, `'${relativePath}'`);
        }
      }
    }
  }

  // Collect dynamic imports (only string literals)
  for (const dynImp of result.module.dynamicImports) {
    const rawText = content.slice(dynImp.moduleRequest.start, dynImp.moduleRequest.end);
    if (
      (rawText.startsWith("'") && rawText.endsWith("'")) ||
      (rawText.startsWith('"') && rawText.endsWith('"'))
    ) {
      const specifier = rawText.slice(1, -1);
      const relativePath = getRelativePath(specifier);
      if (relativePath) {
        addReplacement(dynImp.moduleRequest.start, dynImp.moduleRequest.end, `'${relativePath}'`);
      }
    }
  }

  // For CJS files, also handle require() calls using regex (oxc-parser doesn't track these)
  if (isCjs) {
    const requireRegex = /require\s*\(\s*(['"])([^'"]+)\1\s*\)/g;
    let match;
    while ((match = requireRegex.exec(content)) !== null) {
      const specifier = match[2];
      const relativePath = getRelativePath(specifier);
      if (relativePath) {
        // Calculate the position of just the string literal (including quotes)
        const stringStart = match.index + match[0].indexOf(match[1]);
        const stringEnd = stringStart + match[1].length + specifier.length + match[1].length;
        addReplacement(stringStart, stringEnd, `'${relativePath}'`);
      }
    }
  }

  // Sort replacements in reverse order (end to start) to preserve positions
  // eslint-disable-next-line unicorn/no-array-sort -- safe: sorting a fresh spread copy
  const replacements = [...replacementMap.values()].sort((a, b) => b[0] - a[0]);

  // Apply replacements
  let result_content = content;
  for (const [start, end, newValue] of replacements) {
    result_content = result_content.slice(0, start) + newValue + result_content.slice(end);
  }

  return result_content;
}

/**
 * Fix CJS packages that only export default - extract named exports from the default export
 */
async function fixCjsNamedExports(vendorFilePath: string, specifier: string) {
  let content = await readFile(vendorFilePath, 'utf-8');

  // Match pattern like: export default require_xxx();
  // and: export {  };
  const defaultExportMatch = content.match(/export default (require_\w+)\(\);/);

  if (defaultExportMatch) {
    const requireFn = defaultExportMatch[1];
    console.log(`      Fixing CJS named exports for ${specifier}...`);

    const emptyExportMatch = content.match(/export \{\s*\};/);
    if (emptyExportMatch) {
      // Pattern: export default require_xxx();\nexport {  };
      content = content.replace(
        /export default (require_\w+)\(\);\s*\nexport \{\s*\};/,
        `const __cjs_export__ = ${requireFn}();\nexport const { expectTypeOf } = __cjs_export__;\nexport default __cjs_export__;`,
      );
    } else {
      // Pattern: export default require_xxx(); (no empty export block)
      content = content.replace(
        /export default (require_\w+)\(\);/,
        `const __cjs_export__ = ${requireFn}();\nexport const { expectTypeOf } = __cjs_export__;\nexport default __cjs_export__;`,
      );
    }

    await writeFile(vendorFilePath, content, 'utf-8');
  }
}

/**
 * Create a safe filename from a specifier
 */
function safeFileName(specifier: string): string {
  return specifier.replace(/[@/]/g, '_').replace(/^_/, '');
}

/**
 * Patch pkgRoot/distRoot paths in vendor files.
 * The bundled code assumes files are in dist/, but vendor files are in dist/vendor/
 * So "../.." needs to become "../../.." to correctly resolve to package root.
 * Also patches relative file references like "context.js" to "../context.js".
 */
async function patchVendorPaths() {
  console.log('\nPatching vendor paths...');

  // Patterns that need one more level up due to vendor subdirectory
  const pathPatterns = [
    // Package root calculation: "../.." -> "../../.."
    {
      original: `resolve$1(fileURLToPath(import.meta.url), "../..")`,
      fixed: `resolve$1(fileURLToPath(import.meta.url), "../../..")`,
    },
    // context.js reference: "context.js" -> "../context.js"
    // This is used in browser server to resolve the vitest/browser/context export
    {
      original: `resolve$1(__dirname$1, "context.js")`,
      fixed: `resolve$1(__dirname$1, "../context.js")`,
    },
  ];

  const vendorFiles = fsGlob(join(vendorDir, '*.mjs'));
  let patchedCount = 0;

  for await (const file of vendorFiles) {
    let content = await readFile(file, 'utf-8');
    let modified = false;

    for (const { original, fixed } of pathPatterns) {
      if (content.includes(original)) {
        content = content.replaceAll(original, fixed);
        modified = true;
      }
    }

    if (modified) {
      await writeFile(file, content, 'utf-8');
      console.log(`  Patched paths in ${relative(distDir, file)}`);
      patchedCount++;
    }
  }

  if (patchedCount === 0) {
    console.log('  No vendor files needed path patching');
  } else {
    console.log(`  Successfully patched ${patchedCount} file(s)`);
  }
}

/**
 * Patch VitestCoreResolver to resolve vite-plus/test directly.
 *
 * Problem: CLI's `export * from '@voidzero-dev/vite-plus-test'` creates a re-export
 * chain that breaks module identity in Vite's SSR transform. expect.extend()
 * mutations aren't visible through the re-export.
 *
 * Fix: Make VitestCoreResolver resolve both vite-plus/test and
 * @voidzero-dev/vite-plus-test directly to dist/index.js, bypassing re-exports.
 */
async function patchVitestCoreResolver() {
  console.log('\nPatching VitestCoreResolver for CLI package alias...');

  let cliApiChunk: string | undefined;
  for await (const chunk of fsGlob(join(distDir, 'chunks/cli-api.*.js'))) {
    cliApiChunk = chunk;
    break;
  }

  if (!cliApiChunk) {
    throw new Error('cli-api chunk not found');
  }
  let content = await readFile(cliApiChunk, 'utf8');

  // Find the VitestCoreResolver resolveId function and add our package aliases
  const oldPattern = `async resolveId(id) {
      if (id === "vitest") return resolve(distDir, "index.js");
      if (id.startsWith("@vitest/") || id.startsWith("vitest/"))`;

  const newCode = `async resolveId(id) {
      if (id === "vitest") return resolve(distDir, "index.js");
      // Resolve CLI test path and test package directly to dist/index.js
      // This bypasses the re-export chain and ensures module identity is preserved
      if (id === "vite-plus/test" || id === "@voidzero-dev/vite-plus-test") {
        return resolve(distDir, "index.js");
      }
      // Handle subpaths: vite-plus/test/* -> vitest/*
      if (id.startsWith("vite-plus/test/")) {
        const subpath = id.slice("vite-plus/test/".length);
        return this.resolve("vitest/" + subpath, join(ctx.config.root, "index.html"), { skipSelf: true });
      }
      // Handle subpaths: @voidzero-dev/vite-plus-test/* -> vitest/*
      if (id.startsWith("@voidzero-dev/vite-plus-test/")) {
        const subpath = id.slice("@voidzero-dev/vite-plus-test/".length);
        return this.resolve("vitest/" + subpath, join(ctx.config.root, "index.html"), { skipSelf: true });
      }
      if (id.startsWith("@vitest/") || id.startsWith("vitest/"))`;

  if (!content.includes(oldPattern)) {
    throw new Error(
      'Could not find VitestCoreResolver pattern to patch in ' +
        cliApiChunk +
        '. ' +
        'This likely means vitest code has changed and the patch needs to be updated.',
    );
  }

  content = content.replace(oldPattern, newCode);
  await writeFile(cliApiChunk, content);
  console.log('  Patched VitestCoreResolver to resolve vite-plus/test directly');
}

/**
 * Convert tabs to spaces in all JS files in dist/ for consistent formatting.
 * This allows our patching code to use space-based patterns instead of tabs.
 */
async function convertTabsToSpaces() {
  console.log('\nConverting tabs to spaces in dist/...');

  let convertedCount = 0;

  for await (const file of fsGlob(resolve(distDir, '**/*.js'))) {
    const content = await readFile(file, 'utf-8');
    if (content.includes('\t')) {
      const converted = content.replace(/\t/g, '  ');
      await writeFile(file, converted);
      convertedCount++;
    }
  }

  console.log(`  Converted ${convertedCount} files`);
}

/**
 * Fix pkgRoot path resolution in all `@vitest/*` packages.
 * The original packages use resolve(import.meta.url, "../..") to find their package root.
 * But our files are at `dist/@vitest/star/index.js`, so we need to go up 3 levels, not 2.
 */
async function patchVitestPkgRootPaths() {
  console.log('\nFixing distRoot paths in @vitest/* packages...');

  const vitestDir = resolve(distDir, '@vitest');
  let patchedCount = 0;

  for (const pkg of VITEST_PACKAGES_TO_COPY) {
    const pkgName = pkg.replace('@vitest/', '');
    const indexPath = join(vitestDir, pkgName, 'index.js');

    try {
      await stat(indexPath);
    } catch {
      continue;
    }

    let content = await readFile(indexPath, 'utf-8');

    // The original @vitest/browser had index.js in the dist/ folder, so:
    //   pkgRoot = resolve(import.meta.url, "../..") -> @vitest/browser
    //   distRoot = resolve(pkgRoot, "dist") -> @vitest/browser/dist
    // But our file is at dist/@vitest/browser/index.js, so distRoot should just be
    // the directory containing index.js (not pkgRoot/dist)
    // Replace both lines with just making distRoot = dirname of index.js
    // Use regex to handle both top-level and indented occurrences
    const oldPattern =
      /( *)const pkgRoot = resolve\(fileURLToPath\(import\.meta\.url\), "\.\.\/\.\."\);\n\1const distRoot = resolve\(pkgRoot, "dist"\);/g;
    const newContent = content.replace(
      oldPattern,
      '$1const distRoot = dirname(fileURLToPath(import.meta.url));',
    );

    if (newContent !== content) {
      await writeFile(indexPath, newContent, 'utf-8');
      const matchCount = (content.match(oldPattern) || []).length;
      console.log(`  Fixed ${pkg}/index.js (${matchCount} occurrences)`);
      patchedCount++;
    }
  }

  console.log(`  Patched ${patchedCount} packages`);
}

/**
 * Patch the copied @vitest/browser package to:
 * 1. Inject vitest:vendor-aliases plugin for @vitest/* resolution
 * 2. Add native deps to the exclude list
 * 3. Remove include patterns for bundled deps
 */
async function patchVitestBrowserPackage() {
  console.log('\nPatching @vitest/browser package...');

  const browserIndexPath = join(distDir, '@vitest/browser/index.js');

  try {
    await stat(browserIndexPath);
  } catch {
    console.log('  Warning: @vitest/browser not found in dist, skipping');
    return;
  }

  let content = await readFile(browserIndexPath, 'utf-8');

  // 1. Inject vitest:vendor-aliases plugin into BrowserPlugin return array
  // This allows imports like @vitest/runner to be resolved to our copied @vitest files
  // Exclude @vitest/browser/context from vendor-aliases so that BrowserContext
  // plugin's resolveId can intercept the bare specifier and return the virtual
  // module (which includes the dynamically generated `server` export).
  // Without this, vendor-aliases resolves the bare specifier to the static
  // context.js file (which has no `server`), bypassing BrowserContext entirely.
  // See: https://github.com/voidzero-dev/vite-plus/issues/1086
  const VENDOR_ALIASES_EXCLUDE = new Set(['@vitest/browser/context']);

  const mappingEntries = Object.entries(VITEST_PACKAGE_TO_PATH)
    .filter(([pkg]) => pkg.startsWith('@vitest/') && !VENDOR_ALIASES_EXCLUDE.has(pkg))
    .map(([pkg, file]) => `'${pkg}': resolve(packageRoot, '${file}')`)
    .join(',\n      ');

  // distRoot is @vitest/browser/ so we need to go up two levels to reach the actual dist root
  const vendorAliasesPlugin = `{
    name: 'vitest:vendor-aliases',
    enforce: 'pre',
    resolveId(id) {
      // distRoot is @vitest/browser/, packageRoot is the actual dist/ directory
      const packageRoot = resolve(distRoot, '../..');
      // Resolve module-runner to a browser-safe stub
      // This is critical: module-runner contains Node.js-only code (process.platform, etc.)
      // that causes browsers to hang when loaded
      if (id === '${CORE_PACKAGE_NAME}/module-runner' || id === 'vite/module-runner') {
        return resolve(packageRoot, 'module-runner-stub.js');
      }
      // Mark vite/core as external to prevent Node.js-only code from being bundled
      // This prevents __vite__injectQuery duplication errors in browser tests
      if (id === '${CORE_PACKAGE_NAME}' || id === 'vite') {
        return { id, external: true };
      }
      // Handle vitest/browser and package aliases
      // Return virtual module ID so BrowserContext plugin can load it
      // Supports: vitest/browser, @voidzero-dev/vite-plus-test/browser, vite-plus/test/browser
      if (id === 'vitest/browser' || id === '@voidzero-dev/vite-plus-test/browser' || id === 'vite-plus/test/browser') {
        return '\\0vitest/browser';
      }
      // Handle vitest/* subpaths (resolve to our dist files)
      // Also handle @voidzero-dev package aliases that resolve to the same files
      const vitestSubpathMap = {
        'vitest': resolve(packageRoot, 'index.js'),
        '@voidzero-dev/vite-plus-test': resolve(packageRoot, 'index.js'),
        'vite-plus/test': resolve(packageRoot, 'index.js'),
        'vitest/node': resolve(packageRoot, 'node.js'),
        'vitest/config': resolve(packageRoot, 'config.js'),
        'vitest/internal/browser': resolve(packageRoot, 'browser.js'),
        'vitest/runners': resolve(packageRoot, 'runners.js'),
        'vitest/suite': resolve(packageRoot, 'suite.js'),
        'vitest/environments': resolve(packageRoot, 'environments.js'),
        'vitest/coverage': resolve(packageRoot, 'coverage.js'),
        'vitest/reporters': resolve(packageRoot, 'reporters.js'),
        'vitest/snapshot': resolve(packageRoot, 'snapshot.js'),
        'vitest/mocker': resolve(packageRoot, 'mocker.js'),
        // Browser providers - resolve to our bundled @vitest/browser-* packages
        'vitest/browser-playwright': resolve(packageRoot, '@vitest/browser-playwright/index.js'),
        'vitest/browser-webdriverio': resolve(packageRoot, '@vitest/browser-webdriverio/index.js'),
        'vitest/browser-preview': resolve(packageRoot, '@vitest/browser-preview/index.js'),
      };
      if (vitestSubpathMap[id]) {
        return vitestSubpathMap[id];
      }
      // Handle @voidzero-dev/vite-plus-test/* subpaths (same as vitest/*)
      if (id.startsWith('@voidzero-dev/vite-plus-test/')) {
        const subpath = id.slice('@voidzero-dev/vite-plus-test/'.length);
        const vitestEquiv = 'vitest/' + subpath;
        if (vitestSubpathMap[vitestEquiv]) {
          return vitestSubpathMap[vitestEquiv];
        }
      }
      // Handle vite-plus/test/* subpaths (CLI package paths, same as vitest/*)
      if (id.startsWith('vite-plus/test/')) {
        const subpath = id.slice('vite-plus/test/'.length);
        const vitestEquiv = 'vitest/' + subpath;
        if (vitestSubpathMap[vitestEquiv]) {
          return vitestSubpathMap[vitestEquiv];
        }
      }
      // Handle @vitest/* packages (resolve to our copied files)
      const vendorMap = {
      ${mappingEntries}
      };
      if (vendorMap[id]) {
        return vendorMap[id];
      }
    }
  }`;

  // Find BrowserPlugin return array and inject plugin
  const pluginArrayPattern = /(return \[)(\n +\{\n +enforce: "pre",\n +name: "vitest:browser",)/;
  if (pluginArrayPattern.test(content)) {
    content = content.replace(pluginArrayPattern, `$1\n    ${vendorAliasesPlugin},$2`);
    console.log('  Injected vitest:vendor-aliases plugin');
  } else {
    throw new Error(
      'Failed to inject vendor-aliases plugin in @vitest/browser/index.js: pattern not found. ' +
        'This likely means vitest code has changed and the patch needs to be updated.',
    );
  }

  // 2. Patch exclude list to add native deps
  // Pattern: const exclude = ["vitest", ...
  const excludePattern = /(const exclude = \[)(\n?\s*"vitest",)/;
  // Exclude packages that:
  // Packages to exclude from Vite's dependency pre-bundling (optimizeDeps.exclude)
  const packagesToExclude = [
    // @vitest packages that need our resolveId plugin
    '@vitest/browser',
    '@vitest/ui',
    '@vitest/ui/reporter',
    '@vitest/mocker/node', // imports @voidzero-dev/vite-plus-core

    // Our package aliases - preserve module identity with init scripts
    // This ensures both init scripts (loaded via /@fs/) and tests use the same page singleton
    '@voidzero-dev/vite-plus-test',
    '@voidzero-dev/vite-plus-test/browser',
    '@voidzero-dev/vite-plus-test/browser/context',
    'vite-plus/test',
    'vite-plus/test/browser',
    'vite-plus/test/browser/context',

    // Node.js only packages
    'vite',
    '@voidzero-dev/vite-plus-core',
    '@voidzero-dev/vite-plus-core/module-runner',

    // Native bindings
    'lightningcss',
    '@tailwindcss/oxide',
    'tailwindcss', // pulls in @tailwindcss/oxide
  ];

  const excludeListStr = packagesToExclude.map((pkg) => `"${pkg}"`).join(',\n          ');
  const excludeReplacement = `$1\n          ${excludeListStr},$2`;
  if (excludePattern.test(content)) {
    content = content.replace(excludePattern, excludeReplacement);
    console.log('  Patched exclude list with native deps');
  } else {
    throw new Error(
      'Failed to patch exclude list in @vitest/browser/index.js: pattern not found. ' +
        'This likely means vitest code has changed and the patch needs to be updated.',
    );
  }

  // 3. Remove include patterns that reference bundled deps
  // These patterns like "vitest > expect-type" don't work with our bundled setup
  // since the deps are already bundled into vendor files
  const includePatterns = [
    '"vitest > expect-type"',
    '"vitest > @vitest/snapshot > magic-string"',
    '"vitest > @vitest/expect > chai"',
  ];
  for (const pattern of includePatterns) {
    content = content.replace(
      new RegExp(`\\s*${pattern.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')},?`, 'g'),
      '',
    );
  }
  console.log('  Removed bundled deps from include list');

  // 4. Patch BrowserContext to also handle our package aliases as fallback
  // This allows direct imports from our package without requiring vitest override
  // Supports: vitest/browser, @voidzero-dev/vite-plus-test/browser, vite-plus/test/browser
  const browserContextPattern = /if \(id === ID_CONTEXT\) \{/;
  if (browserContextPattern.test(content)) {
    content = content.replace(
      browserContextPattern,
      `if (id === ID_CONTEXT || id === "@voidzero-dev/vite-plus-test/browser" || id === "vite-plus/test/browser") {`,
    );
    console.log('  Patched BrowserContext to handle package aliases');
  } else {
    throw new Error(
      'Failed to patch BrowserContext in @vitest/browser/index.js: pattern not found. ' +
        'This likely means vitest code has changed and the patch needs to be updated.',
    );
  }

  await writeFile(browserIndexPath, content, 'utf-8');
  console.log('  Successfully patched @vitest/browser/index.js');
}

/**
 * Patch browser provider locators.js files to use browser-safe imports.
 *
 * The original files import from '../browser/index.js' which includes Node.js server code.
 * We need to change them to import from browser-safe files instead.
 *
 * Providers handled:
 *   - @vitest/browser-playwright: import { page, server } from '../browser/index.js';
 *   - @vitest/browser-webdriverio: import { page, server, utils } from '../browser/index.js';
 *   - @vitest/browser-preview: import { page, server, utils, userEvent } from '../browser/index.js';
 */
async function patchBrowserProviderLocators() {
  console.log('\nPatching browser provider locators.js files...');

  const providers = [
    { name: 'browser-playwright', extraImports: [] as string[] },
    { name: 'browser-webdriverio', extraImports: ['utils'] },
    { name: 'browser-preview', extraImports: ['utils', 'userEvent'] },
  ];

  for (const provider of providers) {
    const locatorsPath = join(distDir, `@vitest/${provider.name}/locators.js`);

    try {
      await stat(locatorsPath);
    } catch {
      console.log(`  Warning: @vitest/${provider.name}/locators.js not found, skipping`);
      continue;
    }

    let content = await readFile(locatorsPath, 'utf-8');
    let patched = false;

    // 1. Patch the vitest/browser import to separate page (from context.js) and other imports
    // After rewriteVitestImports(), the import is: import { page, server, ... } from '../browser/index.js';
    // We need:
    //   - page from '../browser/context.js' (browser-safe)
    //   - server removed (we'll use window.__vitest_worker__.config instead)
    //   - other imports (utils, userEvent) still from '../browser/index.js'

    if (provider.extraImports.length === 0) {
      // playwright: just import page from context.js
      const serverImportPattern =
        /import \{ page, server \} from ['"]\.\.\/browser\/index\.js['"];?/;
      if (serverImportPattern.test(content)) {
        content = content.replace(
          serverImportPattern,
          `import { page } from '../browser/context.js';`,
        );
        console.log(`  [${provider.name}] Changed server import to browser-safe context import`);
        patched = true;
      }
    } else {
      // webdriverio/preview: import page from context.js, keep other imports from index.js
      const extraImportsStr = provider.extraImports.join(', ');
      const importPattern = new RegExp(
        `import \\{ page, server, ${extraImportsStr} \\} from ['"]\\.\\./browser/index\\.js['"];?`,
      );
      if (importPattern.test(content)) {
        const replacement = `import { page } from '../browser/context.js';\nimport { ${extraImportsStr} } from '../browser/index.js';`;
        content = content.replace(importPattern, replacement);
        console.log(
          `  [${provider.name}] Split imports: page from context.js, {${extraImportsStr}} from index.js`,
        );
        patched = true;
      }
    }

    if (!patched) {
      console.log(`  Warning: [${provider.name}] Could not find server import to patch`);
    }

    // 2. Replace all server.config references with browser-accessible window.__vitest_worker__.config
    // This handles both:
    //   - server.config.browser.locators.testIdAttribute
    //   - server.config.browser.ui
    const serverConfigPattern = /server\.config\./g;
    const matchCount = (content.match(serverConfigPattern) || []).length;
    if (matchCount > 0) {
      content = content.replace(serverConfigPattern, `window.__vitest_worker__.config.`);
      console.log(
        `  [${provider.name}] Replaced ${matchCount} server.config references with window.__vitest_worker__.config`,
      );
    }

    await writeFile(locatorsPath, content, 'utf-8');
    console.log(`  Successfully patched @vitest/${provider.name}/locators.js`);
  }
}

/**
 * Create browser-compat.js shim that re-exports @vitest/browser compatible symbols.
 * This allows our package to be used as an override for @vitest/browser.
 */
async function createBrowserCompatShim() {
  console.log('\nCreating browser-compat shim...');

  const browserIndexPath = join(distDir, '@vitest/browser/index.js');

  try {
    await stat(browserIndexPath);
  } catch {
    console.log('  Warning: @vitest/browser/index.js not found, skipping');
    return;
  }

  const browserSymbols = [
    'resolveScreenshotPath',
    'defineBrowserProvider',
    'parseKeyDef',
    'defineBrowserCommand',
  ];

  const shimContent = `// Re-export @vitest/browser compatible symbols
// This allows this package to be used as an override for @vitest/browser
export { ${browserSymbols.join(', ')} } from './@vitest/browser/index.js';
`;

  const shimPath = join(distDir, 'browser-compat.js');
  await writeFile(shimPath, shimContent, 'utf-8');
  console.log(`  Created ${relative(projectDir, shimPath)}`);
}

/**
 * Create a browser-safe stub for module-runner.
 * The real module-runner contains Node.js-only code (process.platform, Buffer, etc.)
 * that causes browsers to hang when loaded. This stub provides empty/placeholder
 * exports so that browser code can import without errors.
 */
async function createModuleRunnerStub() {
  console.log('\nCreating browser-safe module-runner stub...');

  const stubContent = `// Browser-safe stub for module-runner
// The real module-runner contains Node.js-only code that crashes browsers
// This stub provides placeholder exports for browser compatibility

// Stub class - browser doesn't actually use these
export class EvaluatedModules {
  constructor() {
    this.idToModuleMap = new Map();
    this.fileToModulesMap = new Map();
    this.urlToIdModuleMap = new Map();
  }
  getModuleById() { return undefined; }
  getModulesByFile() { return []; }
  getModuleByUrl() { return undefined; }
  ensureModule() { return {}; }
  invalidateModule() {}
  clear() {}
}

export class ModuleRunner {
  constructor() {}
  async import() { throw new Error('ModuleRunner is not available in browser'); }
  evaluatedModules = new EvaluatedModules();
}

export class ESModulesEvaluator {
  constructor() {}
  async runExternalModule() { return {}; }
  async runViteModule() { return {}; }
}

// Stub functions
export function createDefaultImportMeta() { return {}; }
export function createNodeImportMeta() { return {}; }
export function createWebSocketModuleRunnerTransport() { return {}; }
export function normalizeModuleId(id) { return id; }

// SSR-related constants (browser doesn't use these)
export const ssrDynamicImportKey = '__vite_ssr_dynamic_import__';
export const ssrExportAllKey = '__vite_ssr_exportAll__';
export const ssrExportNameKey = '__vite_ssr_export__';
export const ssrImportKey = '__vite_ssr_import__';
export const ssrImportMetaKey = '__vite_ssr_import_meta__';
export const ssrModuleExportsKey = '__vite_ssr_exports__';
`;

  const stubPath = join(distDir, 'module-runner-stub.js');
  await writeFile(stubPath, stubContent, 'utf-8');
  console.log(`  Created ${relative(projectDir, stubPath)}`);
}

/**
 * Create a Node.js-specific entry that includes @vitest/browser symbols.
 * Browser code will use index.js (no browser-provider imports) to avoid loading Node.js code.
 * Node.js code (like @vitest/browser-playwright) will use index-node.js which includes
 * the browser symbols needed for pnpm override compatibility.
 *
 * This separation is critical because @vitest/browser/index.js imports from vitest/node,
 * which contains Node.js-only code (including __vite__injectQuery) that crashes browsers.
 */
async function createNodeEntry() {
  console.log('\nCreating Node.js-specific entry for @vitest/browser override...');

  const browserIndexPath = join(distDir, '@vitest/browser/index.js');

  try {
    await stat(browserIndexPath);
  } catch {
    console.log('  Warning: @vitest/browser/index.js not found, skipping');
    return;
  }

  const browserSymbols = [
    'resolveScreenshotPath',
    'defineBrowserProvider',
    'parseKeyDef',
    'defineBrowserCommand',
  ];

  // Create index-node.js that re-exports everything from index.js plus browser symbols
  const nodeEntry = `// Node.js-specific entry that includes @vitest/browser provider symbols
// Browser code should use index.js which doesn't pull in Node.js-only code
export * from './index.js';

// Re-export @vitest/browser symbols for pnpm override compatibility
// These are only needed when this package overrides @vitest/browser in Node.js context
export { ${browserSymbols.join(', ')} } from './@vitest/browser/index.js';
`;

  const nodeEntryPath = join(distDir, 'index-node.js');
  await writeFile(nodeEntryPath, nodeEntry, 'utf-8');
  console.log(`  Created dist/index-node.js with @vitest/browser exports`);
}

/**
 * Copy ALL files from @vitest/browser's dist to our dist.
 * The bundled code in dist/vendor/ calculates paths like:
 *   pkgRoot = resolve(import.meta.url, "../..") -> package root
 *   distRoot = resolve(pkgRoot, "dist") -> dist/
 * Then looks for client/ files at distRoot, so we copy to dist/ not dist/vendor/.
 */
async function copyBrowserClientFiles() {
  console.log('\nCopying @vitest/browser files to dist...');

  // Find @vitest/browser's dist directory
  const vitestBrowserDist = resolve(projectDir, 'node_modules/@vitest/browser/dist');

  // Check if it exists
  try {
    await stat(vitestBrowserDist);
  } catch {
    console.log('  Warning: @vitest/browser not installed, skipping');
    return;
  }

  // Copy all files from @vitest/browser/dist to our dist/
  // The bundled code at dist/vendor/ resolves paths relative to dist/
  // Use recursive directory traversal to include dotfiles (glob doesn't handle them well)
  let copiedCount = 0;

  // Rewrite imports in copied JS files to use our dist files
  // The relative path depends on the file's location relative to dist/
  function rewriteImports(content: string, destPath: string): string {
    const fileDir = parse(destPath).dir;

    // Calculate relative path from file location to vendor directory
    const vendorPath = join(distDir, 'vendor');
    let relativeToVendor = relative(fileDir, vendorPath);
    // Ensure path starts with ./ for relative imports
    if (!relativeToVendor.startsWith('.')) {
      relativeToVendor = './' + relativeToVendor;
    }

    // Calculate relative path from file location to dist directory
    let relativeToDist = relative(fileDir, distDir);
    if (!relativeToDist.startsWith('.')) {
      relativeToDist = './' + relativeToDist;
    }

    // Rewrite @vitest/* imports to use our copied @vitest files
    for (const [pkg, distPath] of Object.entries(VITEST_PACKAGE_TO_PATH)) {
      if (!pkg.startsWith('@vitest/')) {
        continue;
      }
      // Pattern: from"@vitest/runner" or from "@vitest/runner"
      const importPattern = new RegExp(`from\\s*["']${pkg.replace('/', '\\/')}["']`, 'g');
      content = content.replace(importPattern, `from"${relativeToDist}/${distPath}"`);
    }

    // Rewrite vitest/* subpath imports to use our dist files
    // These are the actual entry points for vitest's browser-safe exports
    const vitestSubpathRewrites: Record<string, string> = {
      'vitest/browser': `${relativeToDist}/context.js`, // vitest/browser exports context API
      'vitest/internal/browser': `${relativeToDist}/browser.js`,
      'vitest/runners': `${relativeToDist}/runners.js`,
    };
    for (const [specifier, destFile] of Object.entries(vitestSubpathRewrites)) {
      const importPattern = new RegExp(`from\\s*["']${specifier.replace('/', '\\/')}["']`, 'g');
      content = content.replace(importPattern, `from"${destFile}"`);
    }

    // Special handling for @vitest/browser/client -> our client.js
    // This is needed because the browser client files import from @vitest/browser/client
    const browserClientPattern = /from\s*["']@vitest\/browser\/client["']/g;
    content = content.replace(browserClientPattern, `from"${relativeToDist}/client.js"`);

    // Handle imports from ./index.js which is Node.js-only code
    // In browser context, 'server' should read from __vitest_browser_runner__ at runtime
    // Replace: import{server}from'./index.js' with a browser-safe stub
    const serverStub = `const server = {
  get browser() { return window.__vitest_browser_runner__?.config?.browser?.name; },
  get config() { return window.__vitest_browser_runner__?.config || {}; },
  get commands() { return window.__vitest_browser_runner__?.commands || {}; },
  get provider() { return window.__vitest_browser_runner__?.provider; },
};`;
    content = content.replace(
      /import\s*\{\s*server\s*\}\s*from\s*['"]\.\/index\.js['"];?/g,
      serverStub,
    );

    // Remove side-effect imports from ./index.js (Node.js-only)
    // Pattern: import'./index.js'; at the end of an import statement
    content = content.replace(/import\s*['"]\.\/index\.js['"];?/g, '');

    return content;
  }

  async function copyDirRecursive(srcDir: string, destDir: string) {
    const entries = await readdir(srcDir, { withFileTypes: true });

    for (const entry of entries) {
      const srcPath = join(srcDir, entry.name);
      const destPath = join(destDir, entry.name);

      if (entry.isDirectory()) {
        await mkdir(destPath, { recursive: true });
        await copyDirRecursive(srcPath, destPath);
      } else if (entry.isFile()) {
        // Skip if file already exists (our bundled code takes precedence)
        try {
          await stat(destPath);
          continue;
        } catch {
          // File doesn't exist, copy it
        }
        await mkdir(parse(destPath).dir, { recursive: true });

        // For JS files, rewrite imports; otherwise just copy
        if (entry.name.endsWith('.js')) {
          let content = await readFile(srcPath, 'utf-8');
          content = rewriteImports(content, destPath);
          await writeFile(destPath, content, 'utf-8');
        } else {
          await copyFile(srcPath, destPath);
        }
        copiedCount++;
      }
    }
  }

  await copyDirRecursive(vitestBrowserDist, distDir);

  // Create dummy.js for placeholder exports (matchers, utils)
  const dummyContent = '// Placeholder for browser compatibility\nexport {};\n';
  await writeFile(join(distDir, 'dummy.js'), dummyContent, 'utf-8');

  console.log(`  Copied ${copiedCount} files from @vitest/browser to dist`);

  // Create vendor stubs for browser packages that aren't bundled
  // Other dist files reference these vendor paths but we don't bundle browser packages
  // to avoid Node.js code leakage. Instead, we create stubs that re-export from actual dist files.
  console.log('  Creating vendor stubs for browser packages...');
  const browserVendorStubs = [
    {
      vendorFile: 'vitest_browser.mjs',
      // vitest/browser exports the context API (page, server, userEvent)
      content: `// Stub for browser context - re-exports from our context.js
export * from '../context.js';
`,
    },
    {
      vendorFile: 'vitest_internal_browser.mjs',
      // vitest/internal/browser is browser.js
      content: `// Stub for internal browser API - re-exports from our browser.js
export * from '../browser.js';
`,
    },
    {
      vendorFile: 'vitest_runners.mjs',
      // vitest/runners
      content: `// Stub for runners - re-exports from our runners.js
export * from '../runners.js';
`,
    },
    {
      vendorFile: 'vitest_runner.mjs',
      // @vitest/runner (note: singular, not plural like vitest_runners which is vitest/runners)
      content: `// Stub for @vitest/runner - re-exports from our copied @vitest/runner
export * from '../@vitest/runner/index.js';
`,
    },
  ];

  for (const { vendorFile, content } of browserVendorStubs) {
    const stubPath = join(distDir, 'vendor', vendorFile);
    await writeFile(stubPath, content, 'utf-8');
  }
  console.log(`  Created ${browserVendorStubs.length} vendor stubs`);
}

/**
 * Create browser/ directory at package root with context files.
 * The package exports "./browser" pointing to these files:
 *   - browser/context.js: Runtime guard (throws if used outside browser mode)
 *   - browser/context.d.ts: Re-exports types from dist/@vitest/browser/context.d.ts
 *
 * These files are NOT tracked in git (.gitignore excludes browser/)
 * but ARE included in the package (package.json files: ["browser/**"])
 */
async function createBrowserEntryFiles() {
  console.log('\nCreating browser/ entry files...');

  const browserDir = resolve(projectDir, 'browser');
  await mkdir(browserDir, { recursive: true });

  // 1. Copy context.js from @vitest/browser (runtime guard)
  const srcContextJs = resolve(projectDir, 'node_modules/@vitest/browser/context.js');
  const destContextJs = join(browserDir, 'context.js');
  await copyFile(srcContextJs, destContextJs);
  console.log('  Created browser/context.js');

  // 2. Create context.d.ts that re-exports from our bundled types
  const contextDtsContent = `// Re-export browser context types from bundled @vitest/browser package
// This provides: page, userEvent, server, commands, utils, locators, cdp, Locator, etc.
// The bundled context.d.ts has imports rewritten to point to our dist files
export * from '../dist/@vitest/browser/context.d.ts'
`;
  const destContextDts = join(browserDir, 'context.d.ts');
  await writeFile(destContextDts, contextDtsContent, 'utf-8');
  console.log('  Created browser/context.d.ts');
}

/**
 * Patch module augmentations in global.d.*.d.ts files.
 *
 * The original vitest types use module augmentation like:
 *   declare module "@vitest/expect" { interface Assertion<T> { toMatchSnapshot: ... } }
 *
 * Since we bundle @vitest/* packages inside dist/@vitest/*, the bare specifier
 * "@vitest/expect" doesn't exist as a package for consumers. This breaks the
 * module augmentation - TypeScript can't find @vitest/expect to augment.
 *
 * The fix has two parts:
 * 1. Change module augmentation to use relative paths that TypeScript CAN resolve:
 *      declare module "../@vitest/expect/index.js" { ... }
 * 2. Merge augmented interface/type definitions into the target .d.ts files so that
 *    downstream DTS bundlers (rolldown) can resolve them without cross-file augmentation.
 */
async function patchModuleAugmentations() {
  console.log('\nPatching module augmentations in global.d.*.d.ts files...');

  const chunksDir = join(distDir, 'chunks');
  const globalDtsFiles: string[] = [];

  // Find all global.d.*.d.ts files
  for await (const file of fsGlob(join(chunksDir, 'global.d.*.d.ts'))) {
    globalDtsFiles.push(file);
  }

  if (globalDtsFiles.length === 0) {
    console.log('  No global.d.*.d.ts files found');
    return;
  }

  // Module augmentation mappings: bare specifier -> [relative path, target .d.ts file]
  const augmentationMappings: Record<string, { relativePath: string; targetFile: string }> = {
    '@vitest/expect': {
      relativePath: '../@vitest/expect/index.js',
      targetFile: join(distDir, '@vitest/expect/index.d.ts'),
    },
    '@vitest/runner': {
      relativePath: '../@vitest/runner/index.js',
      targetFile: join(distDir, '@vitest/runner/utils.d.ts'),
    },
  };

  for (const file of globalDtsFiles) {
    let content = await readFile(file, 'utf-8');
    let modified = false;

    for (const [bareSpecifier, { relativePath, targetFile }] of Object.entries(
      augmentationMappings,
    )) {
      const oldPattern = `declare module "${bareSpecifier}"`;

      // Extract the augmentation block content using brace matching
      const startIdx = content.indexOf(oldPattern);
      const braceStart = startIdx !== -1 ? content.indexOf('{', startIdx) : -1;
      if (braceStart === -1) {
        continue;
      }

      let depth = 0;
      let braceEnd = -1;
      for (let i = braceStart; i < content.length; i++) {
        if (content[i] === '{') {
          depth++;
        } else if (content[i] === '}') {
          depth--;
          if (depth === 0) {
            braceEnd = i;
            break;
          }
        }
      }
      if (braceEnd === -1) {
        continue;
      }

      const innerContent = content.slice(braceStart + 1, braceEnd).trim();

      // Merge only NEW type declarations into the target .d.ts file.
      // Interfaces that already exist (e.g., ExpectStatic, Assertion, MatcherState) must NOT
      // be re-declared, as that would shadow extends clauses and break call signatures.
      if (innerContent && existsSync(targetFile)) {
        let targetContent = await readFile(targetFile, 'utf-8');

        // Extract individual interface blocks from the augmentation content
        const interfaceRegex = /(?:export\s+)?interface\s+(\w+)(?:<[^>]*>)?\s*\{/g;
        let match;
        const newDeclarations: string[] = [];

        while ((match = interfaceRegex.exec(innerContent)) !== null) {
          const name = match[1];
          // Only merge if this interface does NOT already exist in the target file.
          // Check both direct declarations (interface Name) and re-exports (export type { Name }).
          const hasDirectDecl = new RegExp(`\\binterface\\s+${name}\\b`).test(targetContent);
          const exportTypeMatch = targetContent.match(/export\s+type\s*\{([^}]*)\}/);
          const isReExported =
            exportTypeMatch != null && new RegExp(`\\b${name}\\b`).test(exportTypeMatch[1]);
          if (hasDirectDecl || isReExported) {
            console.log(
              `  Skipped existing interface "${name}" (already in ${basename(targetFile)})`,
            );
            continue;
          }

          // Extract this interface block using brace matching
          const ifaceStart = match.index;
          const ifaceBraceStart = innerContent.indexOf('{', ifaceStart);
          let ifaceDepth = 0;
          let ifaceBraceEnd = -1;
          for (let i = ifaceBraceStart; i < innerContent.length; i++) {
            if (innerContent[i] === '{') {
              ifaceDepth++;
            } else if (innerContent[i] === '}') {
              ifaceDepth--;
              if (ifaceDepth === 0) {
                ifaceBraceEnd = i;
                break;
              }
            }
          }
          if (ifaceBraceEnd === -1) {
            continue;
          }

          let block = innerContent.slice(ifaceStart, ifaceBraceEnd + 1).trim();
          if (!block.startsWith('export')) {
            block = `export ${block}`;
          }
          newDeclarations.push(block);
          console.log(`  Merged new interface "${name}" into ${basename(targetFile)}`);
        }

        if (newDeclarations.length > 0) {
          targetContent += `\n// Merged from module augmentation: declare module "${bareSpecifier}"\n${newDeclarations.join('\n')}\n`;
          await writeFile(targetFile, targetContent, 'utf-8');
        }
      }

      // Rewrite declare module path to relative
      const newPattern = `declare module "${relativePath}"`;
      content = content.replaceAll(oldPattern, newPattern);
      modified = true;
      console.log(`  Patched: ${bareSpecifier} -> ${relativePath} in ${basename(file)}`);
    }

    if (modified) {
      await writeFile(file, content, 'utf-8');
    }
  }

  // Re-export BrowserCommands from context.d.ts (imported but not exported)
  const contextDtsPath = join(distDir, '@vitest/browser/context.d.ts');
  if (existsSync(contextDtsPath)) {
    let content = await readFile(contextDtsPath, 'utf-8');
    if (
      content.includes('BrowserCommands') &&
      !content.match(/export\s+(type\s+)?\{[^}]*BrowserCommands/)
    ) {
      content += '\nexport type { BrowserCommands };\n';
      await writeFile(contextDtsPath, content, 'utf-8');
      console.log('  Added BrowserCommands re-export to context.d.ts');
    }
  }

  // Validate: ensure no duplicate top-level interface declarations were introduced by merging.
  // Only count interfaces at the module scope (not nested inside declare global, namespace, etc.)
  for (const [bareSpecifier, { targetFile }] of Object.entries(augmentationMappings)) {
    if (!existsSync(targetFile)) {
      continue;
    }
    const finalContent = await readFile(targetFile, 'utf-8');

    // Extract top-level interface names by tracking brace depth
    const topLevelInterfaces: string[] = [];
    let depth = 0;
    for (let i = 0; i < finalContent.length; i++) {
      if (finalContent[i] === '{') {
        depth++;
      } else if (finalContent[i] === '}') {
        depth--;
      } else if (depth === 0) {
        const remaining = finalContent.slice(i);
        const m = remaining.match(/^interface\s+(\w+)/);
        if (m) {
          topLevelInterfaces.push(m[1]);
          i += m[0].length - 1;
        }
      }
    }

    const counts = new Map<string, number>();
    for (const name of topLevelInterfaces) {
      counts.set(name, (counts.get(name) || 0) + 1);
    }

    for (const [name, count] of counts) {
      if (count > 1) {
        throw new Error(
          `Interface "${name}" is declared ${count} times at top level in ${basename(targetFile)}. ` +
            `Module augmentation merge for "${bareSpecifier}" likely created a duplicate ` +
            `declaration that will shadow extends clauses and break type signatures.`,
        );
      }
    }
  }
}

/**
 * Add triple-slash reference to @types/chai in @vitest/expect types.
 *
 * The @vitest/expect types use the Chai namespace (e.g., Chai.Assertion) which
 * is defined in @types/chai. Without a reference directive, TypeScript won't
 * automatically find the Chai types, causing the `not` property and other
 * chai-specific features to be missing from the Assertion interface.
 */
async function patchChaiTypeReference() {
  console.log('\nAdding @types/chai reference to @vitest/expect types...');

  const expectIndexDts = join(distDir, '@vitest/expect/index.d.ts');

  let content = await readFile(expectIndexDts, 'utf-8');

  // Check if reference already exists
  if (content.includes('/// <reference types="chai"')) {
    console.log('  Reference already exists, skipping');
    return;
  }

  // Add triple-slash reference at the top
  content = `/// <reference types="chai" />\n${content}`;

  await writeFile(expectIndexDts, content, 'utf-8');
  console.log('  Added /// <reference types="chai" /> to @vitest/expect/index.d.ts');
}

/**
 * Patch the vitest mocker to recognize @voidzero-dev packages as valid sources for vi/vitest.
 *
 * The mocker's hoistMocks function checks if `vi` is imported from the 'vitest' module.
 * When users import from 'vite-plus/test' instead, the mocker doesn't
 * recognize it and throws "There are some problems in resolving the mocks API".
 *
 * This patch modifies the equality check to also accept our package names:
 * - vite-plus/test
 * - @voidzero-dev/vite-plus-test
 */
async function patchMockerHoistedModule() {
  console.log('\nPatching vitest mocker to recognize @voidzero-dev packages...');

  // The hoistedModule check may be in node.js or chunk-hoistMocks.js depending on the vitest version
  const candidateFiles = [
    join(distDir, '@vitest/mocker/node.js'),
    join(distDir, '@vitest/mocker/chunk-hoistMocks.js'),
  ];

  // Find and replace the hoistedModule check
  // Original: if (hoistedModule === source) {
  // New: if (hoistedModule === source || source === "vite-plus/test" || source === "@voidzero-dev/vite-plus-test") {
  const originalCheck = 'if (hoistedModule === source) {';
  const newCheck =
    'if (hoistedModule === source || source === "vite-plus/test" || source === "@voidzero-dev/vite-plus-test") {';

  let patched = false;
  for (const candidatePath of candidateFiles) {
    let content: string;
    try {
      content = await readFile(candidatePath, 'utf-8');
    } catch {
      continue;
    }
    if (content.includes(originalCheck)) {
      content = content.replace(originalCheck, newCheck);
      await writeFile(candidatePath, content, 'utf-8');
      console.log(`  Patched hoistMocks to recognize @voidzero-dev packages in ${candidatePath}`);
      patched = true;
      break;
    }
  }

  if (!patched) {
    throw new Error(
      'Could not find hoistedModule check to patch in @vitest/mocker. ' +
        'This likely means vitest code has changed and the patch needs to be updated.',
    );
  }
}

/**
 * Patch vitest's ModuleRunnerTransform plugin to automatically add known
 * packages that use `expect.extend()` internally to `server.deps.inline`.
 *
 * When third-party libraries (e.g., @testing-library/jest-dom) call
 * `require('vitest').expect.extend(matchers)`, the npm override causes
 * a separate module instance to be created, so matchers are registered
 * on a different `chai` instance than the one used by the test runner.
 *
 * By inlining these packages via `server.deps.inline`, the Vite module
 * runner processes them through its transform pipeline, ensuring they
 * share the same module instance as the test runner.
 *
 * See: https://github.com/voidzero-dev/vite-plus/issues/897
 */
async function patchServerDepsInline() {
  console.log('\nPatching server.deps.inline for expect.extend compatibility...');

  let cliApiChunk: string | undefined;
  for await (const chunk of fsGlob(join(distDir, 'chunks/cli-api.*.js'))) {
    cliApiChunk = chunk;
    break;
  }

  if (!cliApiChunk) {
    throw new Error('cli-api chunk not found for patchServerDepsInline');
  }

  let content = await readFile(cliApiChunk, 'utf-8');

  // Packages that internally call expect.extend() and break under npm override.
  // These must be inlined so they share the same vitest module instance.
  const inlinePackages = ['@testing-library/jest-dom', '@storybook/test', 'jest-extended'];

  // Find the configResolved handler in ModuleRunnerTransform (vitest:environments-module-runner)
  // and inject our inline packages after the existing server.deps.inline logic.
  const original = `if (external.length) {
          testConfig.server.deps.external ??= [];
          testConfig.server.deps.external.push(...external);
        }`;

  const patched = `if (external.length) {
          testConfig.server.deps.external ??= [];
          testConfig.server.deps.external.push(...external);
        }
        // Auto-inline packages that use expect.extend() internally (#897)
        // Only inline packages that are actually installed in the project.
        if (testConfig.server.deps.inline !== true) {
          testConfig.server.deps.inline ??= [];
          if (Array.isArray(testConfig.server.deps.inline)) {
            const _require = createRequire(config.root + "/package.json");
            const autoInline = ${JSON.stringify(inlinePackages)};
            for (const pkg of autoInline) {
              if (testConfig.server.deps.inline.includes(pkg)) continue;
              try {
                _require.resolve(pkg);
                testConfig.server.deps.inline.push(pkg);
              } catch {
                // Package not installed in the project — skip silently
              }
            }
          }
        }`;

  if (!content.includes(original)) {
    throw new Error(
      'Could not find server.deps.external pattern in ' +
        cliApiChunk +
        '. This likely means vitest code has changed and the patch needs to be updated.',
    );
  }

  content = content.replace(original, patched);
  await writeFile(cliApiChunk, content, 'utf-8');
  console.log(`  Added auto-inline for: ${inlinePackages.join(', ')}`);
}

/**
 * Create /plugins/* exports for all copied @vitest/* packages.
 * This allows pnpm overrides to redirect @vitest/* imports to our copied versions.
 * e.g., @vitest/runner -> vitest/plugins/runner
 *       @vitest/utils/error -> vitest/plugins/utils-error
 */
async function createPluginExports() {
  console.log('\nCreating plugin exports for @vitest/* packages...');

  const pluginsDir = join(distDir, 'plugins');
  // Clean up stale plugin files from previous builds
  await rm(pluginsDir, { recursive: true, force: true });
  await mkdir(pluginsDir, { recursive: true });

  const createdExports: Array<{ exportPath: string; shimFile: string }> = [];

  for (const [pkg, distPath] of Object.entries(VITEST_PACKAGE_TO_PATH)) {
    // Only create exports for @vitest/* packages
    if (!pkg.startsWith('@vitest/')) {
      continue;
    }
    // Convert @vitest/runner -> runner, @vitest/utils/error -> utils-error
    // @vitest/utils/source-map/node -> utils-source-map-node
    const exportName = pkg.replace('@vitest/', '').replaceAll('/', '-');
    const shimFileName = `${exportName}.mjs`;
    const shimPath = join(pluginsDir, shimFileName);

    // Create the shim file that re-exports everything from @vitest/
    const shimContent = `// Re-export ${pkg} from copied @vitest package
export * from '../${distPath}';
`;

    await writeFile(shimPath, shimContent, 'utf-8');
    createdExports.push({
      exportPath: `./plugins/${exportName}`,
      shimFile: `./dist/plugins/${shimFileName}`,
    });
    console.log(`  Created plugins/${shimFileName} -> ${distPath}`);
  }

  return createdExports;
}

/**
 * Validate that all external dependencies in dist are listed in package.json
 */
async function validateExternalDeps() {
  console.log('\nValidating external dependencies...');

  // Collect all declared dependencies
  const declaredDeps = new Set<string>([
    ...Object.keys(pkg.dependencies || {}),
    ...Object.keys(pkg.peerDependencies || {}),
  ]);

  // Also include self-references
  declaredDeps.add(pkg.name);
  declaredDeps.add('vitest'); // Self-reference via vitest name

  // Collect all external specifiers from ALL dist files (including vendor)
  const externalSpecifiers = new Map<string, Set<string>>(); // specifier -> files

  const allJsFiles = fsGlob(join(distDir, '**/*.{js,mjs,cjs}'));

  for await (const file of allJsFiles) {
    const content = await readFile(file, 'utf-8');
    const isCjs = file.endsWith('.cjs');

    // Parse with oxc-parser
    const result = parseSync(file, content, {
      sourceType: isCjs ? 'script' : 'module',
    });

    const specifiers = new Set<string>();

    // Collect ESM static imports
    for (const imp of result.module.staticImports) {
      specifiers.add(imp.moduleRequest.value);
    }

    // Collect ESM static exports (re-exports)
    for (const exp of result.module.staticExports) {
      for (const entry of exp.entries) {
        if (entry.moduleRequest) {
          specifiers.add(entry.moduleRequest.value);
        }
      }
    }

    // Collect dynamic imports (only string literals)
    for (const dynImp of result.module.dynamicImports) {
      const rawText = content.slice(dynImp.moduleRequest.start, dynImp.moduleRequest.end);
      if (
        (rawText.startsWith("'") && rawText.endsWith("'")) ||
        (rawText.startsWith('"') && rawText.endsWith('"'))
      ) {
        specifiers.add(rawText.slice(1, -1));
      }
    }

    // For CJS files, also scan for require() calls
    if (isCjs) {
      const requireRegex = /require\s*\(\s*['"]([^'"]+)['"]\s*\)/g;
      let match;
      while ((match = requireRegex.exec(content)) !== null) {
        specifiers.add(match[1]);
      }
    }

    // Filter and record external specifiers
    for (const specifier of specifiers) {
      // Skip relative paths
      if (specifier.startsWith('.') || specifier.startsWith('/')) {
        continue;
      }
      // Skip node built-ins
      if (NODE_BUILTINS.has(specifier)) {
        continue;
      }
      // Skip Node.js subpath imports
      if (specifier.startsWith('#')) {
        continue;
      }

      // Get the package name (handle scoped packages and subpaths)
      const packageName = getPackageName(specifier);
      if (!packageName) {
        continue;
      }

      // Check if it's declared
      if (declaredDeps.has(packageName)) {
        continue;
      }
      // Check if it's in the blocklist (intentionally external)
      if (EXTERNAL_BLOCKLIST.has(packageName) || EXTERNAL_BLOCKLIST.has(specifier)) {
        continue;
      }

      // Record undeclared external
      if (!externalSpecifiers.has(specifier)) {
        externalSpecifiers.set(specifier, new Set());
      }
      externalSpecifiers.get(specifier)!.add(relative(distDir, file));
    }
  }

  if (externalSpecifiers.size === 0) {
    console.log('  ✓ All external dependencies are declared in package.json');
    return;
  }

  // Group by package name
  const byPackage = new Map<string, Set<string>>();
  for (const [specifier, _files] of externalSpecifiers) {
    const packageName = getPackageName(specifier)!;
    if (!byPackage.has(packageName)) {
      byPackage.set(packageName, new Set());
    }
    byPackage.get(packageName)!.add(specifier);
  }

  console.log(`\n  ⚠ Found ${byPackage.size} undeclared external dependencies:\n`);
  for (const [packageName, specifiers] of byPackage.entries()) {
    const files = externalSpecifiers.get([...specifiers][0])!;
    console.log(`    ${packageName}`);
    for (const specifier of specifiers) {
      if (specifier !== packageName) {
        console.log(`      - ${specifier}`);
      }
    }
    console.log(
      `      (used in: ${[...files].slice(0, 3).join(', ')}${files.size > 3 ? '...' : ''})`,
    );
  }
}

/**
 * Extract the package name from a specifier (handles scoped packages and subpaths)
 */
function getPackageName(specifier: string): string | null {
  // Scoped package: @scope/name or @scope/name/subpath
  if (specifier.startsWith('@')) {
    const parts = specifier.split('/');
    if (parts.length >= 2) {
      return `${parts[0]}/${parts[1]}`;
    }
    return null;
  }
  // Regular package: name or name/subpath
  const parts = specifier.split('/');
  return parts[0] || null;
}
