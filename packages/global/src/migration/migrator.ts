import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@clack/prompts';
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
  scalarString,
  editJsonFile,
  editYamlFile,
  rulesDir,
  type YamlDocument,
  isJsonFile,
  displayRelative,
  detectPackageMetadata,
  VITE_PLUS_NAME,
  VITE_PLUS_VERSION,
  VITE_PLUS_OVERRIDE_PACKAGES,
} from '../utils/index.js';
import { detectConfigs, type ConfigFiles } from './detector.js';

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
      `❌ ${name}@${metadata.version} in ${displayRelative(packageJsonFilePath)} is not supported by auto migration`,
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
export function rewriteStandaloneProject(projectPath: string, workspaceInfo: WorkspaceInfo): void {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }

  const packageManager = workspaceInfo.packageManager;
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

    rewritePackageJson(pkg, packageManager);

    // ensure vite-plus is in devDependencies
    if (!pkg.devDependencies?.[VITE_PLUS_NAME]) {
      pkg.devDependencies = {
        ...pkg.devDependencies,
        [VITE_PLUS_NAME]: VITE_PLUS_VERSION,
      };
    }
    return pkg;
  });

  // set .npmrc to use vite-plus
  rewriteNpmrc(projectPath);
  rewriteLintStagedConfigFile(projectPath);
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
export function rewriteMonorepo(workspaceInfo: WorkspaceInfo): void {
  // rewrite root workspace
  if (workspaceInfo.packageManager === PackageManager.pnpm) {
    rewritePnpmWorkspaceYaml(workspaceInfo.rootDir);
  } else if (workspaceInfo.packageManager === PackageManager.yarn) {
    rewriteYarnrcYml(workspaceInfo.rootDir);
  }
  rewriteRootWorkspacePackageJson(workspaceInfo.rootDir, workspaceInfo.packageManager);

  // rewrite packages
  for (const pkg of workspaceInfo.packages) {
    rewriteMonorepoProject(
      path.join(workspaceInfo.rootDir, pkg.path),
      workspaceInfo.packageManager,
    );
  }

  // set .npmrc to use vite-plus
  rewriteNpmrc(workspaceInfo.rootDir);
  rewriteLintStagedConfigFile(workspaceInfo.rootDir);
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
export function rewriteMonorepoProject(projectPath: string, packageManager: PackageManager): void {
  mergeViteConfigFiles(projectPath);
  mergeTsdownConfigFile(projectPath);

  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }

  editJsonFile<{
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
    scripts?: Record<string, string>;
  }>(packageJsonPath, (pkg) => {
    // rewrite scripts in package.json
    rewritePackageJson(pkg, packageManager, true);
    return pkg;
  });
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
      doc.setIn(['overrides', scalarString(key)], scalarString('catalog:'));
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
      // add @voidzero-dev/*, oxlint, oxlint-tsgolint, oxfmt to minimumReleaseAgeExclude
      const excludes = [
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

    // TODO: remove this when vite-plus is released to npm
    // npmScopes:
    //   voidzero-dev:
    //     npmRegistryServer: 'https://npm.pkg.github.com'
    //     npmAuthToken: '${GITHUB_TOKEN}'
    doc.setIn(
      ['npmScopes', 'voidzero-dev', 'npmRegistryServer'],
      scalarString('https://npm.pkg.github.com'),
    );
    // don't set if it already exists
    if (!doc.getIn(['npmScopes', 'voidzero-dev', 'npmAuthToken'])) {
      doc.setIn(['npmScopes', 'voidzero-dev', 'npmAuthToken'], scalarString('${GITHUB_TOKEN}'));
    }
  });
}

/**
 * Rewrite catalog in pnpm-workspace.yaml or .yarnrc.yml
 * @param doc - The document to rewrite
 */
function rewriteCatalog(doc: YamlDocument): void {
  for (const [key, value] of Object.entries(VITE_PLUS_OVERRIDE_PACKAGES)) {
    doc.setIn(['catalog', key], scalarString(value));
  }
  doc.setIn(['catalog', VITE_PLUS_NAME], scalarString(VITE_PLUS_VERSION));
  for (const name of REMOVE_PACKAGES) {
    doc.deleteIn(['catalog', name]);
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
        [VITE_PLUS_NAME]: packageManager === PackageManager.npm ? VITE_PLUS_VERSION : 'catalog:',
      };
    }
    return pkg;
  });

  // rewrite package.json
  rewriteMonorepoProject(projectPath, packageManager);
}

const RULES_YAML_PATH = path.join(rulesDir, 'vite-tools.yml');

export function rewritePackageJson(
  pkg: {
    scripts?: Record<string, string>;
    'lint-staged'?: Record<string, string | string[]>;
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
  },
  packageManager: PackageManager,
  isMonorepo?: boolean,
): void {
  if (pkg.scripts) {
    const updated = rewriteScripts(
      JSON.stringify(pkg.scripts),
      fs.readFileSync(RULES_YAML_PATH, 'utf8'),
    );
    if (updated) {
      pkg.scripts = JSON.parse(updated);
    }
  }
  if (pkg['lint-staged']) {
    const updated = rewriteScripts(
      JSON.stringify(pkg['lint-staged']),
      fs.readFileSync(RULES_YAML_PATH, 'utf8'),
    );
    if (updated) {
      pkg['lint-staged'] = JSON.parse(updated);
    }
  }
  const supportCatalog = isMonorepo && packageManager !== PackageManager.npm;
  let needVitePlus = false;
  for (const [key, version] of Object.entries(VITE_PLUS_OVERRIDE_PACKAGES)) {
    const value = supportCatalog ? 'catalog:' : version;
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
    const version = supportCatalog ? 'catalog:' : VITE_PLUS_VERSION;
    pkg.devDependencies = {
      ...pkg.devDependencies,
      [VITE_PLUS_NAME]: version,
    };
  }
}

// https://github.com/lint-staged/lint-staged#configuration
// only support json format
function rewriteLintStagedConfigFile(projectPath: string): void {
  let hasUnsupported = false;
  const filenames = ['.lintstagedrc.json', '.lintstagedrc'];
  for (const filename of filenames) {
    const lintStagedConfigJsonPath = path.join(projectPath, filename);
    if (!fs.existsSync(lintStagedConfigJsonPath)) {
      continue;
    }
    if (filename === '.lintstagedrc' && !isJsonFile(lintStagedConfigJsonPath)) {
      prompts.log.warn(
        `❌ ${displayRelative(lintStagedConfigJsonPath)} is not JSON format file, auto migration is not supported`,
      );
      hasUnsupported = true;
      continue;
    }
    editJsonFile<Record<string, string | string[]>>(lintStagedConfigJsonPath, (config) => {
      const updated = rewriteScripts(
        JSON.stringify(config),
        fs.readFileSync(RULES_YAML_PATH, 'utf8'),
      );
      if (updated) {
        prompts.log.success(
          `✅ Rewrote lint-staged config in ${displayRelative(lintStagedConfigJsonPath)}`,
        );
        return JSON.parse(updated);
      }
    });
  }
  // others non-json files
  const others = [
    '.lintstagedrc.yaml',
    '.lintstagedrc.yml',
    'lintstagedrc.mjs',
    'lint-staged.config.mjs',
    'lintstagedrc.cjs',
    'lint-staged.config.cjs',
    '.lintstagedrc.js',
    'lint-staged.config.js',
  ];
  for (const filename of others) {
    const lintStagedConfigPath = path.join(projectPath, filename);
    if (!fs.existsSync(lintStagedConfigPath)) {
      continue;
    }
    prompts.log.warn(
      `❌ ${displayRelative(lintStagedConfigPath)} is not supported by auto migration`,
    );
    hasUnsupported = true;
  }
  if (hasUnsupported) {
    prompts.log.warn(
      `Please migrate the lint-staged config manually, see https://viteplus.dev/migration/#lint-staged for more details`,
    );
  }
}

// TODO: should remove this function after vite-plus is released to npm
/**
 * Rewrite .npmrc to add custom registry and auth token
 * ```
 * @voidzero-dev:registry=https://npm.pkg.github.com/
 * //npm.pkg.github.com/:_authToken=${GITHUB_TOKEN}
 * ```
 * @param projectPath - The path to the project
 */
function rewriteNpmrc(projectPath: string): void {
  const npmrcPath = path.join(projectPath, '.npmrc');
  if (!fs.existsSync(npmrcPath)) {
    fs.writeFileSync(npmrcPath, '');
  }

  let changed = false;
  let content = fs.readFileSync(npmrcPath, 'utf-8');
  const customRegistry = `@voidzero-dev:registry=https://npm.pkg.github.com/`;
  if (!content.includes(customRegistry)) {
    content = content ? `${content.trimEnd()}\n${customRegistry}` : customRegistry;
    changed = true;
  }
  // don't set if it already exists
  let customAuthToken = '//npm.pkg.github.com/:_authToken=';
  if (!content.includes(customAuthToken)) {
    customAuthToken += '${GITHUB_TOKEN}';
    content = content ? `${content.trimEnd()}\n${customAuthToken}` : customAuthToken;
    changed = true;
  }
  if (changed) {
    fs.writeFileSync(npmrcPath, content);
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
    prompts.log.success(`✅ Created vite.config.ts in ${displayRelative(viteConfigPath)}`);
  }
  return configs.viteConfig;
}

/**
 * Merge tsdown.config.* into vite.config.ts
 * - For JSON files: merge content directly into `lib` field and delete the JSON file
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
    mergeAndRemoveJsonConfig(projectPath, viteConfig, configs.tsdownConfig, 'lib');
    return;
  }

  // For TS/JS files, import the config file
  const tsdownRelativePath = `./${configs.tsdownConfig}`;
  const result = mergeTsdownConfig(fullViteConfigPath, tsdownRelativePath);
  if (result.updated) {
    fs.writeFileSync(fullViteConfigPath, result.content);
    prompts.log.success(
      `✅ Added import for ${displayRelative(fullTsdownConfigPath)} in ${displayRelative(fullViteConfigPath)}`,
    );
  }
  // Show documentation link for manual merging since we only added the import
  prompts.log.info(
    `📦 Please manually merge ${displayRelative(fullTsdownConfigPath)} into ${displayRelative(fullViteConfigPath)}, see https://viteplus.dev/migration/#tsdown`,
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
    // TODO: handle jsonc file
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
      `✅ Merged ${displayRelative(fullJsonConfigPath)} into ${displayRelative(fullViteConfigPath)}`,
    );
  } else {
    prompts.log.warn(
      `❌ Failed to merge ${displayRelative(fullJsonConfigPath)} into ${displayRelative(fullViteConfigPath)}`,
    );
    prompts.log.info(
      `Please complete the merge manually and follow the instructions in the documentation: https://viteplus.dev/config/`,
    );
  }
}

/**
 * Rewrite imports in all TypeScript/JavaScript files under a directory
 * This rewrites vite/vitest imports to @voidzero-dev/vite-plus
 * @param projectPath - The root directory to search for files
 */
function rewriteAllImports(projectPath: string): void {
  const result = rewriteImportsInDirectory(projectPath);

  if (result.modifiedFiles.length > 0) {
    prompts.log.success(`✅ Rewrote imports in ${result.modifiedFiles.length} file(s)`);
    prompts.log.info(result.modifiedFiles.map((file) => `  ${displayRelative(file)}`).join('\n'));
  }

  if (result.errors.length > 0) {
    prompts.log.warn(`⚠️ ${result.errors.length} file(s) had errors:`);
    for (const error of result.errors) {
      prompts.log.error(`  ${displayRelative(error.path)}: ${error.message}`);
    }
  }
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
