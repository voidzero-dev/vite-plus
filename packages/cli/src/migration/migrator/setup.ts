import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import semver from 'semver';

import {
  type DownloadPackageManagerResult,
  resolveSupportedNodeRange,
  resolveSupportedNodeVersion,
} from '../../../binding/index.js';
import { SUPPORTED_NODE_RANGE } from '../../utils/constants.ts';
import { editJsonFile, readJsonFile } from '../../utils/json.ts';
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
 * Match an `actions/setup-node` `node-version-file:` value that points at the
 * now-removed `.nvmrc`, capturing the surrounding style so it can be preserved:
 *   1. the key + whitespace (`node-version-file:` ...)
 *   2. the optional opening quote (`'`, `"`, or none), reused as the closing quote
 *   3. the optional `./` prefix
 * The closing quote backreference (`\2`) plus the `(?=\s|$)` boundary keep this
 * pinned to the exact value `.nvmrc` / `./.nvmrc` (quoted or bare) and prevent
 * matching similar values such as `.nvmrc-backup`. Only `node-version-file:` lines
 * are touched, so shell `cat .nvmrc` and comments are left alone.
 */
const NODE_VERSION_FILE_NVMRC_RE = /(node-version-file:[ \t]*)(['"]?)(\.\/)?\.nvmrc\2(?=\s|$)/gm;

/**
 * After `.nvmrc` is converted to `.node-version`, rewrite any GitHub Actions
 * workflow that still references the removed file via `node-version-file:`,
 * otherwise `actions/setup-node` fails in CI with "The specified node version
 * file at: .../.nvmrc does not exist".
 *
 * Best-effort and narrowly scoped: scans `.github/workflows/*.{yml,yaml}` under
 * the workspace root, only rewrites `node-version-file:` values (preserving the
 * original quoting/indentation), and never fails the migration if the directory
 * is absent or a file cannot be read/written. Returns the relative paths of the
 * workflow files that were updated.
 */
function rewriteWorkflowNodeVersionFileReferences(projectPath: string): string[] {
  const workflowsDir = path.join(projectPath, '.github', 'workflows');

  let entries: string[];
  try {
    entries = fs.readdirSync(workflowsDir);
  } catch {
    // No `.github/workflows` directory (or unreadable): nothing to do.
    return [];
  }

  const updated: string[] = [];
  for (const entry of entries) {
    if (!/\.ya?ml$/i.test(entry)) {
      continue;
    }
    const filePath = path.join(workflowsDir, entry);
    try {
      const original = fs.readFileSync(filePath, 'utf8');
      // `$1` key+space, `$2` opening quote (reused as the closing quote),
      // `$3` optional `./` prefix; only `.nvmrc` becomes `.node-version`.
      const rewritten = original.replace(NODE_VERSION_FILE_NVMRC_RE, '$1$2$3.node-version$2');
      if (rewritten !== original) {
        fs.writeFileSync(filePath, rewritten);
        updated.push(path.join('.github', 'workflows', entry));
      }
    } catch {
      // Best-effort: skip files that cannot be read or written.
    }
  }

  return updated;
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

  // The .nvmrc is gone, so repoint any GitHub Actions workflow that fed it to
  // `actions/setup-node` via `node-version-file:` at the new `.node-version`,
  // so CI does not break with "node version file ... does not exist".
  const updatedWorkflows = rewriteWorkflowNodeVersionFileReferences(projectPath);
  if (updatedWorkflows.length > 0) {
    warnMigration(
      `Updated node-version-file from .nvmrc to .node-version in GitHub workflow(s): ${updatedWorkflows.join(', ')}`,
      report,
    );
  }

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

/** A planned Node.js pin rewrite, gathered per independent source. */
interface NodeVersionUpgrade {
  source: 'node-version-file' | 'dev-engines-runtime' | 'engines-node';
  /** The current pin, exactly as written in the source. */
  from: string;
  /** The value to write: a concrete version for `.node-version`, an open-ended
   * `>=<supported-minimum>` range for the constraint fields. */
  to: string;
}

/** Human-readable label per upgrade source, used in the confirm summary. */
const NODE_VERSION_SOURCE_LABELS: Record<NodeVersionUpgrade['source'], string> = {
  'node-version-file': '.node-version',
  'dev-engines-runtime': 'devEngines.runtime',
  'engines-node': 'engines.node',
};

/**
 * Best-effort wrapper around the synchronous, network-free
 * {@link resolveSupportedNodeRange} binding. Returns the open-ended
 * `>=<supported-minimum>` range for a below-floor constraint pin, or `null` when
 * the pin is already supported, unparseable, or in an unsupported major.
 */
function resolveSupportedNodeFloorRange(from: string): string | null {
  try {
    return resolveSupportedNodeRange(from, SUPPORTED_NODE_RANGE) ?? null;
  } catch {
    return null;
  }
}

/**
 * Gather every Node.js pin that sits BELOW the Vite+ supported range (sourced
 * from this package's `engines.node`, e.g. `^20.19.0 || ^22.18.0 || >=24.11.0`),
 * checking all three sources INDEPENDENTLY rather than only the single effective
 * pin. pnpm decides whether to install the native optional dependency by testing
 * its `engines.node` against the FLOOR of the project's declared Node range
 * (chiefly `devEngines.runtime[node].version`), so a too-low floor in ANY source
 * can make pnpm skip the native package and trigger "Cannot find native
 * binding".
 *
 * - `.node-version` (single-version file) → the concrete latest release of the
 *   floor's major via {@link resolveSupportedNodeVersion} (hits the release
 *   index).
 * - `devEngines.runtime[node].version` and `engines.node` (constraint fields) →
 *   an open-ended `>=<supported-minimum>` range via
 *   {@link resolveSupportedNodeFloorRange}; an exact pin in a constraint field
 *   would wrongly reject newer supported releases.
 *
 * Whether a pin is below floor (and what to lift it to) is decided entirely by
 * the binding's FLOOR-based range math, so open ranges already at floor
 * (`>=24.11.0`), caret unions (`^22.18.0`), aliases (`lts/*`), and unsupported
 * majors (21/23) are left untouched. `.nvmrc`/Volta pins are converted to
 * `.node-version` by {@link migrateNodeVersionManagerFile}, which runs first, so
 * they are covered via the `.node-version` source here. Every binding call is
 * best-effort: a failure (e.g. an offline release-index lookup) is treated as
 * "nothing to upgrade" for that source.
 */
async function planNodeVersionUpgrades(projectPath: string): Promise<NodeVersionUpgrade[]> {
  const plans: NodeVersionUpgrade[] = [];

  const nodeVersionPath = path.join(projectPath, '.node-version');
  if (fs.existsSync(nodeVersionPath)) {
    const from = fs.readFileSync(nodeVersionPath, 'utf8').split('\n')[0]?.trim() ?? '';
    if (from) {
      try {
        const to = await resolveSupportedNodeVersion(from, SUPPORTED_NODE_RANGE);
        if (to) {
          plans.push({ source: 'node-version-file', from, to });
        }
      } catch {
        // best-effort: leave this source unchanged
      }
    }
  }

  let pkg: NodePinnedPackageJson | undefined;
  try {
    pkg = readJsonFile(path.join(projectPath, 'package.json')) as NodePinnedPackageJson;
  } catch {
    pkg = undefined;
  }
  if (pkg) {
    const runtimeEntry = findNodeRuntimeEntry(pkg.devEngines?.runtime);
    const runtimeVersion =
      typeof runtimeEntry?.version === 'string' ? runtimeEntry.version : undefined;
    if (runtimeVersion) {
      const to = resolveSupportedNodeFloorRange(runtimeVersion);
      if (to) {
        plans.push({ source: 'dev-engines-runtime', from: runtimeVersion, to });
      }
    }

    const enginesNode = typeof pkg.engines?.node === 'string' ? pkg.engines.node : undefined;
    if (enginesNode) {
      const to = resolveSupportedNodeFloorRange(enginesNode);
      if (to) {
        plans.push({ source: 'engines-node', from: enginesNode, to });
      }
    }
  }

  return plans;
}

/**
 * Whether any Node.js pin (`.node-version`, `devEngines.runtime[node]`, or
 * `engines.node`) sits BELOW the Vite+ supported range and would be lifted by
 * {@link upgradeUnsupportedNodeVersions}.
 *
 * Detection only: this reuses the exact same {@link planNodeVersionUpgrades}
 * planner the upgrade itself uses (so the floor logic is never duplicated) and
 * writes nothing to disk. It exists so the migrate flow's "already using Vite+"
 * early guard does not fire when the project's only pending work is a below-floor
 * Node pin. Otherwise the upgrade step would be skipped and pnpm could keep
 * skipping the native optional binding.
 */
export async function hasUnsupportedNodeVersionPin(projectPath: string): Promise<boolean> {
  return (await planNodeVersionUpgrades(projectPath)).length > 0;
}

/**
 * Apply gathered Node.js upgrades, preserving each file's formatting. The
 * `.node-version` file is overwritten with the concrete version; the package.json
 * constraint fields are rewritten in a single {@link editJsonFile} pass.
 */
function applyNodeVersionUpgrades(projectPath: string, plans: NodeVersionUpgrade[]): void {
  const nodeVersionPlan = plans.find((plan) => plan.source === 'node-version-file');
  if (nodeVersionPlan) {
    fs.writeFileSync(path.join(projectPath, '.node-version'), `${nodeVersionPlan.to}\n`);
  }

  const runtimePlan = plans.find((plan) => plan.source === 'dev-engines-runtime');
  const enginesPlan = plans.find((plan) => plan.source === 'engines-node');
  if (runtimePlan || enginesPlan) {
    editJsonFile<NodePinnedPackageJson>(path.join(projectPath, 'package.json'), (pkg) => {
      if (runtimePlan) {
        const entry = findNodeRuntimeEntry(pkg.devEngines?.runtime);
        if (entry) {
          entry.version = runtimePlan.to;
        }
      }
      if (enginesPlan && pkg.engines) {
        pkg.engines.node = enginesPlan.to;
      }
      return pkg;
    });
  }
}

/**
 * Lift every Node.js pin that sits BELOW the Vite+ supported range up to a
 * supported value, fixing "Cannot find native binding" failures caused by
 * engine-strict installers (pnpm) skipping the native optional dependency when
 * the project's declared Node FLOOR is too low.
 *
 * All three pin sources are normalized INDEPENDENTLY (see
 * {@link planNodeVersionUpgrades}): `.node-version` gets the concrete latest
 * release of its major, while `engines.node` and `devEngines.runtime[node]` get
 * an open-ended `>=<supported-minimum>` range so they keep accepting newer
 * supported releases. The below-floor decision is delegated to the native
 * binding's FLOOR-based range math.
 *
 * In interactive mode a single confirm (default Yes) covers every planned
 * change; in non-interactive mode it proceeds directly.
 *
 * @returns true if any pin was rewritten.
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
  const plans = await planNodeVersionUpgrades(projectPath);
  if (plans.length === 0) {
    return false;
  }

  // One confirm covering every planned change (default Yes in interactive mode;
  // proceed directly when non-interactive).
  if (interactive) {
    pauseProgress?.();
    const summary = plans
      .map((plan) => `${NODE_VERSION_SOURCE_LABELS[plan.source]} ${plan.from} → ${plan.to}`)
      .join(', ');
    const confirmed = await prompts.confirm({
      message: `Upgrade Node.js version pins below the Vite+ supported range? (${summary})`,
      initialValue: true,
    });
    if (prompts.isCancel(confirmed)) {
      cancelAndExit();
    }
    if (!confirmed) {
      return false;
    }
  }

  applyNodeVersionUpgrades(projectPath, plans);
  for (const plan of plans) {
    warnMigration(
      `Upgraded Node.js ${plan.from} to ${plan.to} (below the supported range)`,
      report,
    );
  }
  return true;
}
