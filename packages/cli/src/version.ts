import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { styleText } from 'node:util';

import { VITE_PLUS_NAME } from './global-utils/constants.js';
import { detectPackageMetadata } from './global-utils/package.js';
import { pkgRoot } from './global-utils/path.js';
import { getVitePlusHeader, headline, log } from './global-utils/terminal.js';

const require = createRequire(import.meta.url);

interface PackageJson {
  version: string;
  bundledVersions?: Record<string, string>;
}

interface LocalPackageMetadata {
  name: string;
  version: string;
  path: string;
}

interface ToolVersionSpec {
  command: string;
  displayName: string;
  packageName: string;
  bundledVersionKey?: string;
  fallbackPackageJson?: string;
}

function getGlobalVersion(): string {
  const pkg: PackageJson = require(path.join(pkgRoot, 'package.json'));
  return pkg.version;
}

function getLocalMetadata(cwd: string): LocalPackageMetadata | null {
  return detectPackageMetadata(cwd, VITE_PLUS_NAME) ?? null;
}

function readPackageJsonFromPath(packageJsonPath: string): PackageJson | null {
  try {
    return JSON.parse(fs.readFileSync(packageJsonPath, 'utf8')) as PackageJson;
  } catch {
    return null;
  }
}

function resolvePackageJson(packageName: string, baseDir: string): PackageJson | null {
  try {
    const packageJsonPath = require.resolve(`${packageName}/package.json`, {
      paths: [baseDir],
    });
    return readPackageJsonFromPath(packageJsonPath);
  } catch {
    return null;
  }
}

function resolveToolVersion(tool: ToolVersionSpec, localPackagePath: string): string | null {
  const pkg = resolvePackageJson(tool.packageName, localPackagePath);
  const bundledVersion = tool.bundledVersionKey
    ? (pkg?.bundledVersions?.[tool.bundledVersionKey] ?? null)
    : null;
  if (bundledVersion) {
    return bundledVersion;
  }
  const version = pkg?.version ?? null;
  if (version) {
    return version;
  }
  if (tool.fallbackPackageJson) {
    const fallbackPath = path.join(localPackagePath, tool.fallbackPackageJson);
    return readPackageJsonFromPath(fallbackPath)?.version ?? null;
  }
  return null;
}

function formatToolVersion(tool: ToolVersionSpec, version: string | null): string {
  return `${tool.displayName} ${version ? `v${version}` : `Not found`}`;
}

const columnWidth = 15;
const getColumnWidth = (label: string) => Math.max(1, columnWidth - label.length);

/**
 * Print version information
 */
export async function printVersion(cwd: string) {
  const globalVersion = getGlobalVersion();
  const localMetadata = getLocalMetadata(cwd);
  const localVersion = localMetadata?.version ?? null;

  log((await getVitePlusHeader()) + '\n');
  log(headline('vp Versions:'));
  log(`  ${styleText('bold', 'Global:')}${' '.repeat(getColumnWidth('Global:'))}v${globalVersion}`);
  if (localVersion) {
    log(`  ${styleText('bold', 'Local:')}${' '.repeat(getColumnWidth('Local:'))}v${localVersion}`);
  } else {
    log(`  ${styleText('bold', 'Local:')}${' '.repeat(getColumnWidth('Local:'))}Not installed`);
  }

  if (!localMetadata) {
    return;
  }

  const tools: ToolVersionSpec[] = [
    {
      command: 'vite',
      displayName: 'vite',
      packageName: '@voidzero-dev/vite-plus-core',
      bundledVersionKey: 'vite',
    },
    {
      command: 'rolldown',
      displayName: 'rolldown',
      packageName: '@voidzero-dev/vite-plus-core',
      bundledVersionKey: 'rolldown',
    },
    {
      command: 'test',
      displayName: 'vitest',
      packageName: '@voidzero-dev/vite-plus-test',
      bundledVersionKey: 'vitest',
    },
    {
      command: 'fmt',
      displayName: 'oxfmt',
      packageName: 'oxfmt',
    },
    {
      command: 'lint',
      displayName: 'oxlint',
      packageName: 'oxlint',
    },
    {
      command: 'tsgolint',
      displayName: 'oxlint-tsgolint',
      packageName: 'oxlint-tsgolint',
    },
    {
      command: 'pack',
      displayName: 'tsdown',
      packageName: '@voidzero-dev/vite-plus-core',
      bundledVersionKey: 'tsdown',
    },
  ];

  const resolvedTools = tools.map((tool) => ({
    tool,
    version: resolveToolVersion(tool, localMetadata.path),
  }));
  if (resolvedTools.some(({ tool, version }) => tool.bundledVersionKey && !version)) {
    return;
  }

  log('');
  log(headline('Bundled with vite-plus:'));
  for (const { tool, version } of resolvedTools) {
    log(
      `  ${styleText('bold', `${tool.command}:`)}${' '.repeat(
        getColumnWidth(tool.command),
      )}${formatToolVersion(tool, version)}`,
    );
  }
}

await printVersion(process.cwd());
