import { AsyncLocalStorage } from 'node:async_hooks';
import fs from 'node:fs';
import path from 'node:path';

import { readStaticMigrationConfig } from '../../binding/index.js';
import { migrationIgnorePatterns } from '../migrate-config.ts';
import { createMigrationIgnoreMatcher } from '../migration-ignore.ts';
import { findViteConfig, resolveViteConfig } from '../resolve-vite-config.ts';
import { PackageManager } from '../types/index.ts';
import { VITEST_VERSION, VITE_PLUS_OVERRIDE_PACKAGES } from '../utils/constants.ts';
import { readJsonFile } from '../utils/json.ts';
import {
  extractOverrideParentSegments,
  extractOverrideTargetName,
} from '../utils/package-overrides.ts';
import { isAlignableVitestEcosystemPackage } from '../utils/vitest-ecosystem.ts';
import { readYamlFile } from '../utils/yaml.ts';

export class MigrationScope {
  readonly matcher;
  readonly ignoredManifests: string[] = [];

  constructor(
    readonly root: string,
    readonly patterns: string[],
  ) {
    this.matcher = createMigrationIgnoreMatcher(root, patterns);
    if (patterns.length > 0) {
      this.collectIgnoredManifests(root);
    }
  }

  ignores(file: string, isDirectory = false): boolean {
    return this.matcher.isIgnored(path.resolve(file), isDirectory);
  }

  private collectIgnoredManifests(dir: string): void {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const file = path.join(dir, entry.name);
      if (entry.isDirectory() && !['node_modules', '.git', '.yarn'].includes(entry.name)) {
        this.collectIgnoredManifests(file);
      } else if (entry.isFile() && entry.name === 'package.json' && this.ignores(file)) {
        this.ignoredManifests.push(file);
      }
    }
  }

  get preserveSharedDependencies(): boolean {
    return this.ignoredManifests.length > 0;
  }
}

const scopes = new AsyncLocalStorage<MigrationScope>();
export const withMigrationScope = <T>(scope: MigrationScope, run: () => T): T =>
  scopes.run(scope, run);
export const migrationScope = () => scopes.getStore();
export const isMigrationIgnored = (file: string, directory = false): boolean =>
  migrationScope()?.ignores(file, directory) ?? false;
export const preserveSharedDependencies = (): boolean =>
  migrationScope()?.preserveSharedDependencies ?? false;

async function readMigrationPatterns(root: string): Promise<string[]> {
  const extracted = readStaticMigrationConfig(root);
  const config =
    extracted == null ? (await resolveViteConfig(root)).migrate : JSON.parse(extracted);
  return migrationIgnorePatterns(config);
}

// Package-manager discovery treats a non-member example as a standalone project.
// An explicit target inside this repository must still honor the root policy.
// A nested Git repository (for example a cloned template) is independent.
export async function checkEnclosingMigrationPolicy(root: string): Promise<void> {
  let dir = root;
  while (!fs.existsSync(path.join(dir, '.git'))) {
    const parent = path.dirname(dir);
    if (parent === dir) {
      return;
    }
    dir = parent;
    if (!fs.existsSync(path.join(dir, 'package.json')) || !findViteConfig(dir)) {
      continue;
    }
    const patterns = await readMigrationPatterns(dir);
    if (patterns.length > 0) {
      const ignored = createMigrationIgnoreMatcher(dir, patterns).isIgnored(root, true);
      throw new Error(
        ignored
          ? `Cannot migrate ${path.relative(dir, root)}: excluded by migrate.ignorePatterns in ${findViteConfig(dir)}. No project files were changed.`
          : `Run vp migrate from ${dir} so its migrate.ignorePatterns policy also applies to nested files. No project files were changed.`,
      );
    }
  }
}

export async function loadMigrationScope(
  root: string,
  packageManager?: PackageManager,
): Promise<MigrationScope> {
  const scope = new MigrationScope(root, await readMigrationPatterns(root));
  if (scope.preserveSharedDependencies) {
    checkSharedOverrides(scope, packageManager);
  }
  return scope;
}

// Catalogs and overrides belong to every consumer, including templates outside
// the workspace package list. Use explicit specs for migrated consumers and
// retain these shared settings in their original locations.
export const sharedDependencyFields = [
  ['catalog'],
  ['catalogs'],
  ['overrides'],
  ['resolutions'],
  ['pnpm', 'overrides'],
  ['workspaces', 'catalog'],
  ['workspaces', 'catalogs'],
] as const;

function checkSharedOverrides(scope: MigrationScope, packageManager?: PackageManager): void {
  const pkg = readJsonFile(path.join(scope.root, 'package.json'));
  const settings = [pkg];
  for (const name of ['pnpm-workspace.yaml', '.yarnrc.yml']) {
    if (
      packageManager &&
      name !==
        (packageManager === PackageManager.pnpm
          ? 'pnpm-workspace.yaml'
          : packageManager === PackageManager.yarn
            ? '.yarnrc.yml'
            : '')
    ) {
      continue;
    }
    const file = path.join(scope.root, name);
    if (fs.existsSync(file)) {
      settings.push(readYamlFile(file) ?? {});
    }
  }
  const catalogs =
    settings.find((value) => value.catalog || value.catalogs) ??
    (pkg.workspaces as Record<string, unknown> | undefined) ??
    {};
  const resolveSpec = (spec: string, name: string): unknown => {
    if (spec.startsWith('catalog:')) {
      const catalogName = spec.slice(8) || 'default';
      const named = catalogs.catalogs as Record<string, Record<string, string>> | undefined;
      return (
        catalogName === 'default'
          ? ((catalogs.catalog as Record<string, string> | undefined) ?? named?.default)
          : named?.[catalogName]
      )?.[name];
    }
    if (spec.startsWith('$')) {
      const dependency = spec.slice(1);
      return (
        (pkg.dependencies as Record<string, string> | undefined)?.[dependency] ??
        (pkg.devDependencies as Record<string, string> | undefined)?.[dependency]
      );
    }
    return spec;
  };
  const excludedNames = new Set(
    scope.ignoredManifests.flatMap((file) => {
      try {
        return [readJsonFile(file).name];
      } catch {
        return [];
      }
    }),
  );
  const check = (overrides: unknown, parents: string[] = []) => {
    if (!overrides || typeof overrides !== 'object') {
      return;
    }
    for (const [selector, value] of Object.entries(overrides)) {
      const key = selector === '.' ? parents.at(-1)! : selector;
      const name = extractOverrideTargetName(key);
      const ancestors = selector === '.' ? parents.slice(0, -1) : parents;
      const chain = ancestors
        .map(extractOverrideTargetName)
        .concat(extractOverrideParentSegments(key) ?? []);
      // An override scoped exclusively to an excluded package can stay as-is.
      // For other selectors we cannot prove that the managed graph is isolated.
      const excludedOnly = chain.length > 0 && excludedNames.has(chain[0]);
      if (
        typeof value === 'string' &&
        (name === 'vite' || name === 'vitest' || isAlignableVitestEcosystemPackage(name)) &&
        !excludedOnly
      ) {
        const target = name === 'vite' ? VITE_PLUS_OVERRIDE_PACKAGES.vite : VITEST_VERSION;
        if (resolveSpec(value, name) !== target) {
          throw new Error(
            `Cannot migrate while preserving excluded packages: override ${JSON.stringify(selector)} (${JSON.stringify(value)}) conflicts with ${JSON.stringify(target)}. Review the root override or migrate the excluded packages first. No project files were changed.`,
          );
        }
      }
      if (value && typeof value === 'object') {
        check(value, [...parents, selector]);
      }
    }
  };
  for (const value of settings) {
    if (
      !packageManager ||
      packageManager === PackageManager.npm ||
      packageManager === PackageManager.bun ||
      (packageManager === PackageManager.pnpm && value !== pkg)
    ) {
      check(value.overrides);
    }
    if (!packageManager || packageManager === PackageManager.yarn) {
      check(value.resolutions);
    }
    if (!packageManager || packageManager === PackageManager.pnpm) {
      check((value.pnpm as Record<string, unknown> | undefined)?.overrides);
    }
  }
}
