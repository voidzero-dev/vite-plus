import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { createBuildCommand, NapiCli } from '@napi-rs/cli';
import { format, formatEmbeddedCode } from 'oxfmt';
import {
  createCompilerHost,
  createProgram,
  formatDiagnostics,
  parseJsonSourceFileConfigFileContent,
  readJsonConfigFile,
  sys,
} from 'typescript';

const projectDir = dirname(fileURLToPath(import.meta.url));
const TEST_PACKAGE_NAME = '@voidzero-dev/vite-plus-test';
const CORE_PACKAGE_NAME = '@voidzero-dev/vite-plus-core';

await buildCli();
await buildNapiBinding();
await syncCorePackageExports();
await syncTestPackageExports();

async function buildNapiBinding() {
  const buildCommand = createBuildCommand(process.argv.slice(2));
  const passedInOptions = buildCommand.getOptions();

  const cli = new NapiCli();
  const { task } = await cli.build({
    ...passedInOptions,
    packageJsonPath: '../package.json',
    cwd: 'binding',
    platform: true,
    release: process.env.VITE_PLUS_CLI_DEBUG !== '1',
    esm: true,
  });

  const outputs = await task;
  const fmtConfigPath = join(projectDir, '../../node_modules/.vite/task-cache/.oxfmtrc.json');
  if (!existsSync(fmtConfigPath)) {
    const viteConfig = await import('../../vite.config');
    mkdirSync(dirname(fmtConfigPath), { recursive: true });
    writeFileSync(fmtConfigPath, JSON.stringify(viteConfig.default.fmt, null, 2));
  }
  await format(
    [
      '-c',
      '../../node_modules/.vite/task-cache/.oxfmtrc.json',
      ...outputs.filter((o) => o.kind !== 'node').map((o) => o.path),
    ],
    formatEmbeddedCode,
  );
}

async function buildCli() {
  const tsconfig = readJsonConfigFile(join(projectDir, 'tsconfig.json'), sys.readFile);

  const { options, fileNames } = parseJsonSourceFileConfigFileContent(tsconfig, sys, projectDir);

  const host = createCompilerHost(options);

  const program = createProgram({
    rootNames: fileNames,
    options,
    host,
  });

  const { diagnostics } = program.emit();

  if (diagnostics.length > 0) {
    console.error(formatDiagnostics(diagnostics, host));
    process.exit(1);
  }
}

/**
 * Sync Vite core exports from @voidzero-dev/vite-plus-core to @voidzero-dev/vite-plus
 *
 * This creates shim files for:
 * - ./client (types only)
 * - ./module-runner
 * - ./internal
 * - ./dist/client/* (wildcard)
 * - ./types/* (wildcard, types only)
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
  if (existsSync(coreClientDir)) {
    const { readdirSync } = await import('node:fs');
    for (const file of readdirSync(coreClientDir)) {
      const shimPath = join(clientDir, file);
      if (file.endsWith('.js')) {
        await writeFile(shimPath, `export * from '${CORE_PACKAGE_NAME}/dist/client/${file}';\n`);
      } else if (file.endsWith('.d.ts')) {
        await writeFile(
          shimPath,
          `export * from '${CORE_PACKAGE_NAME}/dist/client/${file.replace('.d.ts', '')}';\n`,
        );
      } else {
        // Copy non-JS/TS files directly (e.g., CSS, source maps)
        const { copyFileSync } = await import('node:fs');
        copyFileSync(join(coreClientDir, file), shimPath);
      }
    }
  }

  // Create ./types/* shims by reading core's dist/vite/types files
  console.log('  Creating ./types/*');
  const coreTypesDir = join(projectDir, '../core/dist/vite/types');
  if (existsSync(coreTypesDir)) {
    const { readdirSync, statSync } = await import('node:fs');
    await syncTypesDir(coreTypesDir, typesDir, '');
  }

  console.log('\nSynced core package exports');
}

async function syncTypesDir(srcDir: string, destDir: string, relativePath: string) {
  const { readdirSync, statSync } = await import('node:fs');
  const entries = readdirSync(srcDir);

  for (const entry of entries) {
    const srcPath = join(srcDir, entry);
    const destPath = join(destDir, entry);
    const entryRelPath = relativePath ? `${relativePath}/${entry}` : entry;

    if (statSync(srcPath).isDirectory()) {
      // Skip internal directory - it's blocked by exports
      if (entry === 'internal') continue;

      await mkdir(destPath, { recursive: true });
      await syncTypesDir(srcPath, destPath, entryRelPath);
    } else if (entry.endsWith('.d.ts')) {
      // Create shim that re-exports from core - must include .d.ts extension for wildcard exports
      // Use 'export type *' since we're re-exporting from a .d.ts file
      await writeFile(
        destPath,
        `export type * from '${CORE_PACKAGE_NAME}/types/${entryRelPath}';\n`,
      );
    }
  }
}

/**
 * Sync exports from @voidzero-dev/vite-plus-test to @voidzero-dev/vite-plus
 *
 * This function reads the test package's exports and creates shim files that
 * re-export everything under the ./test/* subpath. This allows users to import
 * from @voidzero-dev/vite-plus/test/* instead of @voidzero-dev/vite-plus-test/*.
 */
async function syncTestPackageExports() {
  console.log('\nSyncing test package exports...');

  const testPkgPath = join(projectDir, '../test/package.json');
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

    // Create shim files and build export entry
    const shimExport = await createShimForExport(exportPath, exportValue, testDistDir);
    if (shimExport) {
      generatedExports[cliExportPath] = shimExport;
      console.log(`  Created ${cliExportPath}`);
    }
  }

  // Update CLI package.json
  await updateCliPackageJson(cliPkgPath, generatedExports);

  console.log(`\nSynced ${Object.keys(generatedExports).length} exports from test package`);
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
 * Create shim file(s) for a single export and return the export entry for package.json
 */
async function createShimForExport(
  exportPath: string,
  exportValue: unknown,
  distDir: string,
): Promise<ExportValue | null> {
  // Determine the import specifier for the test package
  const testImportSpecifier =
    exportPath === '.' ? TEST_PACKAGE_NAME : `${TEST_PACKAGE_NAME}${exportPath.slice(1)}`;

  // Convert export path to file path: ./foo/bar -> foo/bar, . -> index
  const shimBaseName = exportPath === '.' ? 'index' : exportPath.slice(2);
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
      // Include side-effect import to preserve module augmentations (e.g., toMatchSnapshot on Assertion)
      await writeFile(
        dtsPath,
        `import '${testImportSpecifier}';\nexport * from '${testImportSpecifier}';\n`,
      );
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
      );
    }

    // Simple object with types/default
    const result: ExportValue = {};

    if (value.types && typeof value.types === 'string') {
      const dtsPath = join(shimDirForFile, `${baseFileName}.d.ts`);
      // Include side-effect import to preserve module augmentations (e.g., toMatchSnapshot on Assertion)
      await writeFile(
        dtsPath,
        `import '${testImportSpecifier}';\nexport * from '${testImportSpecifier}';\n`,
      );
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
 */
async function createConditionalShim(
  value: Record<string, unknown>,
  testImportSpecifier: string,
  shimDir: string,
  baseFileName: string,
  shimBaseName: string,
): Promise<ExportValue> {
  const result: ExportValue = {};

  // Handle top-level types (flat structure like { types, require, default })
  if (value.types && typeof value.types === 'string' && !value.import) {
    const dtsPath = join(shimDir, `${baseFileName}.d.ts`);
    // Include side-effect import to preserve module augmentations (e.g., toMatchSnapshot on Assertion)
    await writeFile(
      dtsPath,
      `import '${testImportSpecifier}';\nexport * from '${testImportSpecifier}';\n`,
    );
    (result as Record<string, string>).types = `./dist/test/${shimBaseName}.d.ts`;
  }

  // Handle top-level default (flat structure, only when no import condition)
  if (value.default && typeof value.default === 'string' && !value.import) {
    const jsPath = join(shimDir, `${baseFileName}.js`);
    await writeFile(jsPath, `export * from '${testImportSpecifier}';\n`);
    (result as Record<string, string>).default = `./dist/test/${shimBaseName}.js`;
  }

  // Handle import condition
  if (value.import) {
    const importValue = value.import as Record<string, unknown>;

    if (typeof importValue === 'string') {
      const jsPath = join(shimDir, `${baseFileName}.js`);
      await writeFile(jsPath, `export * from '${testImportSpecifier}';\n`);
      (result as Record<string, unknown>).import = `./dist/test/${shimBaseName}.js`;
    } else if (typeof importValue === 'object' && importValue !== null) {
      const importResult: Record<string, string> = {};

      if (importValue.types && typeof importValue.types === 'string') {
        const dtsPath = join(shimDir, `${baseFileName}.d.ts`);
        // Include side-effect import to preserve module augmentations (e.g., toMatchSnapshot on Assertion)
        await writeFile(
          dtsPath,
          `import '${testImportSpecifier}';\nexport * from '${testImportSpecifier}';\n`,
        );
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

      (result as Record<string, unknown>).import = importResult;
    }
  }

  // Handle require condition
  if (value.require) {
    const requireValue = value.require as Record<string, unknown>;

    if (typeof requireValue === 'string') {
      const cjsPath = join(shimDir, `${baseFileName}.cjs`);
      await writeFile(cjsPath, `module.exports = require('${testImportSpecifier}');\n`);
      (result as Record<string, unknown>).require = `./dist/test/${shimBaseName}.cjs`;
    } else if (typeof requireValue === 'object' && requireValue !== null) {
      const requireResult: Record<string, string> = {};

      if (requireValue.types && typeof requireValue.types === 'string') {
        const dctsPath = join(shimDir, `${baseFileName}.d.cts`);
        // Include side-effect import to preserve module augmentations (e.g., toMatchSnapshot on Assertion)
        await writeFile(
          dctsPath,
          `import '${testImportSpecifier}';\nexport * from '${testImportSpecifier}';\n`,
        );
        requireResult.types = `./dist/test/${shimBaseName}.d.cts`;
      }

      if (requireValue.default && typeof requireValue.default === 'string') {
        const cjsPath = join(shimDir, `${baseFileName}.cjs`);
        await writeFile(cjsPath, `module.exports = require('${testImportSpecifier}');\n`);
        requireResult.default = `./dist/test/${shimBaseName}.cjs`;
      }

      (result as Record<string, unknown>).require = requireResult;
    }
  }

  return result;
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

  await writeFile(pkgPath, JSON.stringify(pkg, null, 2) + '\n');
}
