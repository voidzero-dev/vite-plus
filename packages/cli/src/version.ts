import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';

import { VITE_PLUS_NAME } from './utils/constants.js';
import { renderCliDoc } from './utils/help.js';
import { detectPackageMetadata } from './utils/package.js';
import { getVitePlusHeader, log } from './utils/terminal.js';

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

function getGlobalVersion(): string | null {
  return process.env.VITE_PLUS_GLOBAL_VERSION ?? null;
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

function formatToolVersion(tool: ToolVersionSpec, version: string | null): string {
  return `${tool.displayName} ${version ? `v${version}` : `Not found`}`;
}

/**
 * Print version information
 */
export async function printVersion(cwd: string) {
  const globalVersion = getGlobalVersion();
  const localMetadata = getLocalMetadata(cwd);
  const localVersion = localMetadata?.version ?? null;

  log((await getVitePlusHeader()) + '\n');

  const sections = [
    {
      title: 'Package Versions',
      rows: [
        {
          label: 'global vite-plus',
          description: globalVersion ? `v${globalVersion}` : 'Not found',
        },
        {
          label: 'local vite-plus',
          description: localVersion ? `v${localVersion}` : 'Not found',
        },
      ],
    },
  ];

  if (!localMetadata) {
    log(renderCliDoc({ sections }));
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
    log(renderCliDoc({ sections }));
    return;
  }

  sections.push({
    title: 'Bundled with vite-plus',
    rows: resolvedTools.map(({ tool, version }) => ({
      label: tool.command,
      description: formatToolVersion(tool, version),
    })),
  });

  log(renderCliDoc({ sections }));
}

await printVersion(process.cwd());
