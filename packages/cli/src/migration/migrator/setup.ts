import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import semver from 'semver';

import {
  type DownloadPackageManagerResult,
  resolveProjectNodeVersion,
  resolveSupportedNodeVersion,
} from '../../../binding/index.js';
import { SUPPORTED_NODE_RANGE } from '../../utils/constants.ts';
import { editJsonFile } from '../../utils/json.ts';
import { cancelAndExit } from '../../utils/prompts.ts';
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

interface NodePinnedPackageJson {
  devEngines?: { runtime?: unknown; [key: string]: unknown };
  engines?: { node?: unknown; [key: string]: unknown };
  [key: string]: unknown;
}

type NodeRuntimeEntry = { name?: unknown; version?: unknown; [key: string]: unknown };

/**
 * Locate the `node` entry inside a `devEngines.runtime` value, which may be a
 * single object or an array of runtime objects. Returns the live object so the
 * caller can mutate its `version` in place.
 */
function findNodeRuntimeEntry(runtime: unknown): NodeRuntimeEntry | undefined {
  const isNodeEntry = (entry: unknown): entry is NodeRuntimeEntry =>
    typeof entry === 'object' && entry !== null && (entry as NodeRuntimeEntry).name === 'node';

  if (Array.isArray(runtime)) {
    return runtime.find(isNodeEntry);
  }
  if (isNodeEntry(runtime)) {
    return runtime;
  }
  return undefined;
}

/**
 * Write an upgraded Node.js version back to the source it was resolved from
 * (returned by {@link resolveProjectNodeVersion}):
 * - `node-version-file` → overwrite the `.node-version` file at `sourcePath`.
 * - `dev-engines-runtime` → set the `node` runtime entry's `.version` in package.json.
 * - `engines-node` → set `engines.node` in package.json.
 *
 * package.json edits go through {@link editJsonFile} so the file's formatting is
 * preserved.
 */
function writeUpgradedNodeVersion(source: string, sourcePath: string, version: string): void {
  if (source === 'node-version-file') {
    fs.writeFileSync(sourcePath, `${version}\n`);
    return;
  }
  if (source === 'dev-engines-runtime') {
    editJsonFile<NodePinnedPackageJson>(sourcePath, (pkg) => {
      const entry = findNodeRuntimeEntry(pkg.devEngines?.runtime);
      if (entry) {
        entry.version = version;
      }
      return pkg;
    });
    return;
  }
  if (source === 'engines-node') {
    editJsonFile<NodePinnedPackageJson>(sourcePath, (pkg) => {
      if (pkg.engines) {
        pkg.engines.node = version;
      }
      return pkg;
    });
  }
}

/**
 * Bump the project's effective Node.js pin up to the concrete latest release of
 * the same major when it sits BELOW the Vite+ supported range (sourced from this
 * package's `engines.node`, e.g. `^20.19.0 || ^22.18.0 || >=24.11.0`). This
 * fixes "Cannot find native binding" failures caused by engine-strict
 * installers skipping the native optional dependency under an unsupported
 * Node.js version.
 *
 * The effective pin and its source are read with the shared Rust resolver
 * {@link resolveProjectNodeVersion}, which checks, in priority order:
 * `.node-version` → `devEngines.runtime[node]` → `engines.node`. Only that
 * single effective source is upgraded; shadowed lower-priority pins don't affect
 * the runtime. `.nvmrc`/Volta pins are converted to `.node-version` by
 * {@link migrateNodeVersionManagerFile}, which runs first, so they are covered
 * via the `.node-version` source here.
 *
 * Whether the pin is below range (and what to upgrade it to) is decided by the
 * {@link resolveSupportedNodeVersion} binding via range intersection, so true
 * ranges, caret unions, and aliases like `lts/*` are left untouched. The binding
 * calls are best-effort: any failure (e.g. offline) is treated as "nothing to
 * upgrade".
 *
 * In interactive mode the upgrade is confirmed first (default Yes); in
 * non-interactive mode it proceeds directly.
 *
 * @returns true if the pin was rewritten.
 */
export async function upgradeUnsupportedNodeVersions(
  projectPath: string,
  interactive: boolean,
  report?: MigrationReport,
  // Clears the migration progress spinner before the confirm prompt renders so
  // it does not keep animating underneath the prompt. The caller restarts the
  // spinner with its next progress update.
  pauseProgress?: () => void,
): Promise<boolean> {
  // 1. Read the effective pin + source via the shared Rust resolver.
  let resolution: Awaited<ReturnType<typeof resolveProjectNodeVersion>>;
  try {
    resolution = await resolveProjectNodeVersion(projectPath);
  } catch {
    return false;
  }
  if (!resolution) {
    return false;
  }
  const { version: from, source, sourcePath } = resolution;

  // 2. Plan: resolve the supported upgrade target. null = already supported, a
  // true range/alias, or an unsupported major — nothing to do.
  let to: string | null;
  try {
    to = (await resolveSupportedNodeVersion(from, SUPPORTED_NODE_RANGE)) ?? null;
  } catch {
    return false;
  }
  if (!to) {
    return false;
  }

  // 3. Confirm before writing (default Yes in interactive mode; proceed
  // directly when non-interactive).
  if (interactive) {
    pauseProgress?.();
    const confirmed = await prompts.confirm({
      message: `Upgrade Node.js ${from} to ${to}? ${from} is below the Vite+ supported range.`,
      initialValue: true,
    });
    if (prompts.isCancel(confirmed)) {
      cancelAndExit();
    }
    if (!confirmed) {
      return false;
    }
  }

  // 4. Write the upgrade back to its source.
  writeUpgradedNodeVersion(source, sourcePath, to);
  warnMigration(`Upgraded Node.js ${from} to ${to} (below the supported range)`, report);
  return true;
}
