import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@clack/prompts';
import {
  mergeJsonConfig,
  rewriteScripts,
  rewriteImport,
  type DownloadPackageManagerResult,
} from '@voidzero-dev/vite-plus/binding';
import semver from 'semver';
import { Scalar, YAMLMap, YAMLSeq } from 'yaml';

import { PackageManager, type WorkspaceInfo } from '../types/index.ts';
import {
  scalarString,
  editJsonFile,
  editYamlFile,
  rulesDir,
  type YamlDocument,
} from '../utils/index.ts';
import { detectConfigs, detectPackageMetadata } from './detector.ts';

const VITE_PLUS_NAME = '@voidzero-dev/vite-plus';
const VITE_PLUS_VERSION = 'latest';
const OVERRIDE_PACKAGES = {
  vite: 'npm:@voidzero-dev/vite-plus-core@latest',
  vitest: 'npm:@voidzero-dev/vite-plus-test@latest',
} as const;
const REMOVE_PACKAGES = ['oxlint', 'oxlint-tsgolint', 'oxfmt'];

/**
 * Check the vite version is supported by migration
 * @param projectPath - The path to the project
 * @returns true if the vite version is supported by migration
 */
export function checkViteVersion(projectPath: string): boolean {
  return checkPackageVersion(projectPath, 'vite', '7.0.0');
}

export function checkVitestVersion(projectPath: string): boolean {
  return checkPackageVersion(projectPath, 'vitest', '4.0.0');
}

/**
 * Check the package version is supported by migration
 * @param projectPath - The path to the project
 * @param name - The name of the package
 * @param minVersion - The minimum version of the package
 * @returns true if the package version is supported by migration
 */
function checkPackageVersion(projectPath: string, name: string, minVersion: string): boolean {
  const metadata = detectPackageMetadata(projectPath, name);
  if (!metadata || metadata.name !== name) {
    return true;
  }
  if (semver.satisfies(metadata.version, `<${minVersion}`)) {
    prompts.log.error(`❌ ${name} version ${metadata.version} is not supported by migration`);
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
        ...OVERRIDE_PACKAGES,
      };
    } else if (packageManager === PackageManager.npm) {
      pkg.overrides = {
        ...pkg.overrides,
        ...OVERRIDE_PACKAGES,
      };
    } else if (packageManager === PackageManager.pnpm) {
      pkg.pnpm = {
        ...pkg.pnpm,
        overrides: {
          ...pkg.pnpm?.overrides,
          ...OVERRIDE_PACKAGES,
        },
      };
    }

    for (const [key, version] of Object.entries(OVERRIDE_PACKAGES)) {
      if (pkg.devDependencies?.[key]) {
        pkg.devDependencies[key] = version;
      }
      if (pkg.dependencies?.[key]) {
        pkg.dependencies[key] = version;
      }
    }

    // add vite-plus to devDependencies
    pkg.devDependencies = {
      ...pkg.devDependencies,
      [VITE_PLUS_NAME]: VITE_PLUS_VERSION,
    };

    rewritePackageJson(pkg);
    return pkg;
  });

  // set .npmrc to use vite-plus
  rewriteNpmrc(projectPath);
  rewriteLintStagedConfigFile(projectPath);
  rewriteViteConfigFile(projectPath);
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
  rewriteViteConfigFile(workspaceInfo.rootDir);
  // set package manager
  setPackageManager(workspaceInfo.rootDir, workspaceInfo.downloadPackageManager);
}

/**
 * Rewrite monorepo project to add vite-plus dependencies
 * @param projectPath - The path to the project
 */
export function rewriteMonorepoProject(projectPath: string, packageManager: PackageManager): void {
  rewriteViteConfigFile(projectPath);

  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }

  editJsonFile<{
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
    scripts?: Record<string, string>;
  }>(packageJsonPath, (pkg) => {
    const isNpm = packageManager === PackageManager.npm;
    let needVitePlus = false;
    for (const [key, value] of Object.entries(OVERRIDE_PACKAGES)) {
      const version = isNpm ? value : 'catalog:';
      if (pkg.devDependencies?.[key]) {
        pkg.devDependencies[key] = version;
        needVitePlus = true;
      }
      if (pkg.dependencies?.[key]) {
        pkg.dependencies[key] = version;
        needVitePlus = true;
      }
    }
    if (needVitePlus) {
      // add vite-plus to devDependencies to let vite config `import` rewrite work
      pkg.devDependencies = {
        ...pkg.devDependencies,
        [VITE_PLUS_NAME]: isNpm ? VITE_PLUS_VERSION : 'catalog:',
      };
    }

    // rewrite scripts in package.json
    rewritePackageJson(pkg);
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
    for (const key of Object.keys(OVERRIDE_PACKAGES)) {
      doc.setIn(['overrides', key], scalarString('catalog:'));
    }

    // peerDependencyRules.allowAny
    let allowAny = doc.getIn(['peerDependencyRules', 'allowAny']) as YAMLSeq<Scalar<string>>;
    if (!allowAny) {
      allowAny = new YAMLSeq<Scalar<string>>();
    }
    const existing = new Set(allowAny.items.map((n) => n.value));
    for (const key of Object.keys(OVERRIDE_PACKAGES)) {
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
    for (const key of Object.keys(OVERRIDE_PACKAGES)) {
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
  for (const [key, value] of Object.entries(OVERRIDE_PACKAGES)) {
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
  }>(packageJsonPath, (pkg) => {
    if (packageManager === PackageManager.yarn) {
      pkg.resolutions = {
        ...pkg.resolutions,
        // FIXME: yarn don't support catalog on resolutions
        // https://github.com/yarnpkg/berry/issues/6979
        ...OVERRIDE_PACKAGES,
      };
    } else if (packageManager === PackageManager.npm) {
      pkg.overrides = {
        ...pkg.overrides,
        ...OVERRIDE_PACKAGES,
      };
    }
    // pnpm use overrides field at pnpm-workspace.yaml

    // add vite-plus to devDependencies
    pkg.devDependencies = {
      ...pkg.devDependencies,
      [VITE_PLUS_NAME]: packageManager === PackageManager.npm ? VITE_PLUS_VERSION : 'catalog:',
    };
    return pkg;
  });

  // rewrite package.json
  rewriteMonorepoProject(projectPath, packageManager);
}

const RULES_YAML_PATH = path.join(rulesDir, 'vite-tools.yml');

export function rewritePackageJson(pkg: {
  scripts?: Record<string, string>;
  'lint-staged'?: Record<string, string | string[]>;
  devDependencies?: Record<string, string>;
  dependencies?: Record<string, string>;
}): void {
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
  // remove packages that are replaced with vite-plus
  for (const name of REMOVE_PACKAGES) {
    if (pkg.devDependencies?.[name]) {
      delete pkg.devDependencies[name];
    }
    if (pkg.dependencies?.[name]) {
      delete pkg.dependencies[name];
    }
  }
}

// https://github.com/lint-staged/lint-staged?tab=readme-ov-file#configuration
// only support json format
function rewriteLintStagedConfigFile(projectPath: string): void {
  const names = ['.lintstagedrc.json', '.lintstagedrc'];
  for (const name of names) {
    const lintStagedConfigJsonPath = path.join(projectPath, name);
    if (fs.existsSync(lintStagedConfigJsonPath)) {
      editJsonFile<Record<string, string | string[]>>(lintStagedConfigJsonPath, (config) => {
        const updated = rewriteScripts(
          JSON.stringify(config),
          fs.readFileSync(RULES_YAML_PATH, 'utf8'),
        );
        if (updated) {
          return JSON.parse(updated);
        }
      });
    }
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
 * Rewrite vite.config.ts to use vite-plus
 * - rewrite `import from 'vite'` to `import from 'vite-plus'`
 * - rewrite `import from 'vitest/config'` to `import from 'vite-plus'`
 * - merge oxlint config into vite.config.ts
 * - merge oxfmt config into vite.config.ts
 */
function rewriteViteConfigFile(projectPath: string): void {
  const configs = detectConfigs(projectPath);
  if (configs.viteConfig) {
    rewriteViteConfigImport(projectPath, configs.viteConfig);
  }
  if (configs.vitestConfig) {
    rewriteViteConfigImport(projectPath, configs.vitestConfig);
  }

  if (!configs.oxfmtConfig && !configs.oxlintConfig) {
    return;
  }
  if (!configs.viteConfig) {
    // TODO: handle typescript or javascript
    // create vite.config.ts
    configs.viteConfig = 'vite.config.ts';
    const viteConfigPath = path.join(projectPath, 'vite.config.ts');
    fs.writeFileSync(
      viteConfigPath,
      `import { defineConfig } from '${VITE_PLUS_NAME}';

export default defineConfig({});
`,
    );
    prompts.log.success(`✅ Created vite.config.ts in ${configs.viteConfig}`);
  }
  if (configs.oxlintConfig) {
    // merge oxlint config into vite.config.ts
    mergeAndRemoveJsonConfig(projectPath, configs.viteConfig, configs.oxlintConfig, 'lint');
  }
  if (configs.oxfmtConfig) {
    // merge oxfmt config into vite.config.ts
    mergeAndRemoveJsonConfig(projectPath, configs.viteConfig, configs.oxfmtConfig, 'fmt');
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
    prompts.log.success(`✅ Merged ${jsonConfigPath} into ${viteConfigPath}`);
  } else {
    prompts.log.warn(`❌ Failed to merge ${jsonConfigPath} into ${viteConfigPath}`);
    prompts.log.info(
      `Please complete the merge manually and follow the instructions in the documentation: https://viteplus.dev/config/`,
    );
  }
}

function rewriteViteConfigImport(projectPath: string, viteConfigPath: string): void {
  const fullPath = path.join(projectPath, viteConfigPath);
  const result = rewriteImport(fullPath);
  if (result.updated) {
    fs.writeFileSync(fullPath, result.content);
    prompts.log.success(`✅ Rewrote import in ${viteConfigPath}`);
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
