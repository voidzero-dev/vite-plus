import fs from 'node:fs';
import path from 'node:path';

import { parse as parseYaml } from 'yaml';

const LEGACY_VITEST_PACKAGE = '@voidzero-dev/vite-plus-test';
const LEGACY_VITEST_ALIAS = `npm:${LEGACY_VITEST_PACKAGE}`;

function isLegacyVitestAlias(value: unknown): boolean {
  return (
    typeof value === 'string' &&
    (value === LEGACY_VITEST_ALIAS || value.startsWith(`${LEGACY_VITEST_ALIAS}@`))
  );
}

function containsLegacyVitestAlias(value: unknown): boolean {
  if (isLegacyVitestAlias(value)) {
    return true;
  }
  if (Array.isArray(value)) {
    return value.some(containsLegacyVitestAlias);
  }
  if (value !== null && typeof value === 'object') {
    return Object.values(value).some(containsLegacyVitestAlias);
  }
  return false;
}

function readConfigFile(filePath: string): unknown {
  if (!fs.existsSync(filePath)) {
    return undefined;
  }
  try {
    const source = fs.readFileSync(filePath, 'utf8');
    return path.basename(filePath) === 'package.json' ? JSON.parse(source) : parseYaml(source);
  } catch {
    // Configuration parsing has its own diagnostics. Do not replace them with
    // the stale-alias recovery message when the file is malformed.
    return undefined;
  }
}

/**
 * Find a package-manager setting that still aliases Vitest to the deleted
 * `@voidzero-dev/vite-plus-test` wrapper. Walk upward so lifecycle scripts in
 * workspace packages also inspect the root `pnpm-workspace.yaml`.
 */
export function findLegacyVitestAliasConfig(startDir: string): string | undefined {
  let currentDir = path.resolve(startDir);
  while (true) {
    for (const fileName of ['package.json', 'pnpm-workspace.yaml']) {
      const filePath = path.join(currentDir, fileName);
      if (containsLegacyVitestAlias(readConfigFile(filePath))) {
        return filePath;
      }
    }

    const parentDir = path.dirname(currentDir);
    if (parentDir === currentDir) {
      return undefined;
    }
    currentDir = parentDir;
  }
}
