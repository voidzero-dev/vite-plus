import fs from 'node:fs';
import path from 'node:path';

import { rewriteScripts } from '../../../binding/index.js';
import { type WorkspacePackage } from '../../types/index.ts';
import { editJsonFile, readJsonFile } from '../../utils/json.ts';
import { rulesDir } from '../../utils/path.ts';
import { hasTsconfigTypesToRewrite, rewriteAllImports, rewriteTsconfigTypes } from '../migrator.ts';
import { type MigrationReport } from '../report.ts';

const RULES_YAML_PATH = path.join(rulesDir, 'vite-tools.yml');

const PREPARE_RULES_YAML_PATH = path.join(rulesDir, 'vite-prepare.yml');

// Cache YAML content to avoid repeated disk reads (called once per package in monorepos)
let cachedRulesYaml: string | undefined;
let cachedRulesYamlNoLintStaged: string | undefined;
let cachedPrepareRulesYaml: string | undefined;

export function readRulesYaml(): string {
  cachedRulesYaml ??= fs.readFileSync(RULES_YAML_PATH, 'utf8');
  return cachedRulesYaml;
}

export function getScriptRulesYaml(skipStagedMigration?: boolean): string {
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

export function readPrepareRulesYaml(): string {
  cachedPrepareRulesYaml ??= fs.readFileSync(PREPARE_RULES_YAML_PATH, 'utf8');
  return cachedPrepareRulesYaml;
}

type CoreMigrationWorkspace = {
  rootDir: string;
  packages?: WorkspacePackage[];
};

export type PendingCoreMigration = {
  scripts: boolean;
  tsconfigTypes: boolean;
};

export type CoreMigrationFinalizationResult = {
  scripts: boolean;
  tsconfigTypes: boolean;
  imports: boolean;
};

function getCoreMigrationProjectPaths(workspaceInfo: CoreMigrationWorkspace): string[] {
  return [
    workspaceInfo.rootDir,
    ...(workspaceInfo.packages ?? []).map((pkg) => path.join(workspaceInfo.rootDir, pkg.path)),
  ];
}

function hasCorePackageScriptRewrites(projectPath: string): boolean {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return false;
  }
  const pkg = readJsonFile(packageJsonPath) as { scripts?: Record<string, string> };
  if (!pkg.scripts) {
    return false;
  }
  return !!rewriteScripts(JSON.stringify(pkg.scripts), getScriptRulesYaml(true));
}

function rewriteCorePackageScripts(projectPath: string): boolean {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return false;
  }

  let changed = false;
  editJsonFile<{ scripts?: Record<string, string> }>(packageJsonPath, (pkg) => {
    if (!pkg.scripts) {
      return undefined;
    }
    const updated = rewriteScripts(JSON.stringify(pkg.scripts), getScriptRulesYaml(true));
    if (!updated) {
      return undefined;
    }
    pkg.scripts = JSON.parse(updated);
    changed = true;
    return pkg;
  });
  return changed;
}

export function detectPendingCoreMigration(
  workspaceInfo: CoreMigrationWorkspace,
): PendingCoreMigration {
  const projectPaths = getCoreMigrationProjectPaths(workspaceInfo);
  return {
    scripts: projectPaths.some((projectPath) => hasCorePackageScriptRewrites(projectPath)),
    tsconfigTypes: projectPaths.some((projectPath) => hasTsconfigTypesToRewrite(projectPath)),
  };
}

export function finalizeCoreMigrationForExistingVitePlus(
  workspaceInfo: CoreMigrationWorkspace,
  silent = false,
  report?: MigrationReport,
  pending = detectPendingCoreMigration(workspaceInfo),
): CoreMigrationFinalizationResult {
  const projectPaths = getCoreMigrationProjectPaths(workspaceInfo);
  const result: CoreMigrationFinalizationResult = {
    scripts: false,
    tsconfigTypes: false,
    imports: false,
  };

  if (pending.scripts) {
    for (const projectPath of projectPaths) {
      result.scripts = rewriteCorePackageScripts(projectPath) || result.scripts;
    }
  }

  if (pending.tsconfigTypes) {
    for (const projectPath of projectPaths) {
      result.tsconfigTypes =
        rewriteTsconfigTypes(projectPath, silent, report) || result.tsconfigTypes;
    }
  }

  result.imports = rewriteAllImports(workspaceInfo.rootDir, silent, report, true);

  return result;
}
