import fs from 'node:fs';
import path from 'node:path';

import { parse as parseYaml } from 'yaml';

import type { YamlMapping } from './yaml-value.ts';
import { isYamlMapping } from './yaml-value.ts';

const LEGACY_VITEST_ALIAS = 'npm:@voidzero-dev/vite-plus-test';

function containsLegacyVitestAlias(value: unknown, visited = new Set<object>()): boolean {
  if (typeof value === 'string') {
    return value === LEGACY_VITEST_ALIAS || value.startsWith(`${LEGACY_VITEST_ALIAS}@`);
  }
  if (value === null || typeof value !== 'object' || visited.has(value)) {
    return false;
  }
  visited.add(value);
  return Object.values(value).some((entry) => containsLegacyVitestAlias(entry, visited));
}

function containsAliasInFields(config: unknown, fields: Array<string>): boolean {
  return isYamlMapping(config) && fields.some((field) => containsLegacyVitestAlias(config[field]));
}

function containsPackageAlias(config: YamlMapping | undefined): boolean {
  return (
    containsAliasInFields(config, [
      'dependencies',
      'devDependencies',
      'optionalDependencies',
      'peerDependencies',
      'overrides',
      'resolutions',
      'catalog',
      'catalogs',
    ]) ||
    containsAliasInFields(config?.pnpm, ['overrides']) ||
    containsAliasInFields(config?.workspaces, ['catalog', 'catalogs'])
  );
}

function readConfigFile(filePath: string): YamlMapping | undefined {
  try {
    const source = fs.readFileSync(filePath, 'utf8');
    const config: unknown =
      path.basename(filePath) === 'package.json' ? JSON.parse(source) : parseYaml(source);
    return isYamlMapping(config) ? config : undefined;
  } catch {
    // Configuration parsing has its own diagnostics. Do not replace them with
    // the stale-alias recovery message when the file is malformed.
    return undefined;
  }
}

/**
 * Find a package-manager setting that still aliases Vitest to the deleted
 * `@voidzero-dev/vite-plus-test` wrapper. Check the nearest package and its
 * workspace root, without inspecting unrelated ancestor package settings.
 */
export function findLegacyVitestAliasConfig(startDir: string): string | undefined {
  let currentDir = path.resolve(startDir);
  let foundPackage = false;
  while (true) {
    const packagePath = path.join(currentDir, 'package.json');
    const pkg = readConfigFile(packagePath);
    const isNearestPackage = !foundPackage && fs.existsSync(packagePath);
    if (isNearestPackage) {
      foundPackage = true;
    }

    // Match workspace discovery: the nearest pnpm-workspace.yaml or
    // package.json with a workspaces field defines the workspace root.
    // If neither exists, only the nearest package's settings apply.
    const workspacePath = path.join(currentDir, 'pnpm-workspace.yaml');
    const isWorkspaceRoot =
      fs.existsSync(workspacePath) || (pkg !== undefined && Object.hasOwn(pkg, 'workspaces'));

    if ((isNearestPackage || isWorkspaceRoot) && containsPackageAlias(pkg)) {
      return packagePath;
    }
    if (isWorkspaceRoot) {
      if (
        containsAliasInFields(readConfigFile(workspacePath), ['catalog', 'catalogs', 'overrides'])
      ) {
        return workspacePath;
      }
      return undefined;
    }

    const parentDir = path.dirname(currentDir);
    if (parentDir === currentDir) {
      return undefined;
    }
    currentDir = parentDir;
  }
}
