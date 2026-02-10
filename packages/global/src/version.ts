import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { styleText } from 'node:util';

import { VITE_PLUS_NAME } from './utils/constants.ts';
import { detectPackageMetadata } from './utils/package.ts';
import { getVitePlusHeader, headline, log } from './utils/terminal.ts';

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

/**
 * Get the global CLI version from package.json
 */
function getGlobalVersion(): string {
  const pkg: PackageJson = require('../package.json');
  return pkg.version;
}

function getLocalMetadata(cwd: string): LocalPackageMetadata | null {
  const metadata = detectPackageMetadata(cwd, VITE_PLUS_NAME);
  return metadata ?? null;
}

function readPackageJsonFromPath(packageJsonPath: string): PackageJson | null {
  try {
    return JSON.parse(fs.readFileSync(packageJsonPath, 'utf8')) as PackageJson;
  } catch {
    return null;
  }
}

function readPackageVersionFromPath(packageJsonPath: string): string | null {
  return readPackageJsonFromPath(packageJsonPath)?.version ?? null;
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
    return readPackageVersionFromPath(fallbackPath);
  }
  return null;
}

function formatToolVersion(tool: ToolVersionSpec, version: string | null): string {
  return `${tool.displayName} ${version ? `v${version}` : `Not found`}`;
}

const cliLabel = 'vite-plus-cli';
const localLabel = 'vite-plus';
const columnWidth = cliLabel.length + 1;
const getColumnWidth = (label: string) => Math.max(0, columnWidth - label.length);

/**
 * Print version information for both local and global CLI
 */
export async function printVersion(cwd: string) {
  const globalVersion = getGlobalVersion();
  const localMetadata = getLocalMetadata(cwd);
  const localVersion = localMetadata?.version ?? null;

  log((await getVitePlusHeader()) + '\n');
  log(headline('Package Versions:'));
  log(`  ${styleText('bold', `${cliLabel}:`)} v${globalVersion}`);
  log(
    `  ${styleText('bold', `${localLabel}:`)}${' '.repeat(
      getColumnWidth(localLabel),
    )}${localVersion ? `v${localVersion}` : 'Not found'}`,
  );

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
    /*{
      command: 'doc',
      displayName: 'vitepress',
      packageName: 'vitepress',
      fallbackPackageJson: path.join('dist', 'vitepress', 'package.json'),
    },*/
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
