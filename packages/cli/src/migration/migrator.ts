import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import spawn from 'cross-spawn';
import semver from 'semver';
import { Scalar, YAMLMap, YAMLSeq } from 'yaml';

import {
  mergeJsonConfig,
  mergeTsdownConfig,
  rewriteEslint,
  rewriteScripts,
  rewriteImportsInDirectory,
  type DownloadPackageManagerResult,
} from '../../binding/index.js';
import { PackageManager, type WorkspaceInfo, type WorkspacePackage } from '../types/index.js';
import { runCommandSilently } from '../utils/command.js';
import {
  VITE_PLUS_NAME,
  VITE_PLUS_OVERRIDE_PACKAGES,
  VITE_PLUS_VERSION,
  resolve,
} from '../utils/constants.js';
import { editJsonFile, isJsonFile, readJsonFile } from '../utils/json.js';
import { detectPackageMetadata } from '../utils/package.js';
import { displayRelative, rulesDir } from '../utils/path.js';
import { getSpinner } from '../utils/prompts.js';
import { editYamlFile, scalarString, type YamlDocument } from '../utils/yaml.js';
import { detectConfigs, type ConfigFiles } from './detector.js';

// All known lint-staged config file names.
// JSON-parseable ones come first so rewriteLintStagedConfigFile can rewrite them.
const LINT_STAGED_JSON_CONFIG_FILES = ['.lintstagedrc.json', '.lintstagedrc'] as const;
const LINT_STAGED_OTHER_CONFIG_FILES = [
  '.lintstagedrc.yaml',
  '.lintstagedrc.yml',
  '.lintstagedrc.mjs',
  'lint-staged.config.mjs',
  '.lintstagedrc.cjs',
  'lint-staged.config.cjs',
  '.lintstagedrc.js',
  'lint-staged.config.js',
  '.lintstagedrc.ts',
  'lint-staged.config.ts',
  '.lintstagedrc.mts',
  'lint-staged.config.mts',
  '.lintstagedrc.cts',
  'lint-staged.config.cts',
] as const;
const LINT_STAGED_ALL_CONFIG_FILES = [
  ...LINT_STAGED_JSON_CONFIG_FILES,
  ...LINT_STAGED_OTHER_CONFIG_FILES,
] as const;

// packages that are replaced with vite-plus
const REMOVE_PACKAGES = [
  'oxlint',
  'oxlint-tsgolint',
  'oxfmt',
  'tsdown',
  '@vitest/browser',
  '@vitest/browser-preview',
  '@vitest/browser-playwright',
  '@vitest/browser-webdriverio',
] as const;

export function checkViteVersion(projectPath: string): boolean {
  return checkPackageVersion(projectPath, 'vite', '7.0.0');
}

export function checkVitestVersion(projectPath: string): boolean {
  return checkPackageVersion(projectPath, 'vitest', '4.0.0');
}

/**
 * Check the package version is supported by auto migration
 * @param projectPath - The path to the project
 * @param name - The name of the package
 * @param minVersion - The minimum version of the package
 * @returns true if the package version is supported by auto migration
 */
function checkPackageVersion(projectPath: string, name: string, minVersion: string): boolean {
  const metadata = detectPackageMetadata(projectPath, name);
  if (!metadata || metadata.name !== name) {
    return true;
  }
  if (semver.satisfies(metadata.version, `<${minVersion}`)) {
    const packageJsonFilePath = path.join(projectPath, 'package.json');
    prompts.log.error(
      `✘ ${name}@${metadata.version} in ${displayRelative(packageJsonFilePath)} is not supported by auto migration`,
    );
    prompts.log.info(`Please upgrade ${name} to version >=${minVersion} first`);
    return false;
  }
  return true;
}

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
  const pkg = readJsonFile<{
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
  }>(packageJsonPath);
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
      const wpPkg = readJsonFile<{
        devDependencies?: Record<string, string>;
        dependencies?: Record<string, string>;
      }>(pkgJsonPath);
      if (wpPkg.devDependencies?.eslint || wpPkg.dependencies?.eslint) {
        hasDependency = true;
        break;
      }
    }
  }

  return { hasDependency, configFile, legacyConfigFile };
}

function getInstalledOxlintVersion(): string {
  const oxlintMainPath = resolve('oxlint');
  const oxlintPackageRoot = path.dirname(path.dirname(oxlintMainPath));
  const pkgJson = readJsonFile<{ version: string }>(path.join(oxlintPackageRoot, 'package.json'));
  return pkgJson.version;
}

/**
 * Run a `vp dlx @oxlint/migrate` step with graceful error handling.
 * Returns true on success, false on failure (spawn error or non-zero exit).
 */
async function runOxlintMigrateStep(
  vpBin: string,
  cwd: string,
  migratePackage: string,
  args: string[],
  spinner: ReturnType<typeof getSpinner>,
  failMessage: string,
  manualHint: string,
): Promise<boolean> {
  try {
    const result = await runCommandSilently({
      command: vpBin,
      args: ['dlx', migratePackage, ...args],
      cwd,
      envs: process.env,
    });
    if (result.exitCode !== 0) {
      spinner.stop(failMessage);
      const stderr = result.stderr.toString().trim();
      if (stderr) {
        prompts.log.warn(`⚠ ${stderr}`);
      }
      prompts.log.info(manualHint);
      return false;
    }
    return true;
  } catch {
    spinner.stop(failMessage);
    prompts.log.info(manualHint);
    return false;
  }
}

export async function migrateEslintToOxlint(
  projectPath: string,
  interactive: boolean,
  eslintConfigFile?: string,
  packages?: WorkspacePackage[],
): Promise<boolean> {
  const vpBin = process.env.VITE_PLUS_CLI_BIN ?? 'vp';
  const spinner = getSpinner(interactive);

  // Steps 1-2: Only run @oxlint/migrate if there's an eslint config at root
  if (eslintConfigFile) {
    const migratePackage = `@oxlint/migrate@${getInstalledOxlintVersion()}`;

    // Step 1: Generate .oxlintrc.json from ESLint config
    spinner.start('Migrating ESLint config to Oxlint...');
    const migrateOk = await runOxlintMigrateStep(
      vpBin,
      projectPath,
      migratePackage,
      ['--merge', '--type-aware', '--with-nursery', '--details'],
      spinner,
      'ESLint migration failed',
      `You can run \`vp dlx ${migratePackage} --merge --type-aware --with-nursery --details\` manually later`,
    );
    if (!migrateOk) {
      return false;
    }
    spinner.stop('ESLint config migrated to .oxlintrc.json');

    // Step 2: Replace eslint-disable comments with oxlint-disable
    spinner.start('Replacing ESLint comments with Oxlint equivalents...');
    const replaceOk = await runOxlintMigrateStep(
      vpBin,
      projectPath,
      migratePackage,
      ['--replace-eslint-comments'],
      spinner,
      'ESLint comment replacement failed',
      `You can run \`vp dlx ${migratePackage} --replace-eslint-comments\` manually later`,
    );
    if (replaceOk) {
      spinner.stop('ESLint comments replaced');
    }
    // Continue with cleanup regardless — .oxlintrc.json was generated successfully
  }

  // Step 3: Delete all eslint config files at root
  deleteEslintConfigFiles(projectPath);

  // Step 4: Remove eslint dependency and rewrite eslint scripts (root only)
  rewriteEslintPackageJson(path.join(projectPath, 'package.json'));

  // Step 4b: Rewrite eslint scripts in workspace packages
  if (packages) {
    for (const pkg of packages) {
      rewriteEslintPackageJson(path.join(projectPath, pkg.path, 'package.json'));
    }
  }

  // Step 5: Rewrite eslint references in lint-staged config files
  rewriteEslintLintStagedConfigFiles(projectPath);

  return true;
}

function deleteEslintConfigFiles(basePath: string): void {
  const configs = detectConfigs(basePath);
  for (const file of [configs.eslintConfig, configs.eslintLegacyConfig]) {
    if (file) {
      const configPath = path.join(basePath, file);
      if (fs.existsSync(configPath)) {
        fs.unlinkSync(configPath);
        prompts.log.success(`✔ Removed ${displayRelative(configPath)}`);
      }
    }
  }
}

function rewriteEslintPackageJson(packageJsonPath: string): void {
  editJsonFile<{
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
    scripts?: Record<string, string>;
    'lint-staged'?: Record<string, string | string[]>;
  }>(packageJsonPath, (pkg) => {
    let changed = false;
    if (pkg.devDependencies?.eslint) {
      delete pkg.devDependencies.eslint;
      changed = true;
    }
    if (pkg.dependencies?.eslint) {
      delete pkg.dependencies.eslint;
      changed = true;
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

function rewriteEslintLintStagedConfigFiles(projectPath: string): void {
  for (const filename of LINT_STAGED_JSON_CONFIG_FILES) {
    const configPath = path.join(projectPath, filename);
    if (!fs.existsSync(configPath)) {
      continue;
    }
    if (filename === '.lintstagedrc' && !isJsonFile(configPath)) {
      prompts.log.warn(
        `⚠ ${displayRelative(configPath)} is not JSON — please update eslint references manually`,
      );
      continue;
    }
    editJsonFile<Record<string, string | string[]>>(configPath, (config) => {
      const updated = rewriteEslint(JSON.stringify(config));
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
    prompts.log.warn(`⚠ ${displayRelative(configPath)} — please update eslint references manually`);
  }
}

/**
 * Rewrite standalone project to add vite-plus dependencies
 * @param projectPath - The path to the project
 */
export function rewriteStandaloneProject(
  projectPath: string,
  workspaceInfo: WorkspaceInfo,
  skipStagedMigration?: boolean,
  silent = false,
): void {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }

  const packageManager = workspaceInfo.packageManager;
  let extractedStagedConfig: Record<string, string | string[]> | null = null;
  editJsonFile<{
    overrides?: Record<string, string>;
    resolutions?: Record<string, string>;
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
    scripts?: Record<string, string>;
    pnpm?: {
      overrides?: Record<string, string>;
      // peerDependencyRules?: {
      //   allowAny?: string[];
      //   allowedVersions?: Record<string, string>;
      // };
    };
  }>(packageJsonPath, (pkg) => {
    if (packageManager === PackageManager.yarn) {
      pkg.resolutions = {
        ...pkg.resolutions,
        ...VITE_PLUS_OVERRIDE_PACKAGES,
      };
    } else if (packageManager === PackageManager.npm) {
      pkg.overrides = {
        ...pkg.overrides,
        ...VITE_PLUS_OVERRIDE_PACKAGES,
      };
    } else if (packageManager === PackageManager.pnpm) {
      pkg.pnpm = {
        ...pkg.pnpm,
        overrides: {
          ...pkg.pnpm?.overrides,
          ...VITE_PLUS_OVERRIDE_PACKAGES,
        },
      };
      // remove packages from `resolutions` field if they exist
      // https://pnpm.io/9.x/package_json#resolutions
      for (const key of [...Object.keys(VITE_PLUS_OVERRIDE_PACKAGES), ...REMOVE_PACKAGES]) {
        if (pkg.resolutions?.[key]) {
          delete pkg.resolutions[key];
        }
      }
    }

    extractedStagedConfig = rewritePackageJson(pkg, packageManager, false, skipStagedMigration);

    // ensure vite-plus is in devDependencies
    if (!pkg.devDependencies?.[VITE_PLUS_NAME]) {
      pkg.devDependencies = {
        ...pkg.devDependencies,
        [VITE_PLUS_NAME]: VITE_PLUS_VERSION,
      };
    }
    return pkg;
  });

  // Merge extracted staged config into vite.config.ts, then remove lint-staged from package.json
  if (extractedStagedConfig) {
    if (mergeStagedConfigToViteConfig(projectPath, extractedStagedConfig, silent)) {
      removeLintStagedFromPackageJson(packageJsonPath);
    }
  }

  if (!skipStagedMigration) {
    rewriteLintStagedConfigFile(projectPath);
  }
  mergeViteConfigFiles(projectPath, silent);
  mergeTsdownConfigFile(projectPath, silent);
  // rewrite imports in all TypeScript/JavaScript files
  rewriteAllImports(projectPath, silent);
  // set package manager
  setPackageManager(projectPath, workspaceInfo.downloadPackageManager);
}

/**
 * Rewrite monorepo to add vite-plus dependencies
 * @param workspaceInfo - The workspace info
 */
export function rewriteMonorepo(
  workspaceInfo: WorkspaceInfo,
  skipStagedMigration?: boolean,
  silent = false,
): void {
  // rewrite root workspace
  if (workspaceInfo.packageManager === PackageManager.pnpm) {
    rewritePnpmWorkspaceYaml(workspaceInfo.rootDir);
  } else if (workspaceInfo.packageManager === PackageManager.yarn) {
    rewriteYarnrcYml(workspaceInfo.rootDir);
  }
  rewriteRootWorkspacePackageJson(
    workspaceInfo.rootDir,
    workspaceInfo.packageManager,
    skipStagedMigration,
  );

  // rewrite packages
  for (const pkg of workspaceInfo.packages) {
    rewriteMonorepoProject(
      path.join(workspaceInfo.rootDir, pkg.path),
      workspaceInfo.packageManager,
      skipStagedMigration,
      silent,
    );
  }

  if (!skipStagedMigration) {
    rewriteLintStagedConfigFile(workspaceInfo.rootDir);
  }
  mergeViteConfigFiles(workspaceInfo.rootDir, silent);
  mergeTsdownConfigFile(workspaceInfo.rootDir, silent);
  // rewrite imports in all TypeScript/JavaScript files
  rewriteAllImports(workspaceInfo.rootDir, silent);
  // set package manager
  setPackageManager(workspaceInfo.rootDir, workspaceInfo.downloadPackageManager);
}

/**
 * Rewrite monorepo project to add vite-plus dependencies
 * @param projectPath - The path to the project
 */
export function rewriteMonorepoProject(
  projectPath: string,
  packageManager: PackageManager,
  skipStagedMigration?: boolean,
  silent = false,
): void {
  mergeViteConfigFiles(projectPath, silent);
  mergeTsdownConfigFile(projectPath, silent);

  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }

  let extractedStagedConfig: Record<string, string | string[]> | null = null;
  editJsonFile<{
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
    scripts?: Record<string, string>;
  }>(packageJsonPath, (pkg) => {
    // rewrite scripts in package.json
    extractedStagedConfig = rewritePackageJson(pkg, packageManager, true, skipStagedMigration);
    return pkg;
  });

  // Merge extracted staged config into vite.config.ts, then remove lint-staged from package.json
  if (extractedStagedConfig) {
    if (mergeStagedConfigToViteConfig(projectPath, extractedStagedConfig, silent)) {
      removeLintStagedFromPackageJson(packageJsonPath);
    }
  }
}

/**
 * Rewrite pnpm-workspace.yaml to add vite-plus dependencies
 * @param projectPath - The path to the project
 */
function rewritePnpmWorkspaceYaml(projectPath: string): void {
  const pnpmWorkspaceYamlPath = path.join(projectPath, 'pnpm-workspace.yaml');
  if (!fs.existsSync(pnpmWorkspaceYamlPath)) {
    fs.writeFileSync(pnpmWorkspaceYamlPath, '');
  }

  editYamlFile(pnpmWorkspaceYamlPath, (doc) => {
    // catalog
    rewriteCatalog(doc);

    // overrides
    for (const key of Object.keys(VITE_PLUS_OVERRIDE_PACKAGES)) {
      let version = VITE_PLUS_OVERRIDE_PACKAGES[key];
      if (!version.startsWith('file:')) {
        version = 'catalog:';
      }
      doc.setIn(['overrides', scalarString(key)], scalarString(version));
    }
    // remove dependency selector from vite, e.g. "vite-plugin-svgr>vite": "npm:rolldown-vite@7.0.12"
    const overrides = doc.getIn(['overrides']) as YAMLMap<Scalar<string>, Scalar<string>>;
    for (const item of overrides.items) {
      if (item.key.value.includes('>')) {
        const splits = item.key.value.split('>');
        if (splits[splits.length - 1].trim() === 'vite') {
          overrides.delete(item.key);
        }
      }
    }

    // peerDependencyRules.allowAny
    let allowAny = doc.getIn(['peerDependencyRules', 'allowAny']) as YAMLSeq<Scalar<string>>;
    if (!allowAny) {
      allowAny = new YAMLSeq<Scalar<string>>();
    }
    const existing = new Set(allowAny.items.map((n) => n.value));
    for (const key of Object.keys(VITE_PLUS_OVERRIDE_PACKAGES)) {
      if (!existing.has(key)) {
        allowAny.add(scalarString(key));
      }
    }
    doc.setIn(['peerDependencyRules', 'allowAny'], allowAny);

    // peerDependencyRules.allowedVersions
    let allowedVersions = doc.getIn(['peerDependencyRules', 'allowedVersions']) as YAMLMap<
      Scalar<string>,
      Scalar<string>
    >;
    if (!allowedVersions) {
      allowedVersions = new YAMLMap<Scalar<string>, Scalar<string>>();
    }
    for (const key of Object.keys(VITE_PLUS_OVERRIDE_PACKAGES)) {
      // - vite: '*'
      allowedVersions.set(scalarString(key), scalarString('*'));
    }
    doc.setIn(['peerDependencyRules', 'allowedVersions'], allowedVersions);

    // minimumReleaseAgeExclude
    if (doc.has('minimumReleaseAge')) {
      // add vite-plus, @voidzero-dev/*, oxlint, oxlint-tsgolint, oxfmt to minimumReleaseAgeExclude
      const excludes = [
        'vite-plus',
        '@voidzero-dev/*',
        'oxlint',
        '@oxlint/*',
        'oxlint-tsgolint',
        '@oxlint-tsgolint/*',
        'oxfmt',
        '@oxfmt/*',
      ];
      let minimumReleaseAgeExclude = doc.getIn(['minimumReleaseAgeExclude']) as YAMLSeq<
        Scalar<string>
      >;
      if (!minimumReleaseAgeExclude) {
        minimumReleaseAgeExclude = new YAMLSeq();
      }
      const existing = new Set(minimumReleaseAgeExclude.items.map((n) => n.value));
      for (const exclude of excludes) {
        if (!existing.has(exclude)) {
          minimumReleaseAgeExclude.add(scalarString(exclude));
        }
      }
      doc.setIn(['minimumReleaseAgeExclude'], minimumReleaseAgeExclude);
    }
  });
}

/**
 * Rewrite .yarnrc.yml to add vite-plus dependencies
 * @param projectPath - The path to the project
 */
function rewriteYarnrcYml(projectPath: string): void {
  const yarnrcYmlPath = path.join(projectPath, '.yarnrc.yml');
  if (!fs.existsSync(yarnrcYmlPath)) {
    fs.writeFileSync(yarnrcYmlPath, '');
  }

  editYamlFile(yarnrcYmlPath, (doc) => {
    // catalog
    rewriteCatalog(doc);
  });
}

/**
 * Rewrite catalog in pnpm-workspace.yaml or .yarnrc.yml
 * @param doc - The document to rewrite
 */
function rewriteCatalog(doc: YamlDocument): void {
  for (const [key, value] of Object.entries(VITE_PLUS_OVERRIDE_PACKAGES)) {
    // ERR_PNPM_CATALOG_IN_OVERRIDES  Could not resolve a catalog in the overrides: The entry for 'vite' in catalog 'default' declares a dependency using the 'file' protocol
    // ignore setting catalog if value starts with 'file:'
    if (value.startsWith('file:')) {
      continue;
    }
    doc.setIn(['catalog', key], scalarString(value));
  }
  if (!VITE_PLUS_VERSION.startsWith('file:')) {
    doc.setIn(['catalog', VITE_PLUS_NAME], scalarString(VITE_PLUS_VERSION));
  }
  for (const name of REMOVE_PACKAGES) {
    const path = ['catalog', name];
    if (doc.hasIn(path)) {
      doc.deleteIn(path);
    }
  }

  // TODO: rewrite `catalogs` when OVERRIDE_PACKAGES exists in catalog
}

/**
 * Rewrite root workspace package.json to add vite-plus dependencies
 * @param projectPath - The path to the project
 */
function rewriteRootWorkspacePackageJson(
  projectPath: string,
  packageManager: PackageManager,
  skipStagedMigration?: boolean,
): void {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }

  editJsonFile<{
    resolutions?: Record<string, string>;
    overrides?: Record<string, string>;
    devDependencies?: Record<string, string>;
    pnpm?: {
      overrides?: Record<string, string>;
    };
  }>(packageJsonPath, (pkg) => {
    if (packageManager === PackageManager.yarn) {
      pkg.resolutions = {
        ...pkg.resolutions,
        // FIXME: yarn don't support catalog on resolutions
        // https://github.com/yarnpkg/berry/issues/6979
        ...VITE_PLUS_OVERRIDE_PACKAGES,
      };
    } else if (packageManager === PackageManager.npm) {
      pkg.overrides = {
        ...pkg.overrides,
        ...VITE_PLUS_OVERRIDE_PACKAGES,
      };
    } else if (packageManager === PackageManager.pnpm) {
      // pnpm use overrides field at pnpm-workspace.yaml
      // so we don't need to set overrides field at package.json
      // remove packages from `resolutions` field and `pnpm.overrides` field if they exist
      // https://pnpm.io/9.x/package_json#resolutions
      for (const key of [...Object.keys(VITE_PLUS_OVERRIDE_PACKAGES), ...REMOVE_PACKAGES]) {
        if (pkg.pnpm?.overrides?.[key]) {
          delete pkg.pnpm.overrides[key];
        }
        if (pkg.resolutions?.[key]) {
          delete pkg.resolutions[key];
        }
      }
      // remove dependency selector from vite, e.g. "vite-plugin-svgr>vite": "npm:rolldown-vite@7.0.12"
      for (const key in pkg.pnpm?.overrides) {
        if (key.includes('>')) {
          const splits = key.split('>');
          if (splits[splits.length - 1].trim() === 'vite') {
            delete pkg.pnpm.overrides[key];
          }
        }
      }
    }

    // ensure vite-plus is in devDependencies
    if (!pkg.devDependencies?.[VITE_PLUS_NAME]) {
      pkg.devDependencies = {
        ...pkg.devDependencies,
        [VITE_PLUS_NAME]:
          packageManager === PackageManager.npm || VITE_PLUS_VERSION.startsWith('file:')
            ? VITE_PLUS_VERSION
            : 'catalog:',
      };
    }
    return pkg;
  });

  // rewrite package.json
  rewriteMonorepoProject(projectPath, packageManager, skipStagedMigration);
}

const RULES_YAML_PATH = path.join(rulesDir, 'vite-tools.yml');
const PREPARE_RULES_YAML_PATH = path.join(rulesDir, 'vite-prepare.yml');

// Cache YAML content to avoid repeated disk reads (called once per package in monorepos)
let cachedRulesYaml: string | undefined;
let cachedRulesYamlNoLintStaged: string | undefined;
let cachedPrepareRulesYaml: string | undefined;
function readRulesYaml(): string {
  cachedRulesYaml ??= fs.readFileSync(RULES_YAML_PATH, 'utf8');
  return cachedRulesYaml;
}
function getScriptRulesYaml(skipStagedMigration?: boolean): string {
  const yaml = readRulesYaml();
  if (!skipStagedMigration) {
    return yaml;
  }
  cachedRulesYamlNoLintStaged ??= yaml
    .split('\n\n\n')
    .filter((block) => !block.includes('id: replace-lint-staged'))
    .join('\n\n\n');
  return cachedRulesYamlNoLintStaged;
}
function readPrepareRulesYaml(): string {
  cachedPrepareRulesYaml ??= fs.readFileSync(PREPARE_RULES_YAML_PATH, 'utf8');
  return cachedPrepareRulesYaml;
}

export function rewritePackageJson(
  pkg: {
    scripts?: Record<string, string>;
    'lint-staged'?: Record<string, string | string[]>;
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
  },
  packageManager: PackageManager,
  isMonorepo?: boolean,
  skipStagedMigration?: boolean,
): Record<string, string | string[]> | null {
  if (pkg.scripts) {
    const updated = rewriteScripts(
      JSON.stringify(pkg.scripts),
      getScriptRulesYaml(skipStagedMigration),
    );
    if (updated) {
      pkg.scripts = JSON.parse(updated);
    }
  }
  // Extract staged config from package.json (lint-staged) → will be merged into vite.config.ts.
  // The lint-staged key is NOT deleted here — it's removed by the caller only after
  // the merge into vite.config.ts succeeds, to avoid losing config on merge failure.
  let extractedStagedConfig: Record<string, string | string[]> | null = null;
  if (!skipStagedMigration && pkg['lint-staged']) {
    const config = pkg['lint-staged'];
    const updated = rewriteScripts(JSON.stringify(config), readRulesYaml());
    extractedStagedConfig = updated ? JSON.parse(updated) : config;
  }
  const supportCatalog = isMonorepo && packageManager !== PackageManager.npm;
  let needVitePlus = false;
  for (const [key, version] of Object.entries(VITE_PLUS_OVERRIDE_PACKAGES)) {
    const value = supportCatalog && !version.startsWith('file:') ? 'catalog:' : version;
    if (pkg.devDependencies?.[key]) {
      pkg.devDependencies[key] = value;
      needVitePlus = true;
    }
    if (pkg.dependencies?.[key]) {
      pkg.dependencies[key] = value;
      needVitePlus = true;
    }
  }
  // remove packages that are replaced with vite-plus
  for (const name of REMOVE_PACKAGES) {
    if (pkg.devDependencies?.[name]) {
      delete pkg.devDependencies[name];
      needVitePlus = true;
    }
    if (pkg.dependencies?.[name]) {
      delete pkg.dependencies[name];
      needVitePlus = true;
    }
  }
  if (needVitePlus) {
    // add vite-plus to devDependencies
    const version =
      supportCatalog && !VITE_PLUS_VERSION.startsWith('file:') ? 'catalog:' : VITE_PLUS_VERSION;
    pkg.devDependencies = {
      ...pkg.devDependencies,
      [VITE_PLUS_NAME]: version,
    };
  }
  return extractedStagedConfig;
}

// Remove the "lint-staged" key from package.json after config has been
// successfully merged into vite.config.ts.
function removeLintStagedFromPackageJson(packageJsonPath: string): void {
  editJsonFile<{ 'lint-staged'?: Record<string, string | string[]> }>(packageJsonPath, (pkg) => {
    if (pkg['lint-staged']) {
      delete pkg['lint-staged'];
      return pkg;
    }
    return undefined;
  });
}

// Migrate standalone lint-staged config files into staged in vite.config.ts.
// JSON-parseable files are inlined automatically; non-JSON files get a warning.
function rewriteLintStagedConfigFile(projectPath: string): void {
  let hasUnsupported = false;

  for (const filename of LINT_STAGED_JSON_CONFIG_FILES) {
    const configPath = path.join(projectPath, filename);
    if (!fs.existsSync(configPath)) {
      continue;
    }
    if (filename === '.lintstagedrc' && !isJsonFile(configPath)) {
      prompts.log.warn(
        `✘ ${displayRelative(configPath)} is not JSON format — please migrate to "staged" in vite.config.ts manually`,
      );
      hasUnsupported = true;
      continue;
    }
    // Merge the JSON config into vite.config.ts as "staged" and delete the file.
    // Skip if staged already exists in vite.config.ts (already migrated by rewritePackageJson).
    if (!hasStagedConfigInViteConfig(projectPath)) {
      const config = readJsonFile(configPath);
      const updated = rewriteScripts(JSON.stringify(config), readRulesYaml());
      const finalConfig = updated ? JSON.parse(updated) : config;
      if (!mergeStagedConfigToViteConfig(projectPath, finalConfig)) {
        // Merge failed — preserve the original config file so the user doesn't lose their rules
        continue;
      }
      fs.unlinkSync(configPath);
      prompts.log.success(
        `✔ Inlined ${displayRelative(configPath)} into "staged" in vite.config.ts`,
      );
    } else {
      prompts.log.warn(
        `⚠ ${displayRelative(configPath)} found but "staged" already exists in vite.config.ts — please merge manually`,
      );
    }
  }
  // Non-JSON standalone files — warn
  for (const filename of LINT_STAGED_OTHER_CONFIG_FILES) {
    const configPath = path.join(projectPath, filename);
    if (!fs.existsSync(configPath)) {
      continue;
    }
    prompts.log.warn(
      `✘ ${displayRelative(configPath)} — please migrate to "staged" in vite.config.ts manually`,
    );
    hasUnsupported = true;
  }
  if (hasUnsupported) {
    prompts.log.warn(
      `Only "staged" in vite.config.ts is supported. See https://viteplus.dev/migration/#lint-staged`,
    );
  }
}

/**
 * Ensure vite.config.ts exists, create it if not
 * @returns The vite config filename
 */
function ensureViteConfig(projectPath: string, configs: ConfigFiles, silent = false): string {
  if (!configs.viteConfig) {
    configs.viteConfig = 'vite.config.ts';
    const viteConfigPath = path.join(projectPath, 'vite.config.ts');
    fs.writeFileSync(
      viteConfigPath,
      `import { defineConfig } from '${VITE_PLUS_NAME}';

export default defineConfig({});
`,
    );
    if (!silent) {
      prompts.log.success(`✔ Created vite.config.ts in ${displayRelative(viteConfigPath)}`);
    }
  }
  return configs.viteConfig;
}

/**
 * Merge tsdown.config.* into vite.config.ts
 * - For JSON files: merge content directly into `pack` field and delete the JSON file
 * - For TS/JS files: import the config file
 */
function mergeTsdownConfigFile(projectPath: string, silent = false): void {
  const configs = detectConfigs(projectPath);
  if (!configs.tsdownConfig) {
    return;
  }
  const viteConfig = ensureViteConfig(projectPath, configs, silent);

  const fullViteConfigPath = path.join(projectPath, viteConfig);
  const fullTsdownConfigPath = path.join(projectPath, configs.tsdownConfig);

  // For JSON files, merge content directly and delete the file
  if (configs.tsdownConfig.endsWith('.json')) {
    mergeAndRemoveJsonConfig(projectPath, viteConfig, configs.tsdownConfig, 'pack');
    return;
  }

  // For TS/JS files, import the config file
  const tsdownRelativePath = `./${configs.tsdownConfig}`;
  const result = mergeTsdownConfig(fullViteConfigPath, tsdownRelativePath);
  if (result.updated) {
    fs.writeFileSync(fullViteConfigPath, result.content);
    if (!silent) {
      prompts.log.success(
        `✔ Added import for ${displayRelative(fullTsdownConfigPath)} in ${displayRelative(fullViteConfigPath)}`,
      );
    }
  }
  // Show documentation link for manual merging since we only added the import
  if (!silent) {
    prompts.log.info(
      `Please manually merge ${displayRelative(fullTsdownConfigPath)} into ${displayRelative(fullViteConfigPath)}, see https://viteplus.dev/migration/#tsdown`,
    );
  }
}

/**
 * Merge oxlint and oxfmt config into vite.config.ts
 */
export function mergeViteConfigFiles(projectPath: string, silent = false): void {
  const configs = detectConfigs(projectPath);
  if (!configs.oxfmtConfig && !configs.oxlintConfig) {
    return;
  }
  const viteConfig = ensureViteConfig(projectPath, configs, silent);
  if (configs.oxlintConfig) {
    // merge oxlint config into vite.config.ts
    mergeAndRemoveJsonConfig(projectPath, viteConfig, configs.oxlintConfig, 'lint', silent);
  }
  if (configs.oxfmtConfig) {
    // merge oxfmt config into vite.config.ts
    mergeAndRemoveJsonConfig(projectPath, viteConfig, configs.oxfmtConfig, 'fmt', silent);
  }
}

function mergeAndRemoveJsonConfig(
  projectPath: string,
  viteConfigPath: string,
  jsonConfigPath: string,
  configKey: string,
  silent = false,
): void {
  const fullViteConfigPath = path.join(projectPath, viteConfigPath);
  const fullJsonConfigPath = path.join(projectPath, jsonConfigPath);
  const result = mergeJsonConfig(fullViteConfigPath, fullJsonConfigPath, configKey);
  if (result.updated) {
    fs.writeFileSync(fullViteConfigPath, result.content);
    fs.unlinkSync(fullJsonConfigPath);
    if (!silent) {
      prompts.log.success(
        `✔ Merged ${displayRelative(fullJsonConfigPath)} into ${displayRelative(fullViteConfigPath)}`,
      );
    }
  } else {
    prompts.log.warn(
      `✘ Failed to merge ${displayRelative(fullJsonConfigPath)} into ${displayRelative(fullViteConfigPath)}`,
    );
    prompts.log.info(
      `Please complete the merge manually and follow the instructions in the documentation: https://viteplus.dev/config/`,
    );
  }
}

/**
 * Merge a staged config object into vite.config.ts as `staged: { ... }`.
 * Writes the config to a temp JSON file, calls mergeJsonConfig NAPI, then cleans up.
 */
function mergeStagedConfigToViteConfig(
  projectPath: string,
  stagedConfig: Record<string, string | string[]>,
  silent = false,
): boolean {
  const configs = detectConfigs(projectPath);
  const viteConfig = ensureViteConfig(projectPath, configs);
  const fullViteConfigPath = path.join(projectPath, viteConfig);

  // Write staged config to a temp JSON file for mergeJsonConfig NAPI
  const tempJsonPath = path.join(projectPath, '.staged-config-temp.json');
  fs.writeFileSync(tempJsonPath, JSON.stringify(stagedConfig, null, 2));

  let result;
  try {
    result = mergeJsonConfig(fullViteConfigPath, tempJsonPath, 'staged');
  } finally {
    fs.unlinkSync(tempJsonPath);
  }

  if (result.updated) {
    fs.writeFileSync(fullViteConfigPath, result.content);
    if (!silent) {
      prompts.log.success(`✔ Merged staged config into ${displayRelative(fullViteConfigPath)}`);
    }
    return true;
  } else {
    prompts.log.warn(`✘ Failed to merge staged config into ${displayRelative(fullViteConfigPath)}`);
    prompts.log.info(
      `Please add staged config to ${displayRelative(fullViteConfigPath)} manually, see https://viteplus.dev/config/`,
    );
    return false;
  }
}

/**
 * Check if vite.config.ts already has a `staged` config key.
 */
function hasStagedConfigInViteConfig(projectPath: string): boolean {
  const configs = detectConfigs(projectPath);
  if (!configs.viteConfig) {
    return false;
  }
  const viteConfigPath = path.join(projectPath, configs.viteConfig);
  const content = fs.readFileSync(viteConfigPath, 'utf8');
  return /\bstaged\s*:/.test(content);
}

/**
 * Rewrite imports in all TypeScript/JavaScript files under a directory
 * This rewrites vite/vitest imports to @voidzero-dev/vite-plus
 * @param projectPath - The root directory to search for files
 */
function rewriteAllImports(projectPath: string, silent = false): void {
  const result = rewriteImportsInDirectory(projectPath);
  const modified = result.modifiedFiles.length;
  const errors = result.errors.length;

  if (!silent && modified > 0) {
    prompts.log.success(`Rewrote imports in ${modified === 1 ? 'one file' : `${modified} files`}`);
    prompts.log.info(result.modifiedFiles.map((file) => `  ${displayRelative(file)}`).join('\n'));
  }

  if (errors > 0) {
    prompts.log.warn(`⚠ ${errors === 1 ? 'one file had an error' : `${errors} files had errors`}:`);
    for (const error of result.errors) {
      prompts.log.error(`  ${displayRelative(error.path)}: ${error.message}`);
    }
  }
}

/**
 * Check if the project has an unsupported husky version (<9.0.0).
 * Uses `semver.coerce` to handle ranges like `^8.0.0` → `8.0.0`.
 * Accepts pre-loaded deps to avoid re-reading package.json when called
 * from contexts that already parsed it.
 */
function checkUnsupportedHuskyVersion(
  deps: Record<string, string> | undefined,
  prodDeps: Record<string, string> | undefined,
): boolean {
  const huskyVersion = deps?.husky ?? prodDeps?.husky;
  if (!huskyVersion) {
    return false;
  }
  return semver.satisfies(semver.coerce(huskyVersion) ?? '0.0.0', '<9.0.0');
}

const OTHER_HOOK_TOOLS = ['simple-git-hooks', 'lefthook', 'yorkie'] as const;

// Packages replaced by vite-plus built-in commands and should be removed from devDependencies
const REPLACED_HOOK_PACKAGES = ['husky', 'lint-staged'] as const;

function removeReplacedHookPackages(packageJsonPath: string): void {
  editJsonFile<{
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
  }>(packageJsonPath, (pkg) => {
    for (const name of REPLACED_HOOK_PACKAGES) {
      if (pkg.devDependencies?.[name]) {
        delete pkg.devDependencies[name];
      }
      if (pkg.dependencies?.[name]) {
        delete pkg.dependencies[name];
      }
    }
    return pkg;
  });
}

/**
 * Walk up from `startPath` looking for `.git` (directory or file — submodules
 * use a `.git` file).  Returns the directory that contains `.git`, or `null`.
 */
function findGitRoot(startPath: string): string | null {
  let dir = startPath;
  while (true) {
    if (fs.existsSync(path.join(dir, '.git'))) {
      return dir;
    }
    const parent = path.dirname(dir);
    if (parent === dir) {
      return null;
    }
    dir = parent;
  }
}

/**
 * Normalize "husky install [dir]" → "husky [dir]" so downstream regex
 * and ast-grep rules can match a single pattern.
 */
function collapseHuskyInstall(script: string): string {
  return script.replace('husky install ', 'husky ').replace('husky install', 'husky');
}

/**
 * High-level helper: detect old hooks dir, set up git hooks, and rewrite
 * the prepare script.  Returns true if hooks were successfully installed.
 */
export function installGitHooks(projectPath: string, silent = false): boolean {
  const oldHooksDir = getOldHooksDir(projectPath);
  if (setupGitHooks(projectPath, oldHooksDir, silent)) {
    rewritePrepareScript(projectPath);
    return true;
  }
  return false;
}

/**
 * Read-only probe: extract the old husky hooks directory from `scripts.prepare`
 * without modifying package.json. Returns undefined when no husky reference is found.
 */
export function getOldHooksDir(rootDir: string): string | undefined {
  const packageJsonPath = path.join(rootDir, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }
  const pkg = readJsonFile<{ scripts?: { prepare?: string } }>(packageJsonPath);
  if (!pkg.scripts?.prepare) {
    return;
  }
  const prepare = collapseHuskyInstall(pkg.scripts.prepare);
  const match = prepare.match(/\bhusky(?:\s+([\w./-]+))?/);
  if (!match) {
    return;
  }
  return match[1] ?? '.husky';
}

/**
 * Pre-flight check: verify that git hooks can be set up for this project.
 * Returns `null` if hooks setup can proceed, or a warning reason string
 * explaining why hooks setup should be skipped.
 *
 * These checks are deterministic and read-only — they do not modify
 * the project in any way, making them safe to call before migration.
 */
export function preflightGitHooksSetup(projectPath: string): string | null {
  const gitRoot = findGitRoot(projectPath);
  if (gitRoot && path.resolve(projectPath) !== path.resolve(gitRoot)) {
    return 'Subdirectory project detected — skipping git hooks setup. Configure hooks at the repository root.';
  }
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return null; // silently skip
  }
  const pkgContent = readJsonFile(packageJsonPath);
  const deps = pkgContent.devDependencies as Record<string, string> | undefined;
  const prodDeps = pkgContent.dependencies as Record<string, string> | undefined;
  for (const tool of OTHER_HOOK_TOOLS) {
    if (deps?.[tool] || prodDeps?.[tool] || pkgContent[tool]) {
      return `Detected ${tool} — skipping git hooks setup. Please configure git hooks manually.`;
    }
  }
  if (checkUnsupportedHuskyVersion(deps, prodDeps)) {
    return 'Detected husky <9.0.0 — please upgrade to husky v9+ first, then re-run migration.';
  }
  if (hasUnsupportedLintStagedConfig(projectPath)) {
    return 'Unsupported lint-staged config format — skipping git hooks setup. Please configure git hooks manually.';
  }
  return null;
}

/**
 * Set up git hooks with husky + lint-staged via vp commands.
 * Skips if another hook tool is detected (warns user).
 * Returns true if hooks were successfully set up, false if skipped.
 */
export function setupGitHooks(projectPath: string, oldHooksDir?: string, silent = false): boolean {
  const reason = preflightGitHooksSetup(projectPath);
  if (reason) {
    prompts.log.warn(`⚠ ${reason}`);
    return false;
  }

  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return false;
  }

  const gitRoot = findGitRoot(projectPath);

  // Custom husky dirs (e.g. .config/husky) stay unchanged;
  // only the default .husky dir gets migrated to .vite-hooks.
  const isCustomDir = oldHooksDir != null && oldHooksDir !== '.husky';
  const hooksDir = isCustomDir ? oldHooksDir : '.vite-hooks';

  editJsonFile<{
    scripts?: Record<string, string>;
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
  }>(packageJsonPath, (pkg) => {
    // Ensure vp config is present for projects that didn't have husky.
    // Skip when prepare contains "husky" — rewritePrepareScript (called after
    // setupGitHooks succeeds) will transform husky → vp config.
    if (!pkg.scripts) {
      pkg.scripts = {};
    }
    if (!pkg.scripts.prepare) {
      pkg.scripts.prepare = 'vp config';
    } else if (
      !pkg.scripts.prepare.includes('vp config') &&
      !/\bhusky\b/.test(pkg.scripts.prepare)
    ) {
      pkg.scripts.prepare = `vp config && ${pkg.scripts.prepare}`;
    }

    return pkg;
  });

  // Add staged config to vite.config.ts if not present
  let stagedMerged = hasStagedConfigInViteConfig(projectPath);
  const hasStandaloneConfig = hasStandaloneLintStagedConfig(projectPath);
  if (!stagedMerged && !hasStandaloneConfig) {
    // Use lint-staged config from package.json if available, otherwise use default
    const pkgData = readJsonFile<{ 'lint-staged'?: Record<string, string | string[]> }>(
      packageJsonPath,
    );
    const stagedConfig = pkgData?.['lint-staged'] ?? { '*': 'vp check --fix' };
    const updated = rewriteScripts(JSON.stringify(stagedConfig), readRulesYaml());
    const finalConfig: Record<string, string | string[]> = updated
      ? JSON.parse(updated)
      : stagedConfig;
    stagedMerged = mergeStagedConfigToViteConfig(projectPath, finalConfig, silent);
  }

  // Only remove lint-staged key from package.json after staged config is
  // confirmed in vite.config.ts — prevents losing config on merge failure
  if (stagedMerged) {
    removeLintStagedFromPackageJson(packageJsonPath);
  }

  // Copy default .husky/ hooks to .vite-hooks/ before creating pre-commit hook.
  // Custom dirs (e.g. .config/husky) are kept in-place — no copy needed.
  if (oldHooksDir && !isCustomDir) {
    const oldDir = path.join(projectPath, oldHooksDir);
    if (fs.existsSync(oldDir)) {
      const targetDir = path.join(projectPath, hooksDir);
      fs.mkdirSync(targetDir, { recursive: true });
      for (const entry of fs.readdirSync(oldDir, { withFileTypes: true })) {
        if (entry.isDirectory() || entry.name.startsWith('.')) {
          continue;
        }
        const src = path.join(oldDir, entry.name);
        const dest = path.join(targetDir, entry.name);
        fs.copyFileSync(src, dest);
        fs.chmodSync(dest, 0o755);
      }
      // Remove old .husky/ directory after copying hooks to .vite-hooks/
      fs.rmSync(oldDir, { recursive: true, force: true });
    }
  }

  // Only create pre-commit hook if staged config was merged into vite.config.ts.
  // Standalone lint-staged config files are NOT sufficient — `vp staged` only
  // reads from vite.config.ts, so a hook without merged config would fail.
  if (stagedMerged) {
    createPreCommitHook(projectPath, hooksDir);
  }

  // vp config requires a git workspace — skip if no .git found
  if (!gitRoot) {
    removeReplacedHookPackages(packageJsonPath);
    return true;
  }

  // Clear husky's core.hooksPath so vp config can set the new one.
  // Only clear if it matches the old husky directory — preserve genuinely custom paths.
  if (oldHooksDir) {
    const checkResult = spawn.sync('git', ['config', '--local', 'core.hooksPath'], {
      cwd: projectPath,
      stdio: 'pipe',
    });
    const existingPath = checkResult.status === 0 ? checkResult.stdout?.toString().trim() : '';
    if (existingPath === `${oldHooksDir}/_` || existingPath === oldHooksDir) {
      spawn.sync('git', ['config', '--local', '--unset', 'core.hooksPath'], {
        cwd: projectPath,
        stdio: 'pipe',
      });
    }
  }

  const vpBin = process.env.VITE_PLUS_CLI_BIN ?? 'vp';

  // Install git hooks via vp config (--hooks-only to skip agent setup, handled by migration)
  const configArgs = isCustomDir
    ? ['config', '--hooks-only', '--hooks-dir', hooksDir]
    : ['config', '--hooks-only'];
  const configResult = spawn.sync(vpBin, configArgs, {
    cwd: projectPath,
    stdio: 'pipe',
  });
  if (configResult.status === 0) {
    // vp config outputs skip/info messages to stdout via log().
    // An empty message means hooks were installed successfully;
    // any non-empty output indicates a skip (HUSKY=0, hooksPath
    // already set, .git not found, etc.).
    const stdout = configResult.stdout?.toString().trim() ?? '';
    if (stdout) {
      prompts.log.warn(`⚠ Git hooks not configured — ${stdout}`);
      return false;
    }
    removeReplacedHookPackages(packageJsonPath);
    if (!silent) {
      prompts.log.success('✔ Git hooks configured');
    }
    return true;
  }
  prompts.log.warn('Failed to install git hooks');
  return false;
}

/**
 * Check if a standalone lint-staged config file exists
 */
function hasStandaloneLintStagedConfig(projectPath: string): boolean {
  return LINT_STAGED_ALL_CONFIG_FILES.some((file) => fs.existsSync(path.join(projectPath, file)));
}

/**
 * Check if a standalone lint-staged config exists in a format that can't be
 * auto-migrated to "staged" in vite.config.ts (non-JSON files like .yaml,
 * .mjs, .cjs, .js, or a non-JSON .lintstagedrc).
 */
function hasUnsupportedLintStagedConfig(projectPath: string): boolean {
  for (const filename of LINT_STAGED_OTHER_CONFIG_FILES) {
    if (fs.existsSync(path.join(projectPath, filename))) {
      return true;
    }
  }
  const lintstagedrcPath = path.join(projectPath, '.lintstagedrc');
  if (fs.existsSync(lintstagedrcPath) && !isJsonFile(lintstagedrcPath)) {
    return true;
  }
  return false;
}

/**
 * Create pre-commit hook file in the hooks directory.
 */
// Lint-staged invocation patterns — replaced in-place with `vp staged`.
// The optional prefix group captures env var assignments like `NODE_OPTIONS=... `.
// We still detect old lint-staged patterns to migrate existing hooks.
const STALE_LINT_STAGED_PATTERNS = [
  /^((?:[A-Z_][A-Z0-9_]*(?:=\S*)?\s+)*)(pnpm|pnpm exec|npx|yarn|yarn run|npm exec|npm run|bunx|bun run|bun x)\s+lint-staged\b/,
  /^((?:[A-Z_][A-Z0-9_]*(?:=\S*)?\s+)*)lint-staged\b/,
];

export function createPreCommitHook(projectPath: string, dir = '.vite-hooks'): void {
  const huskyDir = path.join(projectPath, dir);
  fs.mkdirSync(huskyDir, { recursive: true });
  const hookPath = path.join(huskyDir, 'pre-commit');
  if (fs.existsSync(hookPath)) {
    const existing = fs.readFileSync(hookPath, 'utf8');
    if (existing.includes('vp staged')) {
      return; // already has vp staged
    }
    // Replace old lint-staged invocations in-place, preserve everything else
    const lines = existing.split('\n');
    let replaced = false;
    const result: string[] = [];
    for (const line of lines) {
      const trimmed = line.trim();
      if (!replaced) {
        let matched = false;
        for (const pattern of STALE_LINT_STAGED_PATTERNS) {
          const match = pattern.exec(trimmed);
          if (match) {
            // Preserve env var prefix (capture group 1) and flags/chained commands after lint-staged
            const envPrefix = match[1]?.trim() ?? '';
            const rest = trimmed.slice(match[0].length).trim();
            const parts = [envPrefix, 'vp staged', rest].filter(Boolean);
            result.push(parts.join(' '));
            replaced = true;
            matched = true;
            break;
          }
        }
        if (matched) {
          continue;
        }
      }
      result.push(line);
    }
    if (!replaced) {
      // No lint-staged line found — append after existing content
      fs.writeFileSync(hookPath, `${result.join('\n').trimEnd()}\nvp staged\n`);
    } else {
      fs.writeFileSync(hookPath, result.join('\n'));
    }
  } else {
    fs.writeFileSync(hookPath, 'vp staged\n');
    fs.chmodSync(hookPath, 0o755);
  }
}

/**
 * Rewrite only `scripts.prepare` in the root package.json using vite-prepare.yml rules.
 * Collapses "husky install" → "husky" before applying ast-grep so that the
 * replace-husky rule produces "vp config" with any directory argument preserved.
 * Returns the old husky hooks dir (if any) for migration to .vite-hooks.
 * Called only when hooks are being set up (not with --no-hooks).
 */
export function rewritePrepareScript(rootDir: string): string | undefined {
  const packageJsonPath = path.join(rootDir, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }

  let oldDir: string | undefined;

  editJsonFile<{ scripts?: Record<string, string> }>(packageJsonPath, (pkg) => {
    if (!pkg.scripts?.prepare) {
      return pkg;
    }

    // Collapse "husky install" → "husky" so the ast-grep rule
    // produces "vp config" with any directory argument preserved.
    const prepare = collapseHuskyInstall(pkg.scripts.prepare);

    const prepareJson = JSON.stringify({ prepare });
    const updated = rewriteScripts(prepareJson, readPrepareRulesYaml());
    if (updated) {
      let newPrepare: string = JSON.parse(updated).prepare;
      newPrepare = newPrepare.replace(
        /\bvp config(?:\s+(?!-)([\w./-]+))?/,
        (_match: string, dir: string | undefined) => {
          // Capture the old husky dir for hook migration.
          // Default husky dir is .husky; custom dirs keep --hooks-dir flag.
          oldDir = dir ?? '.husky';
          return dir ? `vp config --hooks-dir ${dir}` : 'vp config';
        },
      );
      pkg.scripts.prepare = newPrepare;
    } else if (prepare !== pkg.scripts.prepare) {
      // Pre-processing changed the script (husky install → husky)
      // but no rule matched — keep the collapsed form
      pkg.scripts.prepare = prepare;
    }
    return pkg;
  });

  return oldDir;
}

function setPackageManager(
  projectDir: string,
  downloadPackageManager: DownloadPackageManagerResult,
) {
  // set package manager
  editJsonFile<{ packageManager?: string }>(path.join(projectDir, 'package.json'), (pkg) => {
    if (!pkg.packageManager) {
      pkg.packageManager = `${downloadPackageManager.name}@${downloadPackageManager.version}`;
    }
    return pkg;
  });
}
