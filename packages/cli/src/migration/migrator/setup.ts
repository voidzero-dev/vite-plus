import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import semver from 'semver';

import { type DownloadPackageManagerResult } from '../../../binding/index.js';
import { editJsonFile } from '../../utils/json.ts';
import { detectConfigs } from '../detector.ts';
import { type MigrationReport } from '../report.ts';
import { warnMigration } from './shared.ts';

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

export type NodeVersionManagerDetection =
  | { file: '.nvmrc'; voltaPresent?: true }
  | { file: 'package.json'; voltaNodeVersion: string };

/**
 * Detect a .nvmrc file in the project directory.
 * If not found, check for a Volta node version in package.json.
 * If either is found, return the relevant info for migration.
 * Returns undefined if not found or .node-version already exists.
 */
export function detectNodeVersionManagerFile(
  projectPath: string,
): NodeVersionManagerDetection | undefined {
  // already has .node-version — skip detection to avoid false positives and preserve existing file
  if (fs.existsSync(path.join(projectPath, '.node-version'))) {
    return undefined;
  }

  const configs = detectConfigs(projectPath);

  // .nvmrc takes priority over volta.node when both are present.
  // voltaPresent is carried through so the migration step can remind the user
  // to remove the now-redundant volta field from package.json.
  if (configs.nvmrcFile) {
    return configs.voltaNode ? { file: '.nvmrc', voltaPresent: true } : { file: '.nvmrc' };
  }

  if (configs.voltaNode) {
    return { file: 'package.json', voltaNodeVersion: configs.voltaNode };
  }

  return undefined;
}

/**
 * Parse a version alias from a .nvmrc file into a .node-version compatible string.
 * Accepts the first line of .nvmrc (pre-trimmed).
 * Returns null for unsupported aliases like "system", "default", "iojs".
 */
export function parseNvmrcVersion(alias: string): string | null {
  const version = alias.trim();

  if (!version) {
    return null;
  }

  // "node" and "stable" mean "latest stable release" which maps closely to lts/*.
  // Starting from Node 27, all releases will be LTS, so the gap is shrinking.
  // We map these to lts/* and log the conversion so users are aware.
  if (version === 'node' || version === 'stable') {
    return 'lts/*';
  }

  // "iojs", "system", and "default" have no meaningful equivalent and cannot be auto-migrated.
  if (version === 'iojs' || version === 'system' || version === 'default') {
    return null;
  }

  // LTS aliases (lts/*, lts/iron, etc.) pass through as-is
  if (version.startsWith('lts/')) {
    return version;
  }

  // Strip optional 'v' prefix, then validate as a semver version or range
  const normalized = version.startsWith('v') ? version.slice(1) : version;
  if (!normalized || !semver.validRange(normalized)) {
    return null;
  }
  return normalized;
}

/**
 * Migrate .nvmrc or Volta node version from package.json to .node-version.
 * - For .nvmrc: the source file is removed after migration.
 * - For package.json (Volta): the volta field is left as-is; removal is left to the user's discretion.
 * Returns true on success, false if migration was skipped or failed.
 */
export function migrateNodeVersionManagerFile(
  projectPath: string,
  detection: NodeVersionManagerDetection,
  report?: MigrationReport,
): boolean {
  const nodeVersionPath = path.join(projectPath, '.node-version');

  // Volta: node version was already extracted during detection — no package.json re-read needed
  if (detection.file === 'package.json') {
    const { voltaNodeVersion } = detection;

    // Normalize Volta's "lts" alias to the .node-version compatible form
    const resolvedVersion = voltaNodeVersion === 'lts' ? 'lts/*' : voltaNodeVersion;

    if (!semver.valid(resolvedVersion) && resolvedVersion !== 'lts/*') {
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

  // .nvmrc: parse version alias and write to .node-version
  const sourcePath = path.join(projectPath, '.nvmrc');
  const content = fs.readFileSync(sourcePath, 'utf8');
  const originalAlias = content.split('\n')[0]?.trim() ?? '';
  const version = parseNvmrcVersion(originalAlias);

  if (!version) {
    warnMigration(
      '.nvmrc contains an unsupported version alias. Create .node-version manually with your desired Node.js version.',
      report,
    );
    return false;
  }

  // TODO: remove this log once Node 27+ makes all releases LTS, at which point
  // "node"/"stable" and "lts/*" will be effectively equivalent.
  if (version === 'lts/*' && (originalAlias === 'node' || originalAlias === 'stable')) {
    prompts.log.info(
      `"${originalAlias}" in .nvmrc is not a specific version; automatically mapping to "lts/*"`,
    );
  }

  fs.writeFileSync(nodeVersionPath, `${version}\n`);
  fs.unlinkSync(sourcePath);

  if (report) {
    report.nodeVersionFileMigrated = true;
    // Both .nvmrc and volta were present; .nvmrc was migrated but volta still lingers.
    if (detection.voltaPresent) {
      report.manualSteps.push('Remove the "volta" field from package.json');
    }
  } else if (detection.voltaPresent) {
    prompts.log.info('You can now remove the "volta" field from package.json manually.');
  }
  return true;
}
