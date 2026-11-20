import fs from 'node:fs';
import path from 'node:path';

import { rewriteScripts } from '../../../binding/index.js';
import { type WorkspacePackage } from '../../types/index.ts';
import { editJsonFile, readJsonFile } from '../../utils/json.ts';
import { rulesDir } from '../../utils/path.ts';
import {
  collectPackagesKeepingTsdown,
  dropDeadOxlintPluginsDependency,
  getWorkspaceProjectPaths,
  hasTsconfigTypesToRewrite,
  mergeTsdownConfigFile,
  migrateTaskCacheConfigInViteConfig,
  rewriteAllImports,
  rewriteTsconfigTypes,
} from '../migrator.ts';
import { type MigrationReport } from '../report.ts';

const RULES_YAML_PATH = path.join(rulesDir, 'vite-tools.yml');

const PREPARE_RULES_YAML_PATH = path.join(rulesDir, 'vite-prepare.yml');

// Cache YAML content to avoid repeated disk reads (called once per package in monorepos)
let cachedRulesYaml: string | undefined;
const filteredScriptRulesYaml = new Map<string, string>();
let cachedPrepareRulesYaml: string | undefined;

export function readRulesYaml(): string {
  cachedRulesYaml ??= fs.readFileSync(RULES_YAML_PATH, 'utf8');
  return cachedRulesYaml;
}

// Each rule is one YAML document introduced by a `---` separator line. Split on the
// separators instead of on blank lines, which a formatter is free to collapse. Comment
// lines directly above a separator describe the rule below it, so they move with it.
function splitRuleDocuments(yaml: string): string[] {
  const lines = yaml.split('\n');
  const documents: string[] = [];
  let start = 0;
  for (const [index, line] of lines.entries()) {
    if (line.trimEnd() !== '---') {
      continue;
    }
    let documentStart = index;
    while (documentStart > start && lines[documentStart - 1].trimStart().startsWith('#')) {
      documentStart--;
    }
    if (documentStart > start) {
      documents.push(lines.slice(start, documentStart).join('\n'));
      start = documentStart;
    }
  }
  documents.push(lines.slice(start).join('\n'));
  return documents;
}

export function getScriptRulesYaml(skipStagedMigration = false, keepTsdown = false): string {
  const yaml = readRulesYaml();
  if (!skipStagedMigration && !keepTsdown) {
    return yaml;
  }
  const key = `${skipStagedMigration}:${keepTsdown}`;
  let filtered = filteredScriptRulesYaml.get(key);
  if (filtered === undefined) {
    filtered = splitRuleDocuments(yaml)
      .filter(
        (document) =>
          !(skipStagedMigration && document.includes('id: replace-lint-staged')) &&
          !(keepTsdown && document.includes('id: replace-tsdown')),
      )
      .join('\n');
    filteredScriptRulesYaml.set(key, filtered);
  }
  return filtered;
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
  dependencies: boolean;
  scripts: boolean;
  tsconfigTypes: boolean;
  imports: boolean;
  tsdownConfig: boolean;
  taskCacheConfig: boolean;
  // Kept out of the report so that review items alone do not make an
  // up-to-date project run the rest of the migration.
  taskCacheWarnings: string[];
};

function hasCorePackageScriptRewrites(projectPath: string, keepTsdown: boolean): boolean {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return false;
  }
  const pkg = readJsonFile(packageJsonPath) as { scripts?: Record<string, string> };
  if (!pkg.scripts) {
    return false;
  }
  return !!rewriteScripts(JSON.stringify(pkg.scripts), getScriptRulesYaml(true, keepTsdown));
}

function rewriteCorePackageScripts(projectPath: string, keepTsdown: boolean): boolean {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return false;
  }

  let changed = false;
  editJsonFile<{ scripts?: Record<string, string> }>(packageJsonPath, (pkg) => {
    if (!pkg.scripts) {
      return undefined;
    }
    const updated = rewriteScripts(
      JSON.stringify(pkg.scripts),
      getScriptRulesYaml(true, keepTsdown),
    );
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
  tsdownPackages = collectPackagesKeepingTsdown(workspaceInfo.rootDir, workspaceInfo.packages),
): PendingCoreMigration {
  const projectPaths = getWorkspaceProjectPaths(workspaceInfo);
  return {
    scripts: projectPaths.some((projectPath) =>
      hasCorePackageScriptRewrites(projectPath, tsdownPackages.has(path.resolve(projectPath))),
    ),
    tsconfigTypes: projectPaths.some((projectPath) =>
      hasTsconfigTypesToRewrite(projectPath, tsdownPackages.has(path.resolve(projectPath))),
    ),
  };
}

export function finalizeCoreMigrationForExistingVitePlus(
  workspaceInfo: CoreMigrationWorkspace,
  silent = false,
  report?: MigrationReport,
  pending?: PendingCoreMigration,
  // null defers import rewriting until dynamic lint settings can be resolved.
  importRuleConfig?: string | null,
  tsdownPackages = collectPackagesKeepingTsdown(
    workspaceInfo.rootDir,
    workspaceInfo.packages,
    report,
  ),
): CoreMigrationFinalizationResult {
  pending ??= detectPendingCoreMigration(workspaceInfo, tsdownPackages);
  const projectPaths = getWorkspaceProjectPaths(workspaceInfo);
  const result: CoreMigrationFinalizationResult = {
    dependencies: false,
    scripts: false,
    tsconfigTypes: false,
    imports: false,
    tsdownConfig: false,
    taskCacheConfig: false,
    taskCacheWarnings: [],
  };

  if (pending.scripts) {
    for (const projectPath of projectPaths) {
      result.scripts =
        rewriteCorePackageScripts(projectPath, tsdownPackages.has(path.resolve(projectPath))) ||
        result.scripts;
    }
  }

  if (pending.tsconfigTypes) {
    for (const projectPath of projectPaths) {
      result.tsconfigTypes =
        rewriteTsconfigTypes(
          projectPath,
          silent,
          report,
          tsdownPackages.has(path.resolve(projectPath)),
        ) || result.tsconfigTypes;
    }
  }

  if (importRuleConfig !== null) {
    result.imports = rewriteAllImports(workspaceInfo.rootDir, silent, report, {
      importRuleConfig,
      preserveTsdownDirs: [...tsdownPackages],
    });
  }
  result.dependencies = dropDeadOxlintPluginsDependency(
    workspaceInfo.rootDir,
    workspaceInfo.packages,
  );

  // Partial migrations can already have a Vite+ dependency while leaving
  // tsdown.config.* undiscoverable by vp pack. Finalize those configs on the
  // existing-Vite+ path just as the fresh migration path does.
  for (const projectPath of projectPaths) {
    result.tsdownConfig =
      mergeTsdownConfigFile(
        projectPath,
        silent,
        report,
        tsdownPackages.has(path.resolve(projectPath)),
      ) || result.tsdownConfig;
  }

  for (const projectPath of projectPaths) {
    result.taskCacheConfig =
      migrateTaskCacheConfigInViteConfig(projectPath, silent, report, result.taskCacheWarnings) ||
      result.taskCacheConfig;
  }

  return result;
}
