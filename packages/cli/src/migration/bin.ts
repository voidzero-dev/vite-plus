import path from 'node:path';
import { styleText } from 'node:util';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import semver from 'semver';

import { parseMigrateArgs } from '../../binding/index.js';
import {
  PackageManager,
  type WorkspaceInfo,
  type WorkspaceInfoOptional,
  type WorkspacePackage,
} from '../types/index.ts';
import { writeAgentInstructions } from '../utils/agent.ts';
import { unwrapCliParseOutcome } from '../utils/cli-parse.ts';
import { isForceOverrideMode, SETUP_VP_VERSION, VITE_PLUS_VERSION } from '../utils/constants.ts';
import { writeEditorConfigs } from '../utils/editor.ts';
import { hasVitePlusDependency, readNearestPackageJson } from '../utils/package.ts';
import { displayRelative } from '../utils/path.ts';
import {
  cancelAndExit,
  type CommandRunSummary,
  defaultInteractive,
  downloadPackageManager,
  runViteInstall,
  selectPackageManager,
  upgradeYarn,
} from '../utils/prompts.ts';
import { accent, formatDuration, log, muted, printHeader, warnMsg } from '../utils/terminal.ts';
import {
  confirmBaseUrlFix,
  fixBaseUrlInTsconfig,
  hasBaseUrlInTsconfig,
} from '../utils/tsconfig.ts';
import type { PackageDependencies } from '../utils/types.ts';
import { detectWorkspace } from '../utils/workspace.ts';
import { checkRolldownCompatibility } from './compat/runner.ts';
import { canFormatWithOxfmt, collectChangedFormatPaths, formatMigratedProject } from './format.ts';
import {
  addFrameworkShim,
  checkVitestVersion,
  checkViteVersion,
  collectToolchainVersionChanges,
  confirmPrettierMigration,
  detectEslintProject,
  detectFramework,
  detectNodeVersionManagerFile,
  detectPendingCoreMigration,
  detectPrettierProject,
  detectTsupProject,
  detectVitePlusBootstrapPending,
  detectYarnPnpMode,
  ensureVitePlusBootstrap,
  finalizeCoreMigrationForExistingVitePlus,
  hasFrameworkShim,
  detectLegacyGitHooksMigrationCandidate,
  injectLintTypeCheckDefaults,
  installGitHooks,
  mergeTsdownConfigFile,
  mergeViteConfigFiles,
  migrateEslintToOxlint,
  migrateNodeVersionManagerFile,
  migratePrettierToOxfmt,
  migrateSetupVpVersion,
  migrateTsupToTsdown,
  configureYarnNodeModulesMode,
  rewriteMonorepo,
  rewriteStandaloneProject,
  shouldSkipStagedMigrationForHooks,
  warnPackageLevelPrettier,
  type Framework,
  type NodeVersionManagerDetection,
} from './migrator.ts';
import { prepareNpmViteAliasReinstall } from './npm-reinstall.ts';
import type { MigrationOptions } from './options.ts';
import { addMigrationWarning, createMigrationReport, type MigrationReport } from './report.ts';
import {
  collectMigrationSetupPlan,
  collectTsupMigrationDecision,
  type MigrationSetupPlan,
} from './setup-plan.ts';

async function confirmNodeVersionFileMigration(
  interactive: boolean,
  detection: NodeVersionManagerDetection,
): Promise<boolean> {
  const confirmMessageByFile = {
    'package.json': 'Migrate Volta node version (package.json) to .node-version?',
    '.nvmrc': 'Migrate .nvmrc to .node-version?',
  } as const satisfies Record<NodeVersionManagerDetection['file'], string>;

  const message = confirmMessageByFile[detection.file];
  if (interactive) {
    const confirmed = await prompts.confirm({
      message,
      initialValue: true,
    });
    if (prompts.isCancel(confirmed)) {
      cancelAndExit();
    }
    return confirmed;
  }
  return true;
}

async function confirmFrameworkShim(framework: Framework, interactive: boolean): Promise<boolean> {
  const frameworkNames: Record<Framework, string> = { vue: 'Vue', astro: 'Astro' };
  const name = frameworkNames[framework];
  if (interactive) {
    const confirmed = await prompts.confirm({
      message:
        `Add TypeScript shim for ${name} component files (*.${framework})?\n  ` +
        styleText(
          'gray',
          `Lets TypeScript recognize .${framework} files until vp check fully supports them.`,
        ),
      initialValue: true,
    });
    if (prompts.isCancel(confirmed)) {
      cancelAndExit();
    }
    return confirmed;
  }
  return true;
}

async function confirmYarnNodeModulesMode(
  rootDir: string,
  packageManager: PackageManager | undefined,
  packageManagerVersion: string,
  interactive: boolean,
): Promise<boolean> {
  if (packageManager !== PackageManager.yarn) {
    return false;
  }

  const pnp = detectYarnPnpMode(rootDir, packageManagerVersion);
  if (!pnp) {
    return false;
  }

  prompts.log.warn(`⚠ Vite+ does not currently support Yarn Plug'n'Play (PnP).`);
  if (pnp.source === 'environment') {
    cancelAndExit(
      'YARN_NODE_LINKER=pnp overrides project configuration. Set it to node-modules or unset it, then re-run `vp migrate`.',
      1,
    );
  }

  if (interactive) {
    const confirmed = await prompts.confirm({
      message: 'Switch this project to Yarn node-modules mode and continue?',
      initialValue: true,
    });
    if (prompts.isCancel(confirmed)) {
      cancelAndExit();
    }
    if (!confirmed) {
      cancelAndExit('Migration cancelled. Vite+ requires Yarn node-modules mode.');
    }
  }

  return true;
}

async function fixBaseUrlForWorkspace(
  workspaceInfo: { rootDir: string; packages?: WorkspacePackage[] },
  fixBaseUrl: boolean,
  updateProgress?: (message: string) => void,
  report?: MigrationReport,
): Promise<string[]> {
  if (!fixBaseUrl) {
    return [];
  }

  const fixedProjectPaths: string[] = [];
  for (const projectPath of getWorkspaceProjectPaths(workspaceInfo)) {
    if (!hasBaseUrlInTsconfig(projectPath)) {
      continue;
    }
    updateProgress?.(
      `Fixing tsconfig baseUrl${
        projectPath === workspaceInfo.rootDir
          ? ''
          : ` in ${displayRelative(projectPath, workspaceInfo.rootDir)}`
      }`,
    );
    const status = await fixBaseUrlInTsconfig(projectPath, {
      confirmed: true,
      silent: true,
    });
    if (status === 'failed') {
      const projectLabel = displayRelative(projectPath, workspaceInfo.rootDir) || '.';
      addMigrationWarning(
        report,
        `Failed to remove tsconfig baseUrl in ${projectLabel}. ` +
          'Run `vp dlx @andrewbranch/ts5to6 --fixBaseUrl <tsconfig path>` manually and re-run the migration.',
      );
    }
    if (status === 'fixed') {
      fixedProjectPaths.push(projectPath);
    }
  }
  return fixedProjectPaths;
}

function getWorkspaceProjectPaths(workspaceInfo: {
  rootDir: string;
  packages?: WorkspacePackage[];
}): string[] {
  return [
    workspaceInfo.rootDir,
    ...(workspaceInfo.packages ?? []).map((pkg) => path.join(workspaceInfo.rootDir, pkg.path)),
  ];
}

function hasBaseUrlInWorkspace(workspaceInfo: {
  rootDir: string;
  packages?: WorkspacePackage[];
}): boolean {
  return getWorkspaceProjectPaths(workspaceInfo).some(hasBaseUrlInTsconfig);
}

// Check the root and each workspace package for Rolldown-incompatible config
// patterns, reporting per-package progress. Each package spawns its own Vite
// config resolution, so on a large monorepo this is the slowest silent phase;
// the running counter + package path keeps the spinner moving instead of
// stalling on a stale message.
async function checkWorkspaceRolldownCompatibility(
  workspaceInfo: { rootDir: string; packages?: WorkspacePackage[] },
  report: MigrationReport,
  updateProgress?: (message: string) => void,
): Promise<void> {
  const projectPaths = getWorkspaceProjectPaths(workspaceInfo);
  for (const [index, projectPath] of projectPaths.entries()) {
    const counter = projectPaths.length > 1 ? ` (${index + 1}/${projectPaths.length})` : '';
    // Empty for the root (displayRelative(rootDir, rootDir) === ''), else the
    // package's relative path.
    const label = displayRelative(projectPath, workspaceInfo.rootDir);
    updateProgress?.(`Checking config compatibility${counter}${label ? `: ${label}` : ''}`);
    await checkRolldownCompatibility(projectPath, report);
  }
}

function parseArgs() {
  const parsed = unwrapCliParseOutcome(parseMigrateArgs(process.argv.slice(3)));
  const projectPath = parsed.path ? path.resolve(process.cwd(), parsed.path) : process.cwd();

  return {
    projectPath,
    options: {
      interactive: parsed.interactive ?? defaultInteractive(),
      agent: parsed.agent,
      editor: parsed.editor,
      hooks: parsed.hooks,
      full: parsed.full,
    } satisfies MigrationOptions,
  };
}

interface MigrationPlan extends MigrationSetupPlan {
  packageManager: PackageManager;
  convertYarnPnp: boolean;
  migratePrettier: boolean;
  hasPrettierDependency: boolean;
  prettierConfigFile?: string;
  migrateTsup: boolean;
  tsupConfigFile?: string;
  fixBaseUrl: boolean;
  migrateNodeVersionFile: boolean;
  nodeVersionDetection?: NodeVersionManagerDetection;
  frameworkShimFrameworks?: Framework[];
}

function getFrameworkShimCandidates(rootDir: string, packages?: WorkspacePackage[]): Framework[] {
  const allDetectedFrameworks = new Set<Framework>(detectFramework(rootDir));
  for (const pkg of packages ?? []) {
    for (const framework of detectFramework(path.join(rootDir, pkg.path))) {
      allDetectedFrameworks.add(framework);
    }
  }

  return [...allDetectedFrameworks].filter((framework) => {
    if (detectFramework(rootDir).includes(framework) && !hasFrameworkShim(rootDir, framework)) {
      return true;
    }
    return (packages ?? []).some((pkg) => {
      const pkgPath = path.join(rootDir, pkg.path);
      return detectFramework(pkgPath).includes(framework) && !hasFrameworkShim(pkgPath, framework);
    });
  });
}

async function collectFrameworkShimFrameworks(
  rootDir: string,
  options: MigrationOptions,
  packages?: WorkspacePackage[],
): Promise<Framework[] | undefined> {
  const frameworkShimFrameworks: Framework[] = [];
  for (const framework of getFrameworkShimCandidates(rootDir, packages)) {
    const addShim = await confirmFrameworkShim(framework, options.interactive);
    if (addShim) {
      frameworkShimFrameworks.push(framework);
    }
  }
  return frameworkShimFrameworks.length > 0 ? frameworkShimFrameworks : undefined;
}

function addFrameworkShimsForWorkspace(
  rootDir: string,
  frameworks: Framework[] | undefined,
  packages: WorkspacePackage[] | undefined,
  report: MigrationReport,
  updateMigrationProgress: (message: string) => void,
): boolean {
  if (!frameworks) {
    return false;
  }

  let changed = false;
  updateMigrationProgress('Adding TypeScript shim');
  for (const framework of frameworks) {
    if (detectFramework(rootDir).includes(framework) && !hasFrameworkShim(rootDir, framework)) {
      addFrameworkShim(rootDir, framework, report);
      changed = true;
    }
    for (const pkg of packages ?? []) {
      const pkgPath = path.join(rootDir, pkg.path);
      if (detectFramework(pkgPath).includes(framework) && !hasFrameworkShim(pkgPath, framework)) {
        addFrameworkShim(pkgPath, framework, report);
        changed = true;
      }
    }
  }
  return changed;
}

function hasEnabledOption(value: string | string[] | false | undefined): boolean {
  if (Array.isArray(value)) {
    return value.some((item) => Boolean(item));
  }
  return value !== undefined && value !== false && value !== '';
}

function hasExplicitExistingVitePlusSetupRequest(options: MigrationOptions): boolean {
  return (
    options.hooks === true || hasEnabledOption(options.agent) || hasEnabledOption(options.editor)
  );
}

function hasExistingVitePlusMigrationCandidates(
  workspaceInfo: WorkspaceInfoOptional,
  options: MigrationOptions,
): boolean {
  const eslintProject = detectEslintProject(workspaceInfo.rootDir, workspaceInfo.packages);
  const prettierProject = detectPrettierProject(workspaceInfo.rootDir, workspaceInfo.packages);
  const tsupProject = detectTsupProject(workspaceInfo.rootDir, workspaceInfo.packages);
  return (
    hasExplicitExistingVitePlusSetupRequest(options) ||
    detectLegacyGitHooksMigrationCandidate(workspaceInfo.rootDir) ||
    hasBaseUrlInWorkspace(workspaceInfo) ||
    eslintProject.hasDependency ||
    prettierProject.hasDependency ||
    tsupProject.hasDependency ||
    detectNodeVersionManagerFile(workspaceInfo.rootDir) !== undefined ||
    getFrameworkShimCandidates(workspaceInfo.rootDir, workspaceInfo.packages).length > 0
  );
}

function getExistingVitePlusSetupOptions(
  options: MigrationOptions,
  useFullMigrationDefaults = false,
): MigrationOptions {
  if (useFullMigrationDefaults) {
    return options;
  }
  // On a plain version-update, defer all setup (hooks, agent, editor) to
  // `vp migrate --full`; legacy git-hooks migration (husky/lint-staged) is not
  // auto-applied here. Explicit per-action flags (e.g. --hooks) still opt in.
  return {
    ...options,
    hooks: options.hooks ?? false,
    agent: options.agent ?? false,
    editor: options.editor ?? false,
  };
}

async function collectMigrationPlan(
  rootDir: string,
  detectedPackageManager: PackageManager | undefined,
  detectedPackageManagerVersion: string,
  options: MigrationOptions,
  packages?: WorkspacePackage[],
): Promise<MigrationPlan> {
  // 1. Package manager selection
  const packageManager =
    detectedPackageManager ?? (await selectPackageManager(options.interactive, true));
  const convertYarnPnp = await confirmYarnNodeModulesMode(
    rootDir,
    packageManager,
    detectedPackageManager ? detectedPackageManagerVersion : 'latest',
    options.interactive,
  );

  // 2. Shared setup/tooling decisions
  const setupPlan = await collectMigrationSetupPlan(rootDir, packageManager, options, packages);

  // 3. Prettier detection + prompt
  const prettierProject = detectPrettierProject(rootDir, packages);
  let migratePrettier = false;
  if (prettierProject.hasDependency && prettierProject.configFile) {
    migratePrettier = await confirmPrettierMigration(options.interactive);
  } else if (prettierProject.hasDependency) {
    warnPackageLevelPrettier();
  }

  // 3b. tsup detection + prompt (after Prettier so Prettier -> Oxfmt is checked first)
  const { migrateTsup, tsupConfigFile } = await collectTsupMigrationDecision(
    rootDir,
    options,
    packages,
  );

  // 9. tsconfig baseUrl prompt
  const fixBaseUrl = hasBaseUrlInWorkspace({ rootDir, packages })
    ? await confirmBaseUrlFix(options.interactive)
    : false;

  // 10. Node version manager file detection + prompt
  const nodeVersionDetection = detectNodeVersionManagerFile(rootDir);
  let migrateNodeVersionFile = false;
  if (nodeVersionDetection) {
    migrateNodeVersionFile = await confirmNodeVersionFileMigration(
      options.interactive,
      nodeVersionDetection,
    );
  }

  // 11. Framework shim detection + prompt
  const frameworkShimFrameworks = await collectFrameworkShimFrameworks(rootDir, options, packages);

  const plan: MigrationPlan = {
    packageManager,
    convertYarnPnp,
    ...setupPlan,
    migratePrettier,
    hasPrettierDependency: prettierProject.hasDependency,
    prettierConfigFile: prettierProject.configFile,
    migrateTsup,
    tsupConfigFile,
    fixBaseUrl,
    migrateNodeVersionFile,
    nodeVersionDetection,
    frameworkShimFrameworks,
  };

  return plan;
}

/**
 * Reconcile a CommandRunSummary from `runViteInstall` with the migration's
 * duration counter and exit-code state. `runViteInstall` returns
 * `{ status: 'failed', exitCode }` without throwing; treating that as a success
 * (incrementing duration unconditionally) would let the migration claim
 * "Dependencies installed" while node_modules is desynced from the just-mutated
 * package.json. This helper centralizes the right handling: credit duration on
 * success, warn + flip exitCode on failure, stay silent on skip.
 */
function handleInstallResult(
  installSummary: CommandRunSummary,
  rootDir: string,
  report: MigrationReport,
  // The pre-migration "initial" install is best-effort: the migration proceeds
  // regardless of its outcome, and a post-migration "final" install runs with
  // `--force` / `--no-frozen-lockfile` as the authoritative recovery. Only that
  // final install's failure should flip `process.exitCode` so a successful
  // recovery yields exit 0; the initial failure is still surfaced via
  // `report.warnings` + the warn message.
  options?: { propagateExitCode?: boolean },
): number {
  if (installSummary.status === 'installed') {
    return installSummary.durationMs;
  }
  if (installSummary.status === 'failed') {
    const exitCode = installSummary.exitCode ?? 1;
    const message = `Dependency installation failed (exit code ${exitCode}). Run \`vp install\` manually in ${rootDir} to resync node_modules.`;
    warnMsg(message);
    report.warnings.push(message);
    if (options?.propagateExitCode !== false) {
      process.exitCode = exitCode;
    }
    return 0;
  }
  return 0;
}

// Shown on a bare existing-Vite+ upgrade that skipped the setup bucket, on both
// the "already using Vite+" early-exit and the post-migration summary.
const FULL_MIGRATION_HINT = `${styleText('gray', '•')} Skipped editor, hooks, and lint setup. Run \`vp migrate --full\` to apply them.`;

function showMigrationSummary(options: {
  projectRoot: string;
  packageManager: string;
  packageManagerVersion: string;
  installDurationMs: number;
  finalInstallOk: boolean;
  report: MigrationReport;
  updatedExistingVitePlus?: boolean;
  suggestFullMigration?: boolean;
}) {
  const {
    projectRoot,
    packageManager,
    packageManagerVersion,
    installDurationMs,
    finalInstallOk,
    report,
    updatedExistingVitePlus,
    suggestFullMigration,
  } = options;
  const projectLabel = displayRelative(projectRoot) || '.';
  const configUpdates =
    report.createdViteConfigCount +
    report.mergedConfigCount +
    report.mergedStagedConfigCount +
    report.inlinedLintStagedConfigCount +
    report.removedConfigCount +
    report.tsdownImportCount +
    report.wrappedPluginConfigCount;

  log(
    `${styleText('magenta', '◇')} ${updatedExistingVitePlus ? 'Updated' : 'Migrated'} ${accent(projectLabel)} to Vite+ ${VITE_PLUS_VERSION}`,
  );
  log(
    `${styleText('gray', '•')} Node ${process.versions.node}  ${packageManager} ${packageManagerVersion}`,
  );
  // Toolchain version-upgrade table (existing-Vite+ path only; the fresh path
  // leaves `dependencyUpgrades` empty). One aligned row per changed dependency:
  // name padded to the longest name, the old `from` padded to the longest from
  // and rendered muted, the new `to` rendered green.
  if (report.dependencyUpgrades.length > 0) {
    const nameWidth = Math.max(...report.dependencyUpgrades.map((change) => change.name.length));
    const fromWidth = Math.max(
      ...report.dependencyUpgrades.map((change) => (change.from ?? '').length),
    );
    log(`${styleText('gray', '•')} Dependencies:`);
    for (const change of report.dependencyUpgrades) {
      const name = change.name.padEnd(nameWidth);
      const from = muted((change.from ?? '').padEnd(fromWidth));
      const to = styleText('green', change.to);
      log(`    ${name}  ${from} → ${to}`);
    }
  }
  // Gate the green success line on the FINAL install actually succeeding.
  // A nonzero duration could come from a successful pre-migration install
  // followed by a failed post-migration reinstall — in that case node_modules
  // is desynced and reporting success would mislead the user.
  if (finalInstallOk && installDurationMs > 0) {
    log(
      `${styleText('green', '✓')} Dependencies installed in ${formatDuration(installDurationMs)}`,
    );
  }
  if (configUpdates > 0 || report.rewrittenImportFileCount > 0) {
    const parts: string[] = [];
    if (configUpdates > 0) {
      parts.push(
        `${configUpdates} ${configUpdates === 1 ? 'config update' : 'config updates'} applied`,
      );
    }
    if (report.rewrittenImportFileCount > 0) {
      parts.push(
        `${report.rewrittenImportFileCount} ${
          report.rewrittenImportFileCount === 1 ? 'file had' : 'files had'
        } imports rewritten`,
      );
    }
    log(`${styleText('gray', '•')} ${parts.join(', ')}`);
  }
  if (report.preservedUpstreamVitestImportFileCount > 0) {
    log(
      `${styleText('gray', '•')} Kept upstream \`vitest\` imports in ${report.preservedUpstreamVitestImportFileCount} ${
        report.preservedUpstreamVitestImportFileCount === 1 ? 'file' : 'files'
      } for @nuxt/test-utils compatibility`,
    );
  }
  if (report.eslintMigrated) {
    log(`${styleText('gray', '•')} ESLint rules migrated to Oxlint`);
  }
  if (report.prettierMigrated) {
    log(`${styleText('gray', '•')} Prettier migrated to Oxfmt`);
  }
  if (report.tsupMigrated) {
    log(`${styleText('gray', '•')} tsup config migrated to tsdown (\`vp pack\`)`);
  }
  if (report.nodeVersionFileMigrated) {
    log(`${styleText('gray', '•')} Node version manager file migrated to .node-version`);
  }
  const setupVpFileCount = report.setupVpVersionUpdatedFileCount;
  if (setupVpFileCount > 0) {
    const fileLabel = setupVpFileCount === 1 ? 'file' : 'files';
    log(
      `${styleText('gray', '•')} setup-vp updated to ${SETUP_VP_VERSION} in ${setupVpFileCount} GitHub Actions ${fileLabel}`,
    );
  }
  if (report.wrappedPluginConfigCount > 0) {
    log(
      `${styleText('gray', '•')} Inline Vite plugins wrapped with lazyPlugins for check/lint/fmt`,
    );
  }
  if (report.gitHooksConfigured) {
    log(`${styleText('gray', '•')} Git hooks configured`);
  }
  if (report.frameworkShimAdded) {
    log(`${styleText('gray', '•')} TypeScript shim added for framework component files`);
  }
  if (report.packageManagerBootstrapConfigured) {
    log(`${styleText('gray', '•')} Package manager settings configured`);
  }
  if (suggestFullMigration) {
    log(FULL_MIGRATION_HINT);
  }
  if (report.warnings.length > 0) {
    log(`${styleText('yellow', '!')} Warnings:`);
    for (const warning of report.warnings) {
      log(`  - ${warning}`);
    }
  }
  if (report.manualSteps.length > 0) {
    log(`${styleText('blue', '→')} Manual follow-up:`);
    for (const step of report.manualSteps) {
      log(`  - ${step}`);
    }
  }
}

async function downloadSupportedPackageManager(options: {
  rootDir: string;
  packageManager: PackageManager;
  packageManagerVersion: string;
  interactive: boolean;
  updateMigrationProgress: (message: string) => void;
  failMigrationProgress: (message: string) => void;
}): Promise<Awaited<ReturnType<typeof downloadPackageManager>>> {
  const {
    rootDir,
    packageManager,
    packageManagerVersion,
    interactive,
    updateMigrationProgress,
    failMigrationProgress,
  } = options;

  updateMigrationProgress('Preparing migration');
  const downloadResult = await downloadPackageManager(
    packageManager,
    packageManagerVersion,
    interactive,
    true,
  );

  if (
    packageManager === PackageManager.yarn &&
    semver.satisfies(downloadResult.version, '>=4.0.0 <4.10.0')
  ) {
    updateMigrationProgress('Upgrading Yarn');
    await upgradeYarn(rootDir, interactive, true);
  } else if (
    packageManager === PackageManager.pnpm &&
    semver.satisfies(downloadResult.version, '< 9.5.0')
  ) {
    failMigrationProgress('Migration failed');
    prompts.log.error(
      `✘ pnpm@${downloadResult.version} is not supported by auto migration, please upgrade pnpm to >=9.5.0 first`,
    );
    cancelAndExit('Vite+ cannot automatically migrate this project yet.', 1);
  } else if (
    packageManager === PackageManager.npm &&
    semver.satisfies(downloadResult.version, '< 8.3.0')
  ) {
    failMigrationProgress('Migration failed');
    prompts.log.error(
      `✘ npm@${downloadResult.version} is not supported by auto migration, please upgrade npm to >=8.3.0 first`,
    );
    cancelAndExit('Vite+ cannot automatically migrate this project yet.', 1);
  }

  return downloadResult;
}

async function executeMigrationPlan(
  workspaceInfoOptional: WorkspaceInfoOptional,
  plan: MigrationPlan,
  interactive: boolean,
  preExistingChangedPaths?: ReadonlySet<string>,
): Promise<{
  installDurationMs: number;
  finalInstallOk: boolean;
  packageManagerVersion: string;
  report: MigrationReport;
}> {
  const report = createMigrationReport();
  const migrationProgress = interactive ? prompts.spinner({ indicator: 'timer' }) : undefined;
  let migrationProgressStarted = false;
  const updateMigrationProgress = (message: string) => {
    if (!migrationProgress) {
      return;
    }
    if (migrationProgressStarted) {
      migrationProgress.message(message);
      return;
    }
    migrationProgress.start(message);
    migrationProgressStarted = true;
  };
  const clearMigrationProgress = () => {
    if (migrationProgress && migrationProgressStarted) {
      migrationProgress.clear();
      migrationProgressStarted = false;
    }
  };
  const failMigrationProgress = (message: string) => {
    if (migrationProgress && migrationProgressStarted) {
      migrationProgress.error(message);
      migrationProgressStarted = false;
    }
  };

  // 1. Download package manager + version validation
  const downloadResult = await downloadSupportedPackageManager({
    rootDir: workspaceInfoOptional.rootDir,
    packageManager: plan.packageManager,
    packageManagerVersion: workspaceInfoOptional.packageManagerVersion,
    interactive,
    updateMigrationProgress,
    failMigrationProgress,
  });
  const workspaceInfo: WorkspaceInfo = {
    ...workspaceInfoOptional,
    packageManager: plan.packageManager,
    downloadPackageManager: downloadResult,
  };

  if (plan.convertYarnPnp) {
    updateMigrationProgress('Configuring Yarn node-modules mode');
    report.packageManagerBootstrapConfigured = configureYarnNodeModulesMode(workspaceInfo.rootDir);
    if (report.packageManagerBootstrapConfigured) {
      prompts.log.success('✔ Switched Yarn to node-modules mode');
    }
  }

  // 3. Migrate node version manager file → .node-version (independent of vite version)
  if (plan.migrateNodeVersionFile && plan.nodeVersionDetection) {
    updateMigrationProgress('Migrating node version file');
    migrateNodeVersionManagerFile(workspaceInfo.rootDir, plan.nodeVersionDetection, report);
  }

  updateMigrationProgress('Updating setup-vp workflows');
  migrateSetupVpVersion(workspaceInfo.rootDir, report);

  // 4. Run vp install to ensure the project is ready
  updateMigrationProgress('Installing dependencies');
  const initialInstallSummary = await runViteInstall(
    workspaceInfo.rootDir,
    interactive,
    undefined,
    {
      silent: true,
      packageManager: workspaceInfo.packageManager,
      packageManagerVersion: workspaceInfo.downloadPackageManager.version,
    },
  );

  // 4. Check vite and vitest version is supported by migration
  updateMigrationProgress('Validating toolchain');
  const isViteSupported = checkViteVersion(workspaceInfo.rootDir);
  const isVitestSupported = checkVitestVersion(workspaceInfo.rootDir);
  if (!isViteSupported || !isVitestSupported) {
    failMigrationProgress('Migration failed');
    cancelAndExit('Vite+ cannot automatically migrate this project yet.', 1);
  }

  // 5. Check for Rolldown-incompatible config patterns (root + workspace packages)
  await checkWorkspaceRolldownCompatibility(workspaceInfo, report, updateMigrationProgress);

  await fixBaseUrlForWorkspace(workspaceInfo, plan.fixBaseUrl, updateMigrationProgress, report);

  // 6. ESLint → Oxlint migration (before main rewrite so .oxlintrc.json gets picked up)
  if (plan.migrateEslint) {
    updateMigrationProgress('Migrating ESLint');
    const eslintOk = await migrateEslintToOxlint(
      workspaceInfo.rootDir,
      interactive,
      plan.eslintConfigFile,
      workspaceInfo.packages,
      { silent: true, report },
    );
    if (!eslintOk) {
      failMigrationProgress('Migration failed');
      cancelAndExit('ESLint migration failed. Fix the issue and re-run `vp migrate`.', 1);
    }
  }

  // 5b. Prettier → Oxfmt migration (before main rewrite so .oxfmtrc.json gets picked up)
  if (plan.migratePrettier) {
    updateMigrationProgress('Migrating Prettier');
    const prettierOk = await migratePrettierToOxfmt(
      workspaceInfo.rootDir,
      interactive,
      plan.prettierConfigFile,
      workspaceInfo.packages,
      { silent: true, report },
    );
    if (!prettierOk) {
      failMigrationProgress('Migration failed');
      cancelAndExit('Prettier migration failed. Fix the issue and re-run `vp migrate`.', 1);
    }
  }

  // 6b. tsup → tsdown migration (before main rewrite so tsdown.config.* gets picked up)
  if (plan.migrateTsup) {
    updateMigrationProgress('Migrating tsup');
    const tsupOk = await migrateTsupToTsdown(
      workspaceInfo.rootDir,
      interactive,
      plan.packageManager,
      plan.tsupConfigFile,
      workspaceInfo.packages,
      { silent: true, report },
    );
    if (!tsupOk) {
      failMigrationProgress('Migration failed');
      cancelAndExit('Complete the tsup migration manually, then re-run `vp migrate`.', 1);
    }
  }

  // Preserve lint-staged whenever hook setup is disabled/unsafe or existing
  // project-owned hooks remain authoritative.
  const skipStagedMigration = shouldSkipStagedMigrationForHooks(
    workspaceInfo.rootDir,
    plan.shouldSetupHooks,
    plan.packageManager,
    workspaceInfo.packages,
  );

  // 7. Rewrite configs
  updateMigrationProgress('Rewriting configs');
  if (workspaceInfo.isMonorepo) {
    rewriteMonorepo(workspaceInfo, skipStagedMigration, true, report);
  } else {
    rewriteStandaloneProject(
      workspaceInfo.rootDir,
      workspaceInfo,
      skipStagedMigration,
      true,
      report,
    );
  }

  // 8. Install git hooks
  if (plan.shouldSetupHooks) {
    updateMigrationProgress('Configuring git hooks');
    installGitHooks(
      workspaceInfo.rootDir,
      true,
      report,
      plan.packageManager,
      workspaceInfo.packages,
    );
  }

  // 9. Write agent instructions (using pre-resolved decisions)
  updateMigrationProgress('Writing agent instructions');
  await writeAgentInstructions({
    projectRoot: workspaceInfo.rootDir,
    targetPaths: plan.selectedAgentTargetPaths,
    interactive,
    conflictDecisions: plan.agentConflictDecisions,
    silent: true,
  });

  // 10. Write editor configs (using pre-resolved decisions)
  updateMigrationProgress('Writing editor configs');
  await writeEditorConfigs({
    projectRoot: workspaceInfo.rootDir,
    editorId: plan.selectedEditor,
    interactive,
    conflictDecisions: plan.editorConflictDecisions,
    silent: true,
    packageManager: plan.packageManager,
  });

  // 11. Add framework shims if requested
  addFrameworkShimsForWorkspace(
    workspaceInfo.rootDir,
    plan.frameworkShimFrameworks,
    workspaceInfo.packages,
    report,
    updateMigrationProgress,
  );

  // 12. Reinstall after migration
  // The migration intentionally rewrites overrides/catalogs/deps, so the
  // existing lockfile is guaranteed to be stale. Tell each package manager to
  // re-resolve instead of refusing the install (pnpm/yarn default to
  // frozen-lockfile under CI, npm/bun need an explicit --force).
  const installArgs =
    plan.packageManager === PackageManager.npm || plan.packageManager === PackageManager.bun
      ? ['--force']
      : ['--no-frozen-lockfile'];
  const npmReinstallPreparation =
    plan.packageManager === PackageManager.npm
      ? prepareNpmViteAliasReinstall(workspaceInfo.rootDir, getWorkspaceProjectPaths(workspaceInfo))
      : undefined;
  updateMigrationProgress('Installing dependencies');
  const finalInstallSummary = await runViteInstall(
    workspaceInfo.rootDir,
    interactive,
    installArgs,
    {
      silent: true,
      packageManager: workspaceInfo.packageManager,
      packageManagerVersion: workspaceInfo.downloadPackageManager.version,
    },
  );
  if (finalInstallSummary.status === 'failed') {
    npmReinstallPreparation?.restore();
  } else {
    npmReinstallPreparation?.commit();
  }

  clearMigrationProgress();
  // Process the initial install first so the final install's exit code "wins":
  // if the initial install failed but the final install succeeded, the
  // migration should still report success (exit 0). The initial call opts out
  // of exitCode propagation; only the final call may flip process.exitCode.
  const initialInstallDurationMs = handleInstallResult(
    initialInstallSummary,
    workspaceInfo.rootDir,
    report,
    { propagateExitCode: false },
  );
  const finalInstallDurationMs = handleInstallResult(
    finalInstallSummary,
    workspaceInfo.rootDir,
    report,
  );
  if (
    finalInstallSummary.status === 'installed' &&
    canFormatWithOxfmt(plan.hasPrettierDependency, plan.migratePrettier)
  ) {
    await formatMigratedProject(workspaceInfo.rootDir, interactive, report, {
      excludedPaths: preExistingChangedPaths,
    });
  }
  return {
    installDurationMs: initialInstallDurationMs + finalInstallDurationMs,
    finalInstallOk: finalInstallSummary.status === 'installed',
    packageManagerVersion: downloadResult.version,
    report,
  };
}

async function main() {
  const { projectPath, options } = parseArgs();

  printHeader();

  const workspaceInfoOptional = await detectWorkspace(projectPath);
  if (
    workspaceInfoOptional.isMonorepo &&
    path.resolve(projectPath) !== path.resolve(workspaceInfoOptional.rootDir)
  ) {
    cancelAndExit(
      `Vite+ cannot migrate a workspace member. Run \`vp migrate\` from the workspace root at ${workspaceInfoOptional.rootDir}.`,
      1,
    );
  }
  const initialChangedPaths = await collectChangedFormatPaths(workspaceInfoOptional.rootDir);
  const preExistingChangedPaths = initialChangedPaths ? new Set(initialChangedPaths) : undefined;
  const resolvedPackageManager = workspaceInfoOptional.packageManager ?? 'unknown';

  // Early return if already using Vite+ (only finalization/setup migrations may be needed)
  // In force-override mode (file: tgz overrides), skip this check and run full migration
  const rootPkg = readNearestPackageJson(
    workspaceInfoOptional.rootDir,
  ) as PackageDependencies | null;
  if (hasVitePlusDependency(rootPkg) && !isForceOverrideMode()) {
    // Runs with the detected package manager, which may be undefined for an
    // existing Vite+ project that has no lockfile/`packageManager` pin. In that
    // case `confirmYarnNodeModulesMode` no-ops here and the guard is re-run
    // below once the package manager is actually resolved.
    let convertYarnPnp = await confirmYarnNodeModulesMode(
      workspaceInfoOptional.rootDir,
      workspaceInfoOptional.packageManager,
      workspaceInfoOptional.packageManagerVersion,
      options.interactive,
    );
    let didMigrate = false;
    let installDurationMs = 0;
    let finalInstallOk = true;
    let canFormatMigratedProject = !process.env.VP_SKIP_INSTALL;
    const report = createMigrationReport();
    const migrationProgress = options.interactive
      ? prompts.spinner({ indicator: 'timer' })
      : undefined;
    let migrationProgressStarted = false;
    const updateMigrationProgress = (message: string) => {
      if (!migrationProgress) {
        return;
      }
      if (migrationProgressStarted) {
        migrationProgress.message(message);
        return;
      }
      migrationProgress.start(message);
      migrationProgressStarted = true;
    };
    const clearMigrationProgress = () => {
      if (migrationProgress && migrationProgressStarted) {
        migrationProgress.clear();
        migrationProgressStarted = false;
      }
    };
    const failMigrationProgress = (message: string) => {
      if (migrationProgress && migrationProgressStarted) {
        migrationProgress.error(message);
        migrationProgressStarted = false;
      }
    };

    updateMigrationProgress('Updating setup-vp workflows');
    if (migrateSetupVpVersion(workspaceInfoOptional.rootDir, report).length > 0) {
      didMigrate = true;
    }

    const pendingCoreMigration = detectPendingCoreMigration(workspaceInfoOptional);
    const vitePlusBootstrapPending = detectVitePlusBootstrapPending(
      workspaceInfoOptional.rootDir,
      workspaceInfoOptional.packageManager,
      workspaceInfoOptional.packages,
      workspaceInfoOptional.packageManagerVersion,
    );
    let packageManager: PackageManager | undefined = vitePlusBootstrapPending
      ? (workspaceInfoOptional.packageManager ??
        (await selectPackageManager(options.interactive, true)))
      : workspaceInfoOptional.packageManager;
    let downloadedPackageManager: Awaited<ReturnType<typeof downloadPackageManager>> | undefined;
    let packageManagerVersion = workspaceInfoOptional.packageManagerVersion;
    const downloadExistingPackageManager = async () => {
      if (!packageManager) {
        return undefined;
      }
      downloadedPackageManager ??= await downloadSupportedPackageManager({
        rootDir: workspaceInfoOptional.rootDir,
        packageManager,
        packageManagerVersion,
        interactive: options.interactive,
        updateMigrationProgress,
        failMigrationProgress,
      });
      packageManagerVersion = downloadedPackageManager.version;
      return downloadedPackageManager;
    };
    const ensureExistingPackageManager = async () => {
      packageManager ??= await selectPackageManager(options.interactive, true);
      return downloadExistingPackageManager();
    };

    if (vitePlusBootstrapPending) {
      await ensureExistingPackageManager();
    }

    // The package-manager download above starts the "Preparing migration"
    // spinner. Stop it before gathering interactive decisions below: a live
    // spinner keeps re-rendering its timer line over the prompts and corrupts
    // them (the spinner is restarted for the bootstrap/install phase).
    clearMigrationProgress();

    // The early guard ran before the package manager was resolved. If it was
    // only determined to be Yarn afterwards (e.g. selected because the project
    // had no detectable manager), re-run the guard so a `YARN_NODE_LINKER=pnp`
    // override is still rejected instead of silently writing config for an
    // unsupported PnP layout.
    if (
      !convertYarnPnp &&
      workspaceInfoOptional.packageManager === undefined &&
      packageManager === PackageManager.yarn
    ) {
      convertYarnPnp = await confirmYarnNodeModulesMode(
        workspaceInfoOptional.rootDir,
        packageManager,
        packageManagerVersion,
        options.interactive,
      );
    }

    // Capture the toolchain version changes BEFORE the reconcile mutates the
    // manifest, so the `from` values still reflect the pre-migration versions
    // (raw vite is read from the untouched node_modules).
    report.dependencyUpgrades = await collectToolchainVersionChanges(workspaceInfoOptional.rootDir);

    // finalizeCoreMigrationForExistingVitePlus rewrites Vite/Vitest imports across
    // every workspace package via a synchronous native directory walk; on a large
    // monorepo it is the slowest phase (several seconds) and blocks the event loop,
    // so surface it first instead of leaving the spinner silent. The spinner paints
    // this message synchronously on start(), before the walk begins.
    updateMigrationProgress('Rewriting toolchain imports across the workspace');
    const coreMigrationResult = finalizeCoreMigrationForExistingVitePlus(
      workspaceInfoOptional,
      true,
      report,
      pendingCoreMigration,
    );
    if (
      coreMigrationResult.scripts ||
      coreMigrationResult.tsconfigTypes ||
      coreMigrationResult.imports
    ) {
      didMigrate = true;
    }

    // finalizeCore has finished; stop the spinner before the interactive setup
    // prompts below (collectMigrationSetupPlan / framework shims, in --full mode)
    // so they don't render on top of the still-running progress line.
    clearMigrationProgress();

    // On an existing Vite+ project, `vp migrate` only upgrades the toolchain
    // version. The full setup bucket (hooks, editor, agent files, ESLint/Prettier
    // migration, framework shims, tsconfig baseUrl, .node-version) runs only with
    // `--full`; the per-action flags (--hooks/--agent/--editor) still opt into a
    // single action without `--full`.
    const fullSetup = options.full === true;
    // Hint at `--full` only on a bare upgrade that left setup actions on the
    // table. Suppress it when the user passed an explicit per-action flag
    // (--hooks/--agent/--editor): that ran the action they asked for, so a
    // "Skipped ... hooks ... setup" line would contradict the action above it.
    const skippedSetupCandidates =
      !fullSetup &&
      !hasExplicitExistingVitePlusSetupRequest(options) &&
      hasExistingVitePlusMigrationCandidates(workspaceInfoOptional, options);

    if (
      !didMigrate &&
      !convertYarnPnp &&
      report.warnings.length === 0 &&
      !vitePlusBootstrapPending &&
      !(fullSetup
        ? hasExistingVitePlusMigrationCandidates(workspaceInfoOptional, options)
        : hasExplicitExistingVitePlusSetupRequest(options))
    ) {
      if (skippedSetupCandidates) {
        log(FULL_MIGRATION_HINT);
      }
      prompts.outro(`This project is already using Vite+! ${accent('Happy coding!')}`);
      return;
    }

    const setupOptions = getExistingVitePlusSetupOptions(options, fullSetup);
    const plan = await collectMigrationSetupPlan(
      workspaceInfoOptional.rootDir,
      packageManager,
      setupOptions,
      workspaceInfoOptional.packages,
      fullSetup,
    );
    const frameworkShimFrameworks = fullSetup
      ? await collectFrameworkShimFrameworks(
          workspaceInfoOptional.rootDir,
          options,
          workspaceInfoOptional.packages,
        )
      : undefined;

    let needsInstall = false;
    if (vitePlusBootstrapPending) {
      const downloadResult = await ensureExistingPackageManager();
      if (downloadResult && packageManager) {
        updateMigrationProgress('Configuring package manager');
        const bootstrapResult = ensureVitePlusBootstrap(
          {
            ...workspaceInfoOptional,
            packageManager,
            downloadPackageManager: downloadResult,
          },
          report,
        );
        didMigrate = bootstrapResult.changed || didMigrate;
        needsInstall = bootstrapResult.changed || needsInstall;
      }
    }

    let fixBaseUrl = false;
    if (fullSetup && hasBaseUrlInWorkspace(workspaceInfoOptional)) {
      // Interactive only: stop the "Configuring package manager" spinner so it
      // does not animate beneath the confirm prompt; the next progress update
      // restarts it. Non-interactive shows no prompt, so its output is unchanged.
      if (options.interactive) {
        clearMigrationProgress();
      }
      fixBaseUrl = await confirmBaseUrlFix(options.interactive);
    }

    // Check if tsconfig baseUrl migration is needed
    const fixedBaseUrlProjectPaths = await fixBaseUrlForWorkspace(
      workspaceInfoOptional,
      fixBaseUrl,
      updateMigrationProgress,
      report,
    );
    if (fixedBaseUrlProjectPaths.length > 0) {
      updateMigrationProgress('Updating lint defaults');
      for (const projectPath of fixedBaseUrlProjectPaths) {
        injectLintTypeCheckDefaults(projectPath, true, report);
      }
      didMigrate = true;
    }
    clearMigrationProgress();

    let eslintMigrated = false;
    if (plan.migrateEslint) {
      await ensureExistingPackageManager();
      updateMigrationProgress('Migrating ESLint');
      const eslintOk = await migrateEslintToOxlint(
        workspaceInfoOptional.rootDir,
        options.interactive,
        plan.eslintConfigFile,
        workspaceInfoOptional.packages,
        { silent: true, report },
      );
      if (!eslintOk) {
        clearMigrationProgress();
        cancelAndExit('ESLint migration failed. Fix the issue and re-run `vp migrate`.', 1);
      }
      eslintMigrated = true;
    }

    // Detect Prettier unconditionally so the formatting gate below skips Oxfmt on
    // a project that still uses Prettier, even on a bare (non-`--full`) upgrade
    // that rewrites imports/scripts. The Prettier MIGRATION itself stays in the
    // full-setup bucket.
    const prettierProject = detectPrettierProject(
      workspaceInfoOptional.rootDir,
      workspaceInfoOptional.packages,
    );
    let prettierMigrated = false;
    if (fullSetup) {
      if (prettierProject.hasDependency && prettierProject.configFile) {
        // Interactive only: stop any active spinner (e.g. "Migrating ESLint") so
        // it does not animate beneath the confirm prompt.
        if (options.interactive) {
          clearMigrationProgress();
        }
        const migratePrettier = await confirmPrettierMigration(options.interactive);
        if (migratePrettier) {
          await ensureExistingPackageManager();
          updateMigrationProgress('Migrating Prettier');
          const prettierOk = await migratePrettierToOxfmt(
            workspaceInfoOptional.rootDir,
            options.interactive,
            prettierProject.configFile,
            workspaceInfoOptional.packages,
            { silent: true, report },
          );
          if (!prettierOk) {
            clearMigrationProgress();
            cancelAndExit('Prettier migration failed. Fix the issue and re-run `vp migrate`.', 1);
          }
          prettierMigrated = true;
        }
      } else if (prettierProject.hasDependency) {
        warnPackageLevelPrettier();
      }
    }

    let tsupMigrated = false;
    if (fullSetup) {
      // Interactive only: stop any active spinner (e.g. "Migrating Prettier") so
      // it does not animate beneath the confirm prompt.
      if (options.interactive) {
        clearMigrationProgress();
      }
      const { migrateTsup, tsupConfigFile } = await collectTsupMigrationDecision(
        workspaceInfoOptional.rootDir,
        setupOptions,
        workspaceInfoOptional.packages,
      );
      if (migrateTsup) {
        await ensureExistingPackageManager();
        updateMigrationProgress('Migrating tsup');
        const tsupOk = await migrateTsupToTsdown(
          workspaceInfoOptional.rootDir,
          options.interactive,
          packageManager!, // is it safe to do this?
          tsupConfigFile,
          workspaceInfoOptional.packages,
          { silent: true, report },
        );
        if (!tsupOk) {
          clearMigrationProgress();
          cancelAndExit('Complete the tsup migration manually, then re-run `vp migrate`.', 1);
        }
        tsupMigrated = true;
      }
    }

    // Check if node version manager file migration is needed (full setup only)
    if (fullSetup) {
      const nodeVersionDetection = detectNodeVersionManagerFile(workspaceInfoOptional.rootDir);
      if (nodeVersionDetection) {
        // Interactive only: stop any active spinner so it does not animate beneath
        // the confirm prompt.
        if (options.interactive) {
          clearMigrationProgress();
        }
        const confirmed = await confirmNodeVersionFileMigration(
          options.interactive,
          nodeVersionDetection,
        );
        if (
          confirmed &&
          migrateNodeVersionManagerFile(workspaceInfoOptional.rootDir, nodeVersionDetection, report)
        ) {
          didMigrate = true;
        }
      }
    }

    if (convertYarnPnp) {
      updateMigrationProgress('Configuring Yarn node-modules mode');
      const yarnPnpConverted = configureYarnNodeModulesMode(workspaceInfoOptional.rootDir);
      if (yarnPnpConverted) {
        prompts.log.success('✔ Switched Yarn to node-modules mode');
        report.packageManagerBootstrapConfigured = true;
        didMigrate = true;
        needsInstall = true;
      }
    }

    if (
      addFrameworkShimsForWorkspace(
        workspaceInfoOptional.rootDir,
        frameworkShimFrameworks,
        workspaceInfoOptional.packages,
        report,
        updateMigrationProgress,
      )
    ) {
      didMigrate = true;
    }

    // Merge configs and reinstall once if any tool or bootstrap migration happened
    if (eslintMigrated || prettierMigrated || tsupMigrated) {
      updateMigrationProgress('Rewriting configs');
      mergeViteConfigFiles(
        workspaceInfoOptional.rootDir,
        true,
        report,
        workspaceInfoOptional.packages,
      );
      if (tsupMigrated) {
        mergeTsdownConfigFile(workspaceInfoOptional.rootDir, true, report);
        for (const pkg of workspaceInfoOptional.packages ?? []) {
          mergeTsdownConfigFile(path.join(workspaceInfoOptional.rootDir, pkg.path), true, report);
        }
      }
      needsInstall = true;
      didMigrate = true;
      report.eslintMigrated = eslintMigrated;
      report.prettierMigrated = prettierMigrated;
      report.tsupMigrated = tsupMigrated;
    }

    if (plan.shouldSetupHooks) {
      await ensureExistingPackageManager();
      updateMigrationProgress('Configuring git hooks');
      if (
        installGitHooks(
          workspaceInfoOptional.rootDir,
          true,
          report,
          packageManager,
          workspaceInfoOptional.packages,
        )
      ) {
        didMigrate = true;
        needsInstall = true;
      }
    }

    if (needsInstall) {
      const resolved = await ensureExistingPackageManager();
      updateMigrationProgress('Installing dependencies');
      const resolvedVersion = resolved?.version ?? packageManagerVersion;
      const npmReinstallPreparation =
        packageManager === PackageManager.npm
          ? prepareNpmViteAliasReinstall(
              workspaceInfoOptional.rootDir,
              getWorkspaceProjectPaths(workspaceInfoOptional),
            )
          : undefined;
      const installSummary = await runViteInstall(
        workspaceInfoOptional.rootDir,
        options.interactive,
        // Migration steps rewrote package.json/config, so the lockfile is now
        // stale; tell each package manager to re-resolve instead of refusing
        // (pnpm/yarn default to frozen-lockfile under CI, npm/bun need --force).
        packageManager === PackageManager.npm || packageManager === PackageManager.bun
          ? ['--force']
          : ['--no-frozen-lockfile'],
        {
          silent: true,
          packageManager,
          packageManagerVersion: resolvedVersion,
        },
      );
      // Route the install result through the shared helper (mirrors the full
      // migration path and is enforced by install-failure-guard.spec): a failed
      // install warns, appends to report.warnings, and flips process.exitCode
      // rather than being silently credited as a successful migration. Clear the
      // spinner first only on failure so the warning isn't interleaved with it;
      // on success handleInstallResult returns durationMs, so the credited
      // duration is unchanged.
      if (installSummary.status === 'failed') {
        clearMigrationProgress();
        npmReinstallPreparation?.restore();
      } else {
        npmReinstallPreparation?.commit();
      }
      finalInstallOk = installSummary.status !== 'failed';
      canFormatMigratedProject = finalInstallOk && canFormatMigratedProject;
      installDurationMs += handleInstallResult(
        installSummary,
        workspaceInfoOptional.rootDir,
        report,
      );
    }

    if (plan.selectedAgentTargetPaths && plan.selectedAgentTargetPaths.length > 0) {
      updateMigrationProgress('Writing agent instructions');
      await writeAgentInstructions({
        projectRoot: workspaceInfoOptional.rootDir,
        targetPaths: plan.selectedAgentTargetPaths,
        interactive: options.interactive,
        conflictDecisions: plan.agentConflictDecisions,
        silent: true,
      });
      didMigrate = true;
    }

    if (plan.selectedEditor) {
      updateMigrationProgress('Writing editor configs');
      await writeEditorConfigs({
        projectRoot: workspaceInfoOptional.rootDir,
        editorId: plan.selectedEditor,
        interactive: options.interactive,
        conflictDecisions: plan.editorConflictDecisions,
        silent: true,
        packageManager,
      });
      didMigrate = true;
    }

    // Check for Rolldown-incompatible config patterns (root + workspace packages)
    await checkWorkspaceRolldownCompatibility(
      workspaceInfoOptional,
      report,
      updateMigrationProgress,
    );

    if (
      didMigrate &&
      finalInstallOk &&
      canFormatMigratedProject &&
      canFormatWithOxfmt(prettierProject.hasDependency, prettierMigrated)
    ) {
      clearMigrationProgress();
      await formatMigratedProject(workspaceInfoOptional.rootDir, options.interactive, report, {
        excludedPaths: preExistingChangedPaths,
      });
    }

    if (didMigrate || report.warnings.length > 0) {
      clearMigrationProgress();
      showMigrationSummary({
        projectRoot: workspaceInfoOptional.rootDir,
        packageManager: packageManager ?? resolvedPackageManager,
        packageManagerVersion,
        installDurationMs,
        finalInstallOk,
        report,
        // The existing-Vite+ branch always updates an existing project, so the
        // headline reads "Updated <project> to Vite+ <version>".
        updatedExistingVitePlus: true,
        suggestFullMigration: skippedSetupCandidates,
      });
    } else {
      // Stop the "Checking config compatibility" spinner that
      // checkWorkspaceRolldownCompatibility left running before printing the
      // hint and outro; the sibling branches above already clear it, and
      // without this its timer line keeps re-rendering underneath them.
      clearMigrationProgress();
      if (skippedSetupCandidates) {
        log(FULL_MIGRATION_HINT);
      }
      prompts.outro(`This project is already using Vite+! ${accent('Happy coding!')}`);
    }
    return;
  }

  // Phase 1: Collect all user decisions upfront
  const plan = await collectMigrationPlan(
    workspaceInfoOptional.rootDir,
    workspaceInfoOptional.packageManager,
    workspaceInfoOptional.packageManagerVersion,
    options,
    workspaceInfoOptional.packages,
  );

  // Phase 2: Execute without prompts
  const result = await executeMigrationPlan(
    workspaceInfoOptional,
    plan,
    options.interactive,
    preExistingChangedPaths,
  );
  showMigrationSummary({
    projectRoot: workspaceInfoOptional.rootDir,
    packageManager: plan.packageManager,
    packageManagerVersion: result.packageManagerVersion,
    installDurationMs: result.installDurationMs,
    finalInstallOk: result.finalInstallOk,
    report: result.report,
  });
}

main().catch((err) => {
  prompts.log.error(err.message);
  console.error(err);
  process.exit(1);
});
