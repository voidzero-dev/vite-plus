import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import { globSync } from 'tinyglobby';
import { normalize } from 'verkit';
import { parse as parseYaml } from 'yaml';

import { type DownloadPackageManagerResult } from '../../../binding/index.js';
import { SETUP_VP_VERSION } from '../../utils/constants.ts';
import { editJsonFile } from '../../utils/json.ts';
import { detectConfigs } from '../detector.ts';
import { type MigrationReport } from '../report.ts';
import { isPlainRecord, warnMigration } from './shared.ts';

export function setPackageManager(
  projectDir: string,
  downloadPackageManager: DownloadPackageManagerResult,
) {
  // Set the package manager pin. Compatibility-first rule (rfcs/dev-engines.md):
  // an existing `packageManager` field or `devEngines.packageManager` declaration
  // is the source of truth and is left as-is; otherwise the exact resolved version
  // is written to `devEngines.packageManager` (the recommended standard field).
  editJsonFile<{
    packageManager?: string;
    devEngines?: { packageManager?: unknown; [key: string]: unknown };
  }>(path.join(projectDir, 'package.json'), (pkg) => {
    if (!pkg.packageManager && !pkg.devEngines?.packageManager) {
      // Only spread a well-formed object: spreading a malformed devEngines value
      // (string/array) would corrupt the field with numeric index keys
      const devEngines =
        typeof pkg.devEngines === 'object' &&
        pkg.devEngines !== null &&
        !Array.isArray(pkg.devEngines)
          ? pkg.devEngines
          : undefined;
      pkg.devEngines = {
        ...devEngines,
        packageManager: {
          name: downloadPackageManager.name,
          version: downloadPackageManager.version,
          onFail: 'download',
        },
      };
    }
    return pkg;
  });
}

export type NodeVersionManagerDetection = { file: 'package.json'; voltaNodeVersion: string };

/**
 * Detect a Volta node version in package.json for migration.
 * Returns undefined if not found or the project already has a pin file Vite+ reads.
 */
export function detectNodeVersionManagerFile(
  projectPath: string,
): NodeVersionManagerDetection | undefined {
  // already has .node-version — skip detection to avoid false positives and preserve existing file
  if (fs.existsSync(path.join(projectPath, '.node-version'))) {
    return undefined;
  }

  const configs = detectConfigs(projectPath);

  // Vite+ reads .nvmrc directly, and a new .node-version would take priority over it.
  if (configs.nvmrcFile) {
    return undefined;
  }

  if (configs.voltaNode) {
    return { file: 'package.json', voltaNodeVersion: configs.voltaNode };
  }

  return undefined;
}

function isCompositeActionFile(filePath: string): boolean {
  try {
    const action = parseYaml(fs.readFileSync(filePath, 'utf8')) as unknown;
    return isPlainRecord(action) && isPlainRecord(action.runs) && action.runs.using === 'composite';
  } catch {
    return false;
  }
}

/**
 * Collect top-level workflows and composite action definitions under `.github`.
 * Action metadata must declare `runs.using: composite`. Paths are absolute, and
 * filename matching is case-insensitive.
 */
function collectGithubActionFiles(projectPath: string): string[] {
  const globOptions = {
    cwd: path.join(projectPath, '.github'),
    absolute: true,
    caseSensitiveMatch: false,
    // Otherwise a directory named like `ci.yml` also matches as `ci.yml/**`.
    expandDirectories: false,
  } as const;
  const workflows = globSync('workflows/*.{yml,yaml}', globOptions);
  const compositeActions = globSync('**/action.{yml,yaml}', {
    ...globOptions,
    dot: true,
  }).filter(isCompositeActionFile);

  return [...new Set([...workflows, ...compositeActions])];
}

/**
 * Match an exact `voidzero-dev/setup-vp@v1` action reference while preserving
 * indentation, list syntax, quote style, and trailing comments. The end guard
 * prevents prefix matches such as `@v1.2.3` or `@v10`.
 */
const SETUP_VP_V1_RE =
  /^([ \t]*(?:-[ \t]*)?uses[ \t]*:[ \t]*)(['"]?)voidzero-dev\/setup-vp@v1\2(?=[ \t]*(?:#.*)?\r?$)/gm;

/**
 * Replace the frozen `voidzero-dev/setup-vp@v1` tag with the latest exact
 * release known to this Vite+ version. GitHub workflow files and composite
 * action definitions are both covered. Existing exact versions, commit SHAs,
 * and commented-out steps are left unchanged.
 *
 * Returns the project-relative paths of the updated files.
 */
export function migrateSetupVpVersion(projectPath: string, report?: MigrationReport): string[] {
  const updatedFiles: string[] = [];

  for (const filePath of collectGithubActionFiles(projectPath)) {
    const relativePath = path.relative(projectPath, filePath);
    try {
      const original = fs.readFileSync(filePath, 'utf8');
      const rewritten = original.replace(
        SETUP_VP_V1_RE,
        `$1$2voidzero-dev/setup-vp@${SETUP_VP_VERSION}$2`,
      );
      if (rewritten !== original) {
        fs.writeFileSync(filePath, rewritten);
        updatedFiles.push(relativePath);
      }
    } catch (error) {
      warnMigration(
        `Could not update the setup-vp version in ${relativePath}: ${error instanceof Error ? error.message : String(error)}`,
        report,
      );
    }
  }

  if (report) {
    report.setupVpVersionUpdatedFileCount += updatedFiles.length;
  }
  return updatedFiles;
}

/**
 * Migrate the Volta node version from package.json to .node-version.
 * The volta field is left as-is; removal is left to the user's discretion.
 * Returns true on success, false if migration was skipped or failed.
 */
export function migrateNodeVersionManagerFile(
  projectPath: string,
  detection: NodeVersionManagerDetection,
  report?: MigrationReport,
): boolean {
  const nodeVersionPath = path.join(projectPath, '.node-version');

  // Node version was already extracted during detection — no package.json re-read needed
  const { voltaNodeVersion } = detection;

  // Normalize Volta's "lts" alias to the .node-version compatible form
  const resolvedVersion = voltaNodeVersion === 'lts' ? 'lts/*' : voltaNodeVersion;

  if (!normalize(resolvedVersion) && resolvedVersion !== 'lts/*') {
    warnMigration(
      `package.json volta.node "${voltaNodeVersion}" is not an exact version. Pin an exact version (e.g. ${voltaNodeVersion}.0 or run \`volta pin node@${voltaNodeVersion}\`) then re-run migration.`,
      report,
    );
    return false;
  }

  fs.writeFileSync(nodeVersionPath, `${resolvedVersion}\n`);
  if (report) {
    report.manualSteps.push('Remove the "volta" field from package.json');
    report.nodeVersionFileMigrated = true;
  } else {
    prompts.log.info('You can now remove the "volta" field from package.json manually.');
  }
  return true;
}
