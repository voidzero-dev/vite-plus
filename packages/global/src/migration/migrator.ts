import fs from 'node:fs';
import path from 'node:path';

import { rewriteScripts } from '@voidzero-dev/vite-plus/binding';
import { Scalar, YAMLMap, YAMLSeq } from 'yaml';

import { PackageManager, type WorkspaceInfo } from '../types/index.ts';
import {
  scalarString,
  editJsonFile,
  editYamlFile,
  rulesDir,
  templatesDir,
} from '../utils/index.ts';

const VITE_PLUS_NAME = '@voidzero-dev/vite-plus';
const VITE_PLUS_VERSION = 'latest';
const OVERRIDE_PACKAGES = {
  vite: 'npm:@voidzero-dev/vite-plus-core@latest',
  vitest: 'npm:@voidzero-dev/vite-plus-test@latest',
} as const;
const REMOVE_PACKAGES = ['oxlint', 'oxlint-tsgolint', 'oxfmt', 'tsdown'];

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
}

/**
 * Rewrite monorepo project to add vite-plus dependencies
 * @param projectPath - The path to the project
 */
export function rewriteMonorepoProject(projectPath: string, packageManager: PackageManager): void {
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
    for (const [key, value] of Object.entries(OVERRIDE_PACKAGES)) {
      const version = isNpm ? value : 'catalog:';
      if (pkg.devDependencies?.[key]) {
        pkg.devDependencies[key] = version;
      }
      if (pkg.dependencies?.[key]) {
        pkg.dependencies[key] = version;
      }
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
    for (const [key, value] of Object.entries(OVERRIDE_PACKAGES)) {
      doc.setIn(['catalog', key], scalarString(value));
    }
    doc.setIn(['catalog', scalarString(VITE_PLUS_NAME)], VITE_PLUS_VERSION);
    for (const name of REMOVE_PACKAGES) {
      doc.deleteIn(['catalog', name]);
    }

    // TODO: rewrite `catalogs` when OVERRIDE_PACKAGES exists in catalog

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
      // add @voidzero-dev/*, vite, vitest to minimumReleaseAgeExclude
      let minimumReleaseAgeExclude = doc.getIn(['minimumReleaseAgeExclude']) as YAMLSeq<
        Scalar<string>
      >;
      if (!minimumReleaseAgeExclude) {
        minimumReleaseAgeExclude = new YAMLSeq();
      }
      const existing = new Set(minimumReleaseAgeExclude.items.map((n) => n.value));
      if (!existing.has('@voidzero-dev/*')) {
        minimumReleaseAgeExclude.add(scalarString('@voidzero-dev/*'));
      }
      for (const key of Object.keys(OVERRIDE_PACKAGES)) {
        if (!existing.has(key)) {
          minimumReleaseAgeExclude.add(scalarString(key));
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
    for (const [key, value] of Object.entries(OVERRIDE_PACKAGES)) {
      doc.setIn(['catalog', key], scalarString(value));
    }
  });
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
        vite: 'catalog:',
        vitest: 'catalog:',
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
function rewriteNpmrc(projectPath: string): void {
  const npmrcPath = path.join(projectPath, '.npmrc');
  if (!fs.existsSync(npmrcPath)) {
    fs.writeFileSync(npmrcPath, '');
  }

  const npmrc = fs.readFileSync(path.join(templatesDir, 'config/_npmrc'), 'utf-8');
  let content = fs.readFileSync(npmrcPath, 'utf-8');
  if (content.includes(npmrc)) {
    return;
  }
  content = content ? `${content.trimEnd()}\n${npmrc}` : npmrc;
  fs.writeFileSync(npmrcPath, content);
}
