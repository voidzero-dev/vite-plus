import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';

import { vitePlusHeader } from '../binding/index.js';
import { VITE_PLUS_NAME } from './utils/constants.js';
import { renderCliDoc } from './utils/help.js';
import { detectPackageMetadata, hasVitePlusDependency } from './utils/package.js';
import { accent, log } from './utils/terminal.js';

const require = createRequire(import.meta.url);

interface PackageJson {
  version?: string;
  bundledVersions?: Record<string, string>;
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
}

interface LocalPackageMetadata {
  name: string;
  version: string;
  path: string;
}

interface ToolVersionSpec {
  displayName: string;
  packageName: string;
  bundledVersionKey?: string;
  fallbackPackageJson?: string;
}

function getGlobalVersion(): string | null {
  return process.env.VITE_PLUS_GLOBAL_VERSION ?? null;
}

function getCliVersion(): string | null {
  const pkg = resolvePackageJson(VITE_PLUS_NAME, process.cwd());
  return pkg?.version ?? null;
}

function getLocalMetadata(cwd: string): LocalPackageMetadata | null {
  if (!isVitePlusDeclaredInAncestors(cwd)) {
    return null;
  }
  return detectPackageMetadata(cwd, VITE_PLUS_NAME) ?? null;
}

function isVitePlusDeclaredInAncestors(cwd: string): boolean {
  let currentDir = path.resolve(cwd);
  while (true) {
    const packageJsonPath = path.join(currentDir, 'package.json');
    const pkg = readPackageJsonFromPath(packageJsonPath);
    if (pkg && hasVitePlusDependency(pkg)) {
      return true;
    }
    const parentDir = path.dirname(currentDir);
    if (parentDir === currentDir) {
      break;
    }
    currentDir = parentDir;
  }
  return false;
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
    // Try resolving package.json subpath directly
    const packageJsonPath = require.resolve(`${packageName}/package.json`, {
      paths: [baseDir],
    });
    return readPackageJsonFromPath(packageJsonPath);
  } catch {
    // Fallback for packages with restricted exports that don't expose ./package.json:
    // resolve the main entry and find package.json relative to it
    try {
      const mainPath = require.resolve(packageName, { paths: [baseDir] });
      // Walk up from the resolved entry to find the package.json
      let dir = path.dirname(mainPath);
      while (dir !== path.dirname(dir)) {
        const pkgPath = path.join(dir, 'package.json');
        const pkg = readPackageJsonFromPath(pkgPath);
        if (pkg) {
          return pkg;
        }
        dir = path.dirname(dir);
      }
    } catch {
      // package not found at all
    }
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

/**
 * Print version information
 */
export async function printVersion(cwd: string) {
  const globalVersion = getGlobalVersion();
  const cliVersion = getCliVersion();
  const localMetadata = getLocalMetadata(cwd);
  const localVersion = localMetadata?.version ?? null;
  const vpVersion = globalVersion ?? cliVersion ?? localVersion ?? 'unknown';

  log(vitePlusHeader());
  log('');
  log(`vp v${vpVersion}\n`);

  const sections = [
    {
      title: 'Local vite-plus',
      rows: [
        {
          label: accent('vite-plus'),
          description: localVersion ? `v${localVersion}` : 'Not found',
        },
      ],
    },
  ];

  const tools: ToolVersionSpec[] = [
    {
      displayName: 'vite',
      packageName: '@voidzero-dev/vite-plus-core',
      bundledVersionKey: 'vite',
    },
    {
      displayName: 'rolldown',
      packageName: '@voidzero-dev/vite-plus-core',
      bundledVersionKey: 'rolldown',
    },
    {
      displayName: 'vitest',
      packageName: '@voidzero-dev/vite-plus-test',
      bundledVersionKey: 'vitest',
    },
    {
      displayName: 'oxfmt',
      packageName: 'oxfmt',
    },
    {
      displayName: 'oxlint',
      packageName: 'oxlint',
    },
    {
      displayName: 'oxlint-tsgolint',
      packageName: 'oxlint-tsgolint',
    },
    {
      displayName: 'tsdown',
      packageName: '@voidzero-dev/vite-plus-core',
      bundledVersionKey: 'tsdown',
    },
  ];

  if (localMetadata) {
    const resolvedTools = tools.map((tool) => ({
      tool,
      version: resolveToolVersion(tool, localMetadata.path),
    }));

    sections.push({
      title: 'Tools',
      rows: resolvedTools.map(({ tool, version }) => ({
        label: accent(tool.displayName),
        description: version ? `v${version}` : 'Not found',
      })),
    });
  }

  log(renderCliDoc({ sections }));
}

await printVersion(process.cwd());
