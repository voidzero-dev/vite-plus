import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import spawn from 'cross-spawn';
import semver from 'semver';
import { Scalar, YAMLMap, YAMLSeq } from 'yaml';

import {
  mergeJsonConfig,
  mergeTsdownConfig,
  rewriteScripts,
  rewriteImportsInDirectory,
  type DownloadPackageManagerResult,
} from '../../binding/index.js';
import { PackageManager, type WorkspaceInfo } from '../types/index.js';
import {
  VITE_PLUS_NAME,
  VITE_PLUS_OVERRIDE_PACKAGES,
  VITE_PLUS_VERSION,
} from '../utils/constants.js';
import { editJsonFile, isJsonFile, readJsonFile } from '../utils/json.js';
import { detectPackageMetadata } from '../utils/package.js';
import { displayRelative, rulesDir } from '../utils/path.js';
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

/**
 * Rewrite standalone project to add vite-plus dependencies
 * @param projectPath - The path to the project
 */
export function rewriteStandaloneProject(
  projectPath: string,
  workspaceInfo: WorkspaceInfo,
  skipStagedMigration?: boolean,
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
    if (mergeStagedConfigToViteConfig(projectPath, extractedStagedConfig)) {
      removeLintStagedFromPackageJson(packageJsonPath);
    }
  }

  if (!skipStagedMigration) {
    rewriteLintStagedConfigFile(projectPath);
  }
  mergeViteConfigFiles(projectPath);
  mergeTsdownConfigFile(projectPath);
  // rewrite imports in all TypeScript/JavaScript files
  rewriteAllImports(projectPath);
  // set package manager
  setPackageManager(projectPath, workspaceInfo.downloadPackageManager);
}

/**
 * Rewrite monorepo to add vite-plus dependencies
 * @param workspaceInfo - The workspace info
 */
export function rewriteMonorepo(workspaceInfo: WorkspaceInfo, skipStagedMigration?: boolean): void {
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
    );
  }

  if (!skipStagedMigration) {
    rewriteLintStagedConfigFile(workspaceInfo.rootDir);
  }
  mergeViteConfigFiles(workspaceInfo.rootDir);
  mergeTsdownConfigFile(workspaceInfo.rootDir);
  // rewrite imports in all TypeScript/JavaScript files
  rewriteAllImports(workspaceInfo.rootDir);
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
): void {
  mergeViteConfigFiles(projectPath);
  mergeTsdownConfigFile(projectPath);

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
    if (mergeStagedConfigToViteConfig(projectPath, extractedStagedConfig)) {
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
let cachedPrepareRulesYaml: string | undefined;
function readRulesYaml(): string {
  cachedRulesYaml ??= fs.readFileSync(RULES_YAML_PATH, 'utf8');
  return cachedRulesYaml;
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
    const updated = rewriteScripts(JSON.stringify(pkg.scripts), readRulesYaml());
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
    }
    fs.unlinkSync(configPath);
    prompts.log.success(`✔ Inlined ${displayRelative(configPath)} into "staged" in vite.config.ts`);
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
function ensureViteConfig(projectPath: string, configs: ConfigFiles): string {
  if (!configs.viteConfig) {
    configs.viteConfig = 'vite.config.ts';
    const viteConfigPath = path.join(projectPath, 'vite.config.ts');
    fs.writeFileSync(
      viteConfigPath,
      `import { defineConfig } from '${VITE_PLUS_NAME}';

export default defineConfig({});
`,
    );
    prompts.log.success(`✔ Created vite.config.ts in ${displayRelative(viteConfigPath)}`);
  }
  return configs.viteConfig;
}

/**
 * Merge tsdown.config.* into vite.config.ts
 * - For JSON files: merge content directly into `pack` field and delete the JSON file
 * - For TS/JS files: import the config file
 */
function mergeTsdownConfigFile(projectPath: string): void {
  const configs = detectConfigs(projectPath);
  if (!configs.tsdownConfig) {
    return;
  }
  const viteConfig = ensureViteConfig(projectPath, configs);

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
    prompts.log.success(
      `✔ Added import for ${displayRelative(fullTsdownConfigPath)} in ${displayRelative(fullViteConfigPath)}`,
    );
  }
  // Show documentation link for manual merging since we only added the import
  prompts.log.info(
    `Please manually merge ${displayRelative(fullTsdownConfigPath)} into ${displayRelative(fullViteConfigPath)}, see https://viteplus.dev/migration/#tsdown`,
  );
}

/**
 * Merge oxlint and oxfmt config into vite.config.ts
 */
function mergeViteConfigFiles(projectPath: string): void {
  const configs = detectConfigs(projectPath);
  if (!configs.oxfmtConfig && !configs.oxlintConfig) {
    return;
  }
  const viteConfig = ensureViteConfig(projectPath, configs);
  if (configs.oxlintConfig) {
    // merge oxlint config into vite.config.ts
    mergeAndRemoveJsonConfig(projectPath, viteConfig, configs.oxlintConfig, 'lint');
  }
  if (configs.oxfmtConfig) {
    // merge oxfmt config into vite.config.ts
    mergeAndRemoveJsonConfig(projectPath, viteConfig, configs.oxfmtConfig, 'fmt');
  }
}

function mergeAndRemoveJsonConfig(
  projectPath: string,
  viteConfigPath: string,
  jsonConfigPath: string,
  configKey: string,
): void {
  const fullViteConfigPath = path.join(projectPath, viteConfigPath);
  const fullJsonConfigPath = path.join(projectPath, jsonConfigPath);
  const result = mergeJsonConfig(fullViteConfigPath, fullJsonConfigPath, configKey);
  if (result.updated) {
    fs.writeFileSync(fullViteConfigPath, result.content);
    fs.unlinkSync(fullJsonConfigPath);
    prompts.log.success(
      `✔ Merged ${displayRelative(fullJsonConfigPath)} into ${displayRelative(fullViteConfigPath)}`,
    );
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
    prompts.log.success(`✔ Merged staged config into ${displayRelative(fullViteConfigPath)}`);
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
function rewriteAllImports(projectPath: string): void {
  const result = rewriteImportsInDirectory(projectPath);
  const modified = result.modifiedFiles.length;
  const errors = result.errors.length;

  if (modified > 0) {
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
 */
export function hasUnsupportedHuskyVersion(rootDir: string): boolean {
  const packageJsonPath = path.join(rootDir, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return false;
  }
  const pkg = readJsonFile(packageJsonPath);
  const deps = pkg.devDependencies as Record<string, string> | undefined;
  const prodDeps = pkg.dependencies as Record<string, string> | undefined;
  const huskyVersion = deps?.husky ?? prodDeps?.husky;
  if (!huskyVersion) {
    return false;
  }
  return semver.satisfies(semver.coerce(huskyVersion) ?? '0.0.0', '<9.0.0');
}

const OTHER_HOOK_TOOLS = ['simple-git-hooks', 'lefthook', 'yorkie'] as const;

// Packages replaced by vite-plus built-in commands and should be removed from devDependencies
const REPLACED_HOOK_PACKAGES = ['husky', 'lint-staged'] as const;

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
 * Set up git hooks with husky + lint-staged via vp commands.
 * Skips if another hook tool is detected (warns user).
 */
export function setupGitHooks(projectPath: string): void {
  // Check git root first — subdirectory projects must not set core.hooksPath
  // (running vp config from a subdirectory would hijack the repo-wide hooksPath)
  const gitRoot = findGitRoot(projectPath);
  if (gitRoot && path.resolve(projectPath) !== path.resolve(gitRoot)) {
    prompts.log.warn(
      '⚠ Subdirectory project detected — skipping git hooks setup. Configure hooks at the repository root.',
    );
    return;
  }

  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }

  // Check for other hook tools → warn and skip
  const pkgContent = readJsonFile(packageJsonPath);
  const deps = pkgContent.devDependencies as Record<string, string> | undefined;
  const prodDeps = pkgContent.dependencies as Record<string, string> | undefined;
  for (const tool of OTHER_HOOK_TOOLS) {
    if (deps?.[tool] || prodDeps?.[tool] || pkgContent[tool]) {
      prompts.log.warn(
        `⚠ Detected ${tool} — skipping git hooks setup. Please configure git hooks manually.`,
      );
      return;
    }
  }

  // Check for unsupported husky version (<9.0.0) → warn and skip
  if (hasUnsupportedHuskyVersion(projectPath)) {
    prompts.log.warn(
      '⚠ Detected husky <9.0.0 — please upgrade to husky v9+ first, then re-run migration.',
    );
    return;
  }

  // Skip hook setup if lint-staged config exists in a format that can't be
  // auto-migrated — the config still references old commands (oxlint/oxfmt)
  // that migration can't rewrite, so vp staged would fail on the next commit.
  if (hasUnsupportedLintStagedConfig(projectPath)) {
    prompts.log.warn(
      '⚠ Unsupported lint-staged config format — skipping git hooks setup. Please configure git hooks manually.',
    );
    return;
  }

  // Extract custom hooks dir from the prepare script.
  // By this point rewriteScripts() has already replaced "husky" → "vp config".
  const scripts = pkgContent.scripts as Record<string, string> | undefined;
  const hooksDirMatch = scripts?.prepare?.match(/\bvp\s+config\s+--hooks-dir\s+([^\s&|;]+)/);
  // If migrating from husky (vp config --hooks-dir was set), use that dir.
  // Otherwise default to .vite-hooks for new projects.
  const hooksDir = hooksDirMatch?.[1] ?? '.vite-hooks';

  editJsonFile<{
    scripts?: Record<string, string>;
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
  }>(packageJsonPath, (pkg) => {
    // rewriteScripts() already replaced husky → vp config.
    // Just ensure vp config is present for projects that didn't have husky.
    if (!pkg.scripts) {
      pkg.scripts = {};
    }
    if (!pkg.scripts.prepare) {
      pkg.scripts.prepare = 'vp config';
    } else if (!pkg.scripts.prepare.includes('vp config')) {
      pkg.scripts.prepare = `vp config && ${pkg.scripts.prepare}`;
    }

    // Remove husky and lint-staged from devDependencies (replaced by vp built-in commands).
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

  // Add staged config to vite.config.ts if not present
  let stagedMerged = hasStagedConfigInViteConfig(projectPath);
  if (!stagedMerged && !hasStandaloneLintStagedConfig(projectPath)) {
    stagedMerged = mergeStagedConfigToViteConfig(projectPath, { '*': 'vp check --fix' });
  }

  // Only remove lint-staged key from package.json after staged config is
  // confirmed in vite.config.ts — prevents losing config on merge failure
  if (stagedMerged) {
    removeLintStagedFromPackageJson(packageJsonPath);
  }

  // Hook file creation (no git needed — only filesystem ops)
  createPreCommitHook(projectPath, hooksDir);

  // vp config requires a git workspace — skip if no .git found
  if (!gitRoot) {
    return;
  }

  const vpBin = process.env.VITE_PLUS_CLI_BIN ?? 'vp';

  // Install git hooks via vp config (--hooks-only to skip agent setup, handled by migration)
  const configArgs =
    hooksDir !== '.vite-hooks'
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
    } else {
      prompts.log.success('✔ Git hooks configured');
    }
  } else {
    prompts.log.warn('Failed to install git hooks');
  }
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
 * After rewriting, converts positional dir arg to --hooks-dir flag:
 *   "vp config .config/husky" → "vp config --hooks-dir .config/husky"
 *   "vp config" (was "husky") → "vp config --hooks-dir .husky" (preserve default husky dir)
 * Called only when hooks are being set up (not with --no-hooks).
 */
export function rewritePrepareScript(rootDir: string): void {
  const packageJsonPath = path.join(rootDir, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }

  editJsonFile<{ scripts?: Record<string, string> }>(packageJsonPath, (pkg) => {
    if (!pkg.scripts?.prepare) {
      return pkg;
    }

    // Collapse "husky install" → "husky" so the ast-grep rule
    // produces "vp config" with any directory argument preserved.
    // (Moved from Rust rewrite_script pre-processing)
    let prepare = pkg.scripts.prepare;
    prepare = prepare.replace('husky install ', 'husky ');
    prepare = prepare.replace('husky install', 'husky');

    const prepareJson = JSON.stringify({ prepare });
    const updated = rewriteScripts(prepareJson, readPrepareRulesYaml());
    if (updated) {
      let newPrepare: string = JSON.parse(updated).prepare;
      // Post-processing: convert positional dir arg to --hooks-dir flag,
      // and add --hooks-dir .husky for the default case (user had husky with default dir).
      newPrepare = newPrepare.replace(
        /\bvp config(?:\s+(?!-)([\w./-]+))?/,
        (_match: string, dir: string | undefined) =>
          dir ? `vp config --hooks-dir ${dir}` : 'vp config --hooks-dir .husky',
      );
      pkg.scripts.prepare = newPrepare;
    } else if (prepare !== pkg.scripts.prepare) {
      // Pre-processing changed the script (husky install → husky)
      // but no rule matched — keep the collapsed form
      pkg.scripts.prepare = prepare;
    }
    return pkg;
  });
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
