import fs from 'node:fs';
import path from 'node:path';

import { defineRule } from '@oxlint/plugins';
import type { Context } from '@oxlint/plugins';

type PackageJson = {
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
  optionalDependencies?: Record<string, string>;
  peerDependencies?: Record<string, string>;
  scripts?: Record<string, string>;
};

const DEPENDENCY_FIELDS = [
  'dependencies',
  'devDependencies',
  'optionalDependencies',
  'peerDependencies',
] as const;

const VITE_CONFIG_FILE_RE = /^vite\.config\.[cm]?[jt]s$/;

function readJsonFile(file: string): unknown {
  try {
    return JSON.parse(fs.readFileSync(file, 'utf8'));
  } catch {
    return null;
  }
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isStringRecord(value: unknown): value is Record<string, string> {
  return isObject(value) && Object.values(value).every((entry) => typeof entry === 'string');
}

function normalizePackageJson(value: unknown): PackageJson | null {
  if (!isObject(value)) {
    return null;
  }

  const pkg: PackageJson = {};
  for (const field of DEPENDENCY_FIELDS) {
    if (isStringRecord(value[field])) {
      pkg[field] = value[field];
    }
  }
  if (isStringRecord(value.scripts)) {
    pkg.scripts = value.scripts;
  }
  return pkg;
}

function findNearestFile(startDir: string, fileName: string): string | null {
  let currentDir = path.resolve(startDir);
  while (true) {
    const candidate = path.join(currentDir, fileName);
    if (fs.existsSync(candidate)) {
      return candidate;
    }

    const parentDir = path.dirname(currentDir);
    if (parentDir === currentDir) {
      return null;
    }
    currentDir = parentDir;
  }
}

function isViteConfigFile(filename: string): boolean {
  return VITE_CONFIG_FILE_RE.test(path.basename(filename));
}

function hasDependency(pkg: PackageJson, name: string): boolean {
  return DEPENDENCY_FIELDS.some((field) => pkg[field]?.[name] !== undefined);
}

function hasVitePlusAppScript(pkg: PackageJson): boolean {
  return Object.values(pkg.scripts ?? {}).some((script) =>
    /(?:^|[;&|]\s*)vp\s+(?:dev|build|preview)(?:\s|$)/.test(script),
  );
}

export function pnpmWorkspaceAliasesViteToVitePlusCore(workspaceConfig: string): boolean {
  const lines = workspaceConfig.split(/\r?\n/);
  const catalogViteAliasesCore = lines.some((line) =>
    /^\s*['"]?vite['"]?\s*:\s*['"]?(?:npm:|workspace:)?@voidzero-dev\/vite-plus-core@?/.test(line),
  );

  return lines.some((line, index) => {
    if (!/^\s*['"]?vite['"]?\s*:\s*['"]?catalog:['"]?\s*$/.test(line)) {
      return false;
    }

    const previousText = lines.slice(Math.max(0, index - 12), index).join('\n');
    return /\boverrides\s*:/.test(previousText) && catalogViteAliasesCore;
  });
}

export function shouldRequirePnpmViteAlias(filename: string): boolean {
  if (!isViteConfigFile(filename)) {
    return false;
  }

  const packageJsonPath = path.join(path.dirname(filename), 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return false;
  }

  const pkg = normalizePackageJson(readJsonFile(packageJsonPath));
  if (
    !pkg ||
    !hasDependency(pkg, 'vite-plus') ||
    hasDependency(pkg, 'vite') ||
    !hasVitePlusAppScript(pkg)
  ) {
    return false;
  }

  const workspaceConfigPath = findNearestFile(path.dirname(packageJsonPath), 'pnpm-workspace.yaml');
  if (!workspaceConfigPath) {
    return false;
  }

  let workspaceConfig = '';
  try {
    workspaceConfig = fs.readFileSync(workspaceConfigPath, 'utf8');
  } catch {
    return false;
  }

  return pnpmWorkspaceAliasesViteToVitePlusCore(workspaceConfig);
}

export const requirePnpmViteAliasRule = defineRule({
  meta: {
    type: 'problem',
    docs: {
      description:
        'Require pnpm Vite+ application packages to keep a direct vite alias dependency.',
      recommended: true,
      url: 'https://viteplus.dev/config/lint-rules#vite-plus-require-pnpm-vite-alias',
    },
    messages: {
      requirePnpmViteAlias:
        "pnpm Vite+ application packages must keep a direct 'vite' dependency so the workspace override resolves to @voidzero-dev/vite-plus-core.",
    },
  },
  createOnce(context: Context) {
    return {
      Program(node) {
        if (!shouldRequirePnpmViteAlias(context.physicalFilename)) {
          return;
        }

        context.report({
          node,
          messageId: 'requirePnpmViteAlias',
        });
      },
    };
  },
});
