import fs from 'node:fs';
import path from 'node:path';
import { styleText } from 'node:util';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import { type OxlintConfig } from 'oxlint';

import { rewriteEslint } from '../../../binding/index.js';
import { type WorkspacePackage } from '../../types/index.ts';
import { runCommandSilently } from '../../utils/command.ts';
import { editJsonFile, isJsonFile, readJsonFile } from '../../utils/json.ts';
import { displayRelative } from '../../utils/path.ts';
import { cancelAndExit } from '../../utils/prompts.ts';
import { getSpinner } from '../../utils/spinner.ts';
import { hasBaseUrlInTsconfig } from '../../utils/tsconfig.ts';
import { detectConfigs } from '../detector.ts';
import { type MigrationReport } from '../report.ts';
import {
  LINT_STAGED_JSON_CONFIG_FILES,
  LINT_STAGED_OTHER_CONFIG_FILES,
  warnMigration,
} from './shared.ts';

// Plugins Oxlint resolves natively (no JS import). Source:
// `LintPluginOptionsSchema` in `node_modules/oxlint/dist/index.d.ts`.
// Anything else in the merged `lint.plugins[]` after migration is a
// reference left over from `@oxlint/migrate` that won't resolve at lint
// time.
const OXLINT_NATIVE_PLUGINS = new Set<string>([
  'eslint',
  'react',
  'unicorn',
  'typescript',
  'oxc',
  'import',
  'jsdoc',
  'jest',
  'vitest',
  'jsx-a11y',
  'nextjs',
  'react-perf',
  'promise',
  'node',
  'vue',
]);

export function detectEslintProject(
  projectPath: string,
  packages?: WorkspacePackage[],
): {
  hasDependency: boolean;
  configFile?: string;
  legacyConfigFile?: string;
} {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return { hasDependency: false };
  }
  const pkg = readJsonFile(packageJsonPath) as {
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
  };
  let hasDependency = !!(pkg.devDependencies?.eslint || pkg.dependencies?.eslint);
  const configs = detectConfigs(projectPath);
  let configFile = configs.eslintConfig;
  const legacyConfigFile = configs.eslintLegacyConfig;

  // If root doesn't have eslint dependency, check workspace packages
  if (!hasDependency && packages) {
    for (const wp of packages) {
      const pkgJsonPath = path.join(projectPath, wp.path, 'package.json');
      if (!fs.existsSync(pkgJsonPath)) {
        continue;
      }
      const wpPkg = readJsonFile(pkgJsonPath) as {
        devDependencies?: Record<string, string>;
        dependencies?: Record<string, string>;
      };
      if (wpPkg.devDependencies?.eslint || wpPkg.dependencies?.eslint) {
        hasDependency = true;
        break;
      }
    }
  }

  return { hasDependency, configFile, legacyConfigFile };
}

/**
 * Run a `vp dlx @oxlint/migrate` step with graceful error handling.
 *
 * `packages` lists candidate `@oxlint/migrate` specs in priority order. The
 * first entry is normally the version pinned to the bundled oxlint; later
 * entries are fallbacks. `@oxlint/migrate` is published in lockstep with
 * oxlint, but the migrate release can lag the linter by a few hours — when
 * the pinned version isn't on npm yet, we silently fall through to the next
 * candidate (the latest published migrate) instead of failing the migration.
 *
 * Returns `{ ok, usedPackage }` where `usedPackage` is the spec that ran
 * successfully (so later steps reuse the same version).
 */
async function runOxlintMigrateStep(
  vpBin: string,
  cwd: string,
  packages: string[],
  args: string[],
  spinner: ReturnType<typeof getSpinner>,
  failMessage: string,
  manualHint: string,
): Promise<{ ok: boolean; usedPackage?: string }> {
  for (let i = 0; i < packages.length; i++) {
    const migratePackage = packages[i];
    try {
      const result = await runCommandSilently({
        command: vpBin,
        args: ['dlx', migratePackage, ...args],
        cwd,
        envs: process.env,
      });
      if (result.exitCode === 0) {
        return { ok: true, usedPackage: migratePackage };
      }
      const stderr = result.stderr.toString();
      // If the pinned version simply isn't published yet, try the next
      // (unpinned) candidate silently rather than reporting a failure.
      const missingVersion = /ERR_PNPM_NO_MATCHING_VERSION|No matching version found/i.test(stderr);
      if (missingVersion && i < packages.length - 1) {
        continue;
      }
      spinner.stop(failMessage);
      const trimmed = stderr.trim();
      if (trimmed) {
        prompts.log.warn(`⚠ ${trimmed}`);
      }
      prompts.log.info(manualHint);
      return { ok: false };
    } catch {
      spinner.stop(failMessage);
      prompts.log.info(manualHint);
      return { ok: false };
    }
  }
  return { ok: false };
}

export async function migrateEslintToOxlint(
  projectPath: string,
  interactive: boolean,
  eslintConfigFile?: string,
  packages?: WorkspacePackage[],
  options?: { silent?: boolean; report?: MigrationReport },
): Promise<boolean> {
  const vpBin = process.env.VP_CLI_BIN ?? 'vp';
  const spinner = options?.silent
    ? {
        start: () => {},
        stop: () => {},
        pause: () => {},
        resume: () => {},
        cancel: () => {},
        error: () => {},
        clear: () => {},
        message: () => {},
        isCancelled: false,
      }
    : getSpinner(interactive);

  // Steps 1-2: Only run @oxlint/migrate if there's an eslint config at root
  if (eslintConfigFile) {
    // Pin @oxlint/migrate to the bundled oxlint version, falling back to the
    // latest published migrate when that exact version isn't on npm yet (the
    // migrate package can lag oxlint by a few hours after a release).
    // @ts-expect-error — resolved at runtime from dist/ → dist/versions.js
    const { versions } = await import('../versions.js');
    const pinnedPackage = `@oxlint/migrate@${versions.oxlint}`;
    const fallbackPackage = '@oxlint/migrate';
    const migrateCandidates = [pinnedPackage, fallbackPackage];
    const migrateArgs = [
      '--merge',
      ...(!hasBaseUrlInTsconfig(projectPath) ? ['--type-aware'] : []),
      '--with-nursery',
      '--details',
    ];

    // Step 1: Generate .oxlintrc.json from ESLint config
    spinner.start('Migrating ESLint config to Oxlint...');
    const migrateResult = await runOxlintMigrateStep(
      vpBin,
      projectPath,
      migrateCandidates,
      migrateArgs,
      spinner,
      'ESLint migration failed',
      `You can run \`vp dlx ${fallbackPackage} ${migrateArgs.join(' ')}\` manually later`,
    );
    if (!migrateResult.ok) {
      return false;
    }
    // Reuse the version that actually resolved for the remaining steps.
    const migratePackage = migrateResult.usedPackage ?? pinnedPackage;
    spinner.stop('ESLint config migrated to .oxlintrc.json');

    // Step 2: Replace eslint-disable comments with oxlint-disable
    spinner.start('Replacing ESLint comments with Oxlint equivalents...');
    const replaceResult = await runOxlintMigrateStep(
      vpBin,
      projectPath,
      [migratePackage],
      ['--replace-eslint-comments'],
      spinner,
      'ESLint comment replacement failed',
      `You can run \`vp dlx ${migratePackage} --replace-eslint-comments\` manually later`,
    );
    if (replaceResult.ok) {
      spinner.stop('ESLint comments replaced');
    }
    // Continue with cleanup regardless — .oxlintrc.json was generated successfully
  }

  if (options?.report) {
    options.report.eslintMigrated = true;
  }

  // Read the generated `.oxlintrc.json` to find any packages it references
  // in `lint.jsPlugins`. Those packages need to stay in `package.json` so
  // Oxlint can actually `import()` them at lint time — without this carve-out,
  // the next step would strip them via `isEslintEcosystemDep` and we'd
  // immediately invalidate the config we just generated. Local-path
  // specifiers (`./X`, `../X`, `/X`) are skipped — they're paths, not
  // package names, and have no `package.json` entry to preserve.
  const preserveJsPlugins = collectJsPluginPackageNames(projectPath);

  // Step 3-5: Cleanup runs uniformly across the root and every workspace
  // package — delete eslint config files, scrub ESLint-ecosystem deps from
  // package.json, and rewrite eslint references in any local lint-staged
  // config. A monorepo running `vp migrate` is treated as adopted as a
  // whole; there's no per-package opt-out today. If a workspace package
  // publishes a shared ESLint preset that you want to keep intact, exclude
  // it from your `pnpm-workspace.yaml` / `workspaces` before running
  // `vp migrate`, then add it back afterwards.
  const cleanupTargets = [
    projectPath,
    ...(packages ?? []).map((p) => path.join(projectPath, p.path)),
  ];
  for (const target of cleanupTargets) {
    if (!fs.existsSync(path.join(target, 'package.json'))) {
      continue;
    }
    deleteEslintConfigFiles(target, options?.report, options?.silent);
    rewriteEslintPackageJson(path.join(target, 'package.json'), preserveJsPlugins);
    rewriteEslintLintStagedConfigFiles(target, options?.report);
  }

  return true;
}

/**
 * Read `<projectPath>/.oxlintrc.json` (if any) and collect the package
 * names referenced via `lint.jsPlugins[]` string entries. Object-form
 * entries (`{ name, specifier }`) and local-path specifiers (`./X`,
 * `../X`, `/X`) are excluded — neither maps to a `package.json` entry
 * we'd accidentally strip.
 */
function collectJsPluginPackageNames(projectPath: string): Set<string> {
  const out = new Set<string>();
  const oxlintConfigPath = path.join(projectPath, '.oxlintrc.json');
  if (!fs.existsSync(oxlintConfigPath)) {
    return out;
  }
  let config: OxlintConfig;
  try {
    config = readJsonFile(oxlintConfigPath, true);
  } catch {
    return out;
  }
  const collectFrom = (jsPlugins: OxlintConfig['jsPlugins']): void => {
    for (const entry of jsPlugins ?? []) {
      if (typeof entry !== 'string') {
        continue;
      }
      if (entry.startsWith('./') || entry.startsWith('../') || entry.startsWith('/')) {
        continue;
      }
      out.add(entry);
    }
  };
  collectFrom(config.jsPlugins);
  if (Array.isArray(config.overrides)) {
    for (const override of config.overrides) {
      collectFrom(override.jsPlugins);
    }
  }
  return out;
}

function deleteEslintConfigFiles(basePath: string, report?: MigrationReport, silent = false): void {
  const configs = detectConfigs(basePath);
  for (const file of [configs.eslintConfig, configs.eslintLegacyConfig]) {
    if (file) {
      const configPath = path.join(basePath, file);
      if (fs.existsSync(configPath)) {
        fs.unlinkSync(configPath);
        if (report) {
          report.removedConfigCount++;
        }
        if (!silent) {
          prompts.log.success(`✔ Removed ${displayRelative(configPath)}`);
        }
      }
    }
  }
}

// Bare names of packages whose sole purpose is to support ESLint. Removed
// at root cleanup. Reusable AST libraries published under
// `@typescript-eslint/*` (`utils`, `typescript-estree`, `scope-manager`,
// `types`) are deliberately absent so codemods and doc generators that
// import them directly keep working after migration.
const ESLINT_ECOSYSTEM_NAMES = new Set<string>([
  'eslint',
  'typescript-eslint',
  'eslintrc',
  'eslint-utils',
  'eslint-visitor-keys',
  'eslint-scope',
  'eslint-define-config',
  'eslint-doc-generator',
  // ESLint-only typescript-eslint entry points:
  '@typescript-eslint/eslint-plugin',
  '@typescript-eslint/parser',
  '@typescript-eslint/rule-tester',
  // Note: framework-ESLint integration modules (e.g. `@nuxt/eslint`)
  // are NOT listed here. They short-circuit the entire ESLint
  // migration via `INCOMPATIBLE_ESLINT_INTEGRATIONS`, so this list is
  // never consulted for them. Keeping them out avoids duplicating the
  // "what to do about Nuxt" decision in two places.
]);

// Flat name prefixes that mark an ESLint-only package.
const ESLINT_ECOSYSTEM_PREFIXES = ['eslint-plugin-', 'eslint-config-', 'eslint-formatter-'];

// Scopes whose every package is part of the ESLint ecosystem.
//   @eslint/*           — official ESLint scope (e.g. @eslint/js, @eslint/eslintrc)
//   @eslint-community/* — community-maintained ESLint dependencies
//   @angular-eslint/*   — Angular's ESLint integration family
const ESLINT_ECOSYSTEM_SCOPES = ['@eslint/', '@eslint-community/', '@angular-eslint/'];

/**
 * Decide whether a dependency entry should be removed alongside `eslint`
 * itself. The set is intentionally broad: anything whose only purpose is
 * to extend, configure, format, or wire ESLint becomes dead weight after
 * migration. `@types/<X>` packages are checked symmetrically with `<X>`
 * so type-only counterparts of removed runtime packages also go.
 */
function isEslintEcosystemDep(name: string): boolean {
  const stripped = name.startsWith('@types/') ? name.slice('@types/'.length) : name;
  if (ESLINT_ECOSYSTEM_NAMES.has(stripped)) {
    return true;
  }
  if (ESLINT_ECOSYSTEM_PREFIXES.some((p) => stripped.startsWith(p))) {
    return true;
  }
  if (ESLINT_ECOSYSTEM_SCOPES.some((s) => stripped.startsWith(s))) {
    return true;
  }
  // Scoped plugins/configs/formatters, e.g.:
  //   @vue/eslint-config-typescript
  //   @stylistic/eslint-plugin-ts
  //   @vitest/eslint-plugin
  if (/^@[^/]+\/eslint-(plugin|config|formatter)(-.+)?$/.test(stripped)) {
    return true;
  }
  return false;
}

/**
 * Rewrite a project's `package.json` after ESLint has been migrated to
 * Oxlint: drop every ESLint-ecosystem dependency (see
 * `isEslintEcosystemDep`), strip empty containers, and rewrite eslint
 * tokens in scripts / lint-staged. Applied uniformly to the root and to
 * every workspace package — the migration treats the whole workspace as
 * in scope for adoption, so a half-cleanup at the workspace level would
 * be inconsistent with the rest of the flow (which already replaces
 * vite-related overrides and adds vite-plus across all packages).
 *
 * `preserveJsPlugins` names packages that `@oxlint/migrate` referenced
 * via `lint.jsPlugins` and that Oxlint will need to `import()` at lint
 * time. They override `isEslintEcosystemDep` so the generated config
 * isn't immediately invalidated by the cleanup step.
 */
export function rewriteEslintPackageJson(
  packageJsonPath: string,
  preserveJsPlugins: ReadonlySet<string> = new Set(),
): void {
  editJsonFile<{
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
    peerDependencies?: Record<string, string>;
    optionalDependencies?: Record<string, string>;
    scripts?: Record<string, string>;
    'lint-staged'?: Record<string, string | string[]>;
  }>(packageJsonPath, (pkg) => {
    let changed = false;
    for (const field of [
      'devDependencies',
      'dependencies',
      'peerDependencies',
      'optionalDependencies',
    ] as const) {
      const deps = pkg[field];
      if (!deps) {
        continue;
      }
      let removedAny = false;
      for (const name of Object.keys(deps)) {
        if (preserveJsPlugins.has(name)) {
          continue;
        }
        if (isEslintEcosystemDep(name)) {
          delete deps[name];
          changed = true;
          removedAny = true;
        }
      }
      // Drop the field entirely if our cleanup emptied it — avoid
      // leaving `"devDependencies": {}` noise in the output.
      if (removedAny && Object.keys(deps).length === 0) {
        delete pkg[field];
      }
    }
    if (pkg.scripts) {
      const updated = rewriteEslint(JSON.stringify(pkg.scripts));
      if (updated) {
        pkg.scripts = JSON.parse(updated);
        changed = true;
      }
    }
    if (pkg['lint-staged']) {
      const updated = rewriteEslint(JSON.stringify(pkg['lint-staged']));
      if (updated) {
        pkg['lint-staged'] = JSON.parse(updated);
        changed = true;
      }
    }
    return changed ? pkg : undefined;
  });
}

/**
 * Rewrite tool references in lint-staged config files (JSON ones are rewritten,
 * non-JSON ones get a warning).
 */
export function rewriteToolLintStagedConfigFiles(
  projectPath: string,
  rewriteFn: (json: string) => string | null,
  toolName: string,
  report?: MigrationReport,
): void {
  for (const filename of LINT_STAGED_JSON_CONFIG_FILES) {
    const configPath = path.join(projectPath, filename);
    if (!fs.existsSync(configPath)) {
      continue;
    }
    if (filename === '.lintstagedrc' && !isJsonFile(configPath)) {
      warnMigration(
        `${displayRelative(configPath)} is not JSON — please update ${toolName} references manually`,
        report,
      );
      continue;
    }
    editJsonFile<Record<string, string | string[]>>(configPath, (config) => {
      const updated = rewriteFn(JSON.stringify(config));
      if (updated) {
        return JSON.parse(updated);
      }
      return undefined;
    });
  }
  for (const filename of LINT_STAGED_OTHER_CONFIG_FILES) {
    const configPath = path.join(projectPath, filename);
    if (!fs.existsSync(configPath)) {
      continue;
    }
    warnMigration(
      `${displayRelative(configPath)} — please update ${toolName} references manually`,
      report,
    );
  }
}

function rewriteEslintLintStagedConfigFiles(projectPath: string, report?: MigrationReport): void {
  rewriteToolLintStagedConfigFiles(projectPath, rewriteEslint, 'eslint', report);
}

/**
 * Best-effort: derive the Oxlint rule-namespace a JS plugin package
 * contributes. Mirrors the conventions @oxlint/migrate uses when
 * translating ESLint configs, and the conventions Oxlint-native plugin
 * authors use (`oxlint-plugin-<name>` — see posva/pinia-colada in the
 * wild):
 *   `eslint-plugin-unocss`         → `unocss`        (rules: `unocss/order`)
 *   `oxlint-plugin-posva`          → `posva`         (rules: `posva/foo`)
 *   `@stylistic/eslint-plugin`     → `@stylistic`    (rules: `@stylistic/indent`)
 *   `@stylistic/eslint-plugin-ts`  → `@stylistic/ts` (rules: `@stylistic/ts/indent`)
 *   `@scope/oxlint-plugin-x`       → `@scope/x`
 *   anything else                  → the package name verbatim
 */
function deriveJsPluginNamespace(packageName: string): string {
  for (const prefix of ['eslint-plugin-', 'oxlint-plugin-']) {
    if (packageName.startsWith(prefix)) {
      const suffix = packageName.slice(prefix.length);
      return suffix || packageName;
    }
  }
  const scoped = packageName.match(/^(@[^/]+)\/(?:eslint|oxlint)-plugin(?:-(.+))?$/);
  if (scoped) {
    return scoped[2] ? `${scoped[1]}/${scoped[2]}` : scoped[1];
  }
  return packageName;
}

/**
 * Collect every dependency name declared across the root + workspace
 * `package.json` files after the ESLint cleanup has run. Used to verify
 * that JS plugins referenced by the generated `.oxlintrc.json` are
 * actually installable.
 */
export function collectInstalledPackageNames(
  projectPath: string,
  packages?: WorkspacePackage[],
): Set<string> {
  const names = new Set<string>();
  const paths = [projectPath, ...(packages ?? []).map((p) => path.join(projectPath, p.path))];
  for (const dir of paths) {
    const pkgJsonPath = path.join(dir, 'package.json');
    if (!fs.existsSync(pkgJsonPath)) {
      continue;
    }
    let pkg: Record<string, Record<string, string> | undefined>;
    try {
      pkg = readJsonFile(pkgJsonPath) as typeof pkg;
    } catch {
      continue;
    }
    // Limit to actual install groups. A package's own peerDependencies are
    // not installed in its node_modules (the consumer must provide them), so
    // a peer-only package is not actually loadable by Oxlint at lint time and
    // must not count as available here.
    for (const field of ['devDependencies', 'dependencies', 'optionalDependencies'] as const) {
      const deps = pkg[field];
      if (deps) {
        for (const name of Object.keys(deps)) {
          names.add(name);
        }
      }
    }
  }
  return names;
}

/**
 * Test whether a rule key (e.g. `@stylistic/ts/indent`) belongs to any
 * namespace in `namespaces`. We can't just split on the first `/` —
 * `@stylistic/eslint-plugin-ts` contributes the multi-segment namespace
 * `@stylistic/ts`, so the lookup has to try progressively longer
 * prefixes until one matches or we run out of slashes.
 */
function ruleKeyMatchesNamespace(key: string, namespaces: Set<string>): boolean {
  if (!key.includes('/')) {
    return true;
  }
  let idx = key.indexOf('/');
  while (idx !== -1) {
    if (namespaces.has(key.slice(0, idx))) {
      return true;
    }
    idx = key.indexOf('/', idx + 1);
  }
  return false;
}

/** Filter a rules object to only entries whose namespace is recognized. */
function filterRulesAgainstNamespaces(
  rules: Record<string, unknown>,
  namespaces: Set<string>,
): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(rules)) {
    if (ruleKeyMatchesNamespace(key, namespaces)) {
      out[key] = value;
    }
  }
  return out;
}

/**
 * Sort a jsPlugins array into installed entries (kept) and string
 * entries for packages that aren't present in the workspace. Object-form
 * entries (`{ name, specifier }`) and string entries that look like
 * local paths (`./X`, `/X`, `../X`) are passed through — Oxlint resolves
 * them itself.
 */
function partitionJsPlugins(
  entries: NonNullable<OxlintConfig['jsPlugins']>,
  availablePackages: Set<string>,
): {
  kept: NonNullable<OxlintConfig['jsPlugins']>;
  dropped: string[];
} {
  const kept: NonNullable<OxlintConfig['jsPlugins']> = [];
  const dropped: string[] = [];
  for (const entry of entries) {
    if (typeof entry !== 'string') {
      kept.push(entry);
      continue;
    }
    // Local-path specifiers don't go through `package.json`; preserve
    // them so users with hand-authored local plugin imports survive
    // a `vp migrate` re-run.
    if (entry.startsWith('./') || entry.startsWith('../') || entry.startsWith('/')) {
      kept.push(entry);
      continue;
    }
    if (availablePackages.has(entry)) {
      kept.push(entry);
    } else {
      dropped.push(entry);
    }
  }
  return { kept, dropped };
}

/** Build the set of rule-key namespaces backed by a given jsPlugins set. */
function jsPluginsToNamespaces(entries: NonNullable<OxlintConfig['jsPlugins']>): Set<string> {
  const ns = new Set<string>();
  for (const entry of entries) {
    if (typeof entry === 'string') {
      ns.add(deriveJsPluginNamespace(entry));
    } else if (entry && typeof entry === 'object' && 'name' in entry && entry.name) {
      ns.add(entry.name);
    }
  }
  // Empty-string namespace (e.g. from `eslint-plugin-` with no suffix)
  // would smuggle slash-prefixed rules through; drop it defensively.
  ns.delete('');
  return ns;
}

/**
 * Sanitize the `.oxlintrc.json` produced by `@oxlint/migrate` (in-place)
 * before it gets merged into `vite.config.ts`. Drop references that
 * won't resolve at lint time and warn the user.
 *
 * Why: `@oxlint/migrate` can emit `jsPlugins[]` / `plugins[]` / `rules`
 * entries referring to packages the user never installed (e.g.
 * translating `@unocss/eslint-config` into `eslint-plugin-unocss`),
 * to plugins outside Oxlint's native set, or under namespaces no
 * surviving plugin contributes. Without sanitization, `vp lint` aborts
 * with "Failed to load JS plugin" / "Plugin not found" before running
 * any rule. This produces a degraded-but-functional config instead.
 *
 * Per-override entries (`overrides[].jsPlugins`, `.plugins`, `.rules`)
 * are sanitized independently — an override can introduce its own
 * jsPlugin, so namespace availability is computed per-override (base
 * namespaces ∪ the override's own surviving jsPlugins' namespaces).
 */
export function sanitizeMigratedOxlintConfig(
  config: OxlintConfig,
  availablePackages: Set<string>,
  report?: MigrationReport,
): void {
  // Track everything we strip so we can warn the user.
  const allDroppedJsPlugins = new Set<string>();
  const allDroppedPlugins = new Set<string>();

  // 1. Sanitize base-level jsPlugins.
  const baseSplit = partitionJsPlugins(config.jsPlugins ?? [], availablePackages);
  for (const n of baseSplit.dropped) {
    allDroppedJsPlugins.add(n);
  }
  if (config.jsPlugins && baseSplit.dropped.length > 0) {
    config.jsPlugins = baseSplit.kept;
  }

  // 2. Base namespaces = native plugins + surviving jsPlugins' namespaces.
  const baseNamespaces = new Set<string>(OXLINT_NATIVE_PLUGINS);
  for (const ns of jsPluginsToNamespaces(baseSplit.kept)) {
    baseNamespaces.add(ns);
  }

  // 3. Sanitize base-level plugins[] against base namespaces.
  if (config.plugins) {
    type PluginEntry = NonNullable<OxlintConfig['plugins']>[number];
    const keptPlugins: PluginEntry[] = [];
    for (const p of config.plugins) {
      if (baseNamespaces.has(p)) {
        keptPlugins.push(p);
      } else {
        allDroppedPlugins.add(p);
      }
    }
    if (keptPlugins.length !== config.plugins.length) {
      config.plugins = keptPlugins;
    }
  }

  // 4. Sanitize base rules. Guard the reassignment to avoid adding a
  // `rules: undefined` property that would shift downstream key
  // emission in the merged vite.config.ts.
  if (config.rules) {
    const filtered = filterRulesAgainstNamespaces(config.rules, baseNamespaces);
    if (Object.keys(filtered).length !== Object.keys(config.rules).length) {
      config.rules = filtered as typeof config.rules;
    }
  }

  // 5. Sanitize each override INDEPENDENTLY. An override can declare
  // its own `jsPlugins` / `plugins`, so we compute a per-override
  // namespace set: base namespaces ∪ the override's own surviving
  // jsPlugins' namespaces. If `override.plugins` is present it
  // replaces base.plugins per Oxlint's schema, but for namespace
  // resolution we still include the base set (rules under a base
  // namespace are still valid inside the override).
  if (Array.isArray(config.overrides)) {
    for (const override of config.overrides) {
      // Override jsPlugins.
      let overrideSurvivors: NonNullable<OxlintConfig['jsPlugins']> = [];
      if (override.jsPlugins) {
        const split = partitionJsPlugins(override.jsPlugins, availablePackages);
        for (const n of split.dropped) {
          allDroppedJsPlugins.add(n);
        }
        if (split.dropped.length > 0) {
          override.jsPlugins = split.kept;
        }
        overrideSurvivors = split.kept;
      }
      const overrideNamespaces = new Set<string>(baseNamespaces);
      for (const ns of jsPluginsToNamespaces(overrideSurvivors)) {
        overrideNamespaces.add(ns);
      }

      // Override plugins[].
      if (override.plugins) {
        type OverridePluginEntry = NonNullable<typeof override.plugins>[number];
        const keptOverridePlugins: OverridePluginEntry[] = [];
        for (const p of override.plugins) {
          if (overrideNamespaces.has(p)) {
            keptOverridePlugins.push(p);
          } else {
            allDroppedPlugins.add(p);
          }
        }
        if (keptOverridePlugins.length !== override.plugins.length) {
          override.plugins = keptOverridePlugins;
        }
      }

      // Override rules.
      if (override.rules) {
        const filtered = filterRulesAgainstNamespaces(override.rules, overrideNamespaces);
        if (Object.keys(filtered).length !== Object.keys(override.rules).length) {
          override.rules = filtered as typeof override.rules;
        }
      }
    }
  }

  // 6. Warn.
  //
  // We deliberately don't try to distinguish "we just removed this
  // package as part of the ESLint-ecosystem cleanup" from "the user
  // never had it installed" — the only honest signal we have is "not
  // in any package.json after cleanup", and a name-based heuristic
  // (matches `eslint-plugin-*`?) misclassifies the @oxlint/migrate
  // phantom-reference case (e.g. `@unocss/eslint-config` translating
  // into `eslint-plugin-unocss` even though the user never had it).
  // A single accurate message covers both paths.
  if (allDroppedJsPlugins.size > 0) {
    warnMigration(
      `Stripped JS plugin reference(s) from the generated lint config: ${[...allDroppedJsPlugins].join(', ')}. ` +
        'No matching package is present in this workspace, so loading them at lint time would fail. ' +
        'If you want their Oxlint coverage back, install each package (e.g. `vp install <name>`) and add its name back to `lint.jsPlugins` in vite.config.ts.',
      report,
    );
  }
  if (allDroppedPlugins.size > 0) {
    warnMigration(
      `Stripped unknown plugin reference(s) from the generated lint config: ${[...allDroppedPlugins].join(', ')}. ` +
        "These aren't native Oxlint plugins and no surviving JS plugin contributes them.",
      report,
    );
  }
}

export function warnPackageLevelEslint() {
  prompts.log.warn(
    'ESLint detected in workspace packages but no root config found. Package-level ESLint must be migrated manually.',
  );
}

// Framework-ESLint integration packages we can't migrate cleanly today.
// When any of these is present, the ESLint migration is skipped entirely
// — the user's ESLint setup stays intact and they get told how to proceed
// manually.
//
// `@nuxt/eslint` is a Nuxt module that loads ESLint at runtime via the
// dev server and writes a generated config to `.nuxt/eslint.config.mjs`,
// which the user's `eslint.config.mjs` re-exports. Migrating it
// produces a broken state: `vite.config.ts` references `@nuxt/eslint-plugin`
// (no longer installed) and `nuxt.config.ts` still tries to load the
// removed module. Track at https://github.com/voidzero-dev/vite-plus/issues
// once an issue exists.
const INCOMPATIBLE_ESLINT_INTEGRATIONS = ['@nuxt/eslint'] as const;

/**
 * Detect framework-ESLint integration packages whose ESLint migration is
 * known to be incompatible. Returns the offending package name, or
 * `undefined` if none is present.
 */
export function detectIncompatibleEslintIntegration(
  projectPath: string,
  packages?: WorkspacePackage[],
): string | undefined {
  const candidates = [projectPath, ...(packages ?? []).map((p) => path.join(projectPath, p.path))];
  for (const candidate of candidates) {
    const pkgJsonPath = path.join(candidate, 'package.json');
    if (!fs.existsSync(pkgJsonPath)) {
      continue;
    }
    let pkg: { devDependencies?: Record<string, string>; dependencies?: Record<string, string> };
    try {
      pkg = readJsonFile(pkgJsonPath);
    } catch {
      continue;
    }
    for (const name of INCOMPATIBLE_ESLINT_INTEGRATIONS) {
      if (pkg.devDependencies?.[name] || pkg.dependencies?.[name]) {
        return name;
      }
    }
  }
  return undefined;
}

export function warnIncompatibleEslintIntegration(name: string): void {
  prompts.log.warn(
    `${name} detected — automatic ESLint migration is skipped. ` +
      `${name} wires ESLint into a framework-specific flow that Vite+ cannot migrate cleanly yet. ` +
      'Your ESLint setup is preserved. ' +
      `To migrate manually, remove ${name} from package.json and re-run \`vp migrate\`.`,
  );
}

export function warnLegacyEslintConfig(legacyConfigFile: string) {
  prompts.log.warn(
    `Legacy ESLint configuration detected (${legacyConfigFile}). ` +
      'Automatic migration to Oxlint requires ESLint v9+ with flat config format (eslint.config.*). ' +
      'Please upgrade to ESLint v9 first: https://eslint.org/docs/latest/use/migrate-to-9.0.0',
  );
}

export async function confirmEslintMigration(interactive: boolean): Promise<boolean> {
  if (interactive) {
    const confirmed = await prompts.confirm({
      message:
        'Migrate ESLint rules to Oxlint using @oxlint/migrate?\n  ' +
        styleText(
          'gray',
          "Oxlint is Vite+'s built-in linter — significantly faster than ESLint with compatible rule support. @oxlint/migrate converts your existing rules automatically.",
        ),
      initialValue: true,
    });
    if (prompts.isCancel(confirmed)) {
      cancelAndExit();
    }
    return confirmed;
  }
  return true;
}

export async function promptEslintMigration(
  projectPath: string,
  interactive: boolean,
  packages?: WorkspacePackage[],
): Promise<boolean> {
  const incompatible = detectIncompatibleEslintIntegration(projectPath, packages);
  if (incompatible) {
    warnIncompatibleEslintIntegration(incompatible);
    return false;
  }
  const eslintProject = detectEslintProject(projectPath, packages);
  if (eslintProject.hasDependency && !eslintProject.configFile && eslintProject.legacyConfigFile) {
    warnLegacyEslintConfig(eslintProject.legacyConfigFile);
    return false;
  }
  if (!eslintProject.hasDependency) {
    return false;
  }
  if (!eslintProject.configFile) {
    // Packages have eslint but no root config → warn and skip
    warnPackageLevelEslint();
    return false;
  }
  const confirmed = await confirmEslintMigration(interactive);
  if (!confirmed) {
    return false;
  }
  const ok = await migrateEslintToOxlint(
    projectPath,
    interactive,
    eslintProject.configFile,
    packages,
  );
  if (!ok) {
    cancelAndExit('ESLint migration failed.', 1);
  }
  return true;
}
