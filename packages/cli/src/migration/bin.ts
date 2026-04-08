import path from 'node:path';
import { styleText } from 'node:util';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import mri from 'mri';
import semver from 'semver';

import { vitePlusHeader } from '../../binding/index.js';
import {
  PackageManager,
  type WorkspaceInfo,
  type WorkspaceInfoOptional,
  type WorkspacePackage,
} from '../types/index.ts';
import {
  detectAgentConflicts,
  detectExistingAgentTargetPaths,
  selectAgentTargetPaths,
  writeAgentInstructions,
} from '../utils/agent.ts';
import { isForceOverrideMode } from '../utils/constants.ts';
import {
  detectEditorConflicts,
  type EditorId,
  selectEditor,
  writeEditorConfigs,
} from '../utils/editor.ts';
import { renderCliDoc } from '../utils/help.ts';
import { hasVitePlusDependency, readNearestPackageJson } from '../utils/package.ts';
import { displayRelative } from '../utils/path.ts';
import {
  cancelAndExit,
  defaultInteractive,
  downloadPackageManager,
  promptGitHooks,
  runViteInstall,
  selectPackageManager,
  upgradeYarn,
} from '../utils/prompts.ts';
import { accent, log, muted } from '../utils/terminal.ts';
import type { PackageDependencies } from '../utils/types.ts';
import { detectWorkspace } from '../utils/workspace.ts';
import {
  checkVitestVersion,
  checkViteVersion,
  detectEslintProject,
  detectNodeVersionManagerFile,
  detectPrettierProject,
  installGitHooks,
  mergeViteConfigFiles,
  migrateEslintToOxlint,
  migrateNodeVersionManagerFile,
  migratePrettierToOxfmt,
  preflightGitHooksSetup,
  rewriteMonorepo,
  rewriteStandaloneProject,
  type NodeVersionManagerDetection,
} from './migrator.ts';
import { createMigrationReport, type MigrationReport } from './report.ts';

function warnPackageLevelEslint() {
  prompts.log.warn(
    'ESLint detected in workspace packages but no root config found. Package-level ESLint must be migrated manually.',
  );
}

function warnLegacyEslintConfig(legacyConfigFile: string) {
  prompts.log.warn(
    `Legacy ESLint configuration detected (${legacyConfigFile}). ` +
      'Automatic migration to Oxlint requires ESLint v9+ with flat config format (eslint.config.*). ' +
      'Please upgrade to ESLint v9 first: https://eslint.org/docs/latest/use/migrate-to-9.0.0',
  );
}

async function confirmEslintMigration(interactive: boolean): Promise<boolean> {
  if (interactive) {
    const confirmed = await prompts.confirm({
      message:
        'Migrate ESLint rules to Oxlint using @oxlint/migrate?\n  ' +
        styleText(
          'gray',
          "Oxlint is Vite+'s built-in linter — significantly faster than ESLint with compatible rule support. @oxlint/migrate converts your existing rules automatically.",
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

async function promptEslintMigration(
  projectPath: string,
  interactive: boolean,
  packages?: WorkspacePackage[],
): Promise<boolean> {
  const eslintProject = detectEslintProject(projectPath, packages);
  if (eslintProject.hasDependency && !eslintProject.configFile && eslintProject.legacyConfigFile) {
    warnLegacyEslintConfig(eslintProject.legacyConfigFile);
    return false;
  }
  if (!eslintProject.hasDependency) {
    return false;
  }
  if (!eslintProject.configFile) {
    // Packages have eslint but no root config → warn and skip
    warnPackageLevelEslint();
    return false;
  }
  const confirmed = await confirmEslintMigration(interactive);
  if (!confirmed) {
    return false;
  }
  const ok = await migrateEslintToOxlint(
    projectPath,
    interactive,
    eslintProject.configFile,
    packages,
  );
  if (!ok) {
    cancelAndExit('ESLint migration failed. Fix the issue and re-run `vp migrate`.', 1);
  }
  return true;
}

function warnPackageLevelPrettier() {
  prompts.log.warn(
    'Prettier detected in workspace packages but no root config found. Package-level Prettier must be migrated manually.',
  );
}

async function confirmPrettierMigration(interactive: boolean): Promise<boolean> {
  if (interactive) {
    const confirmed = await prompts.confirm({
      message:
        'Migrate Prettier to Oxfmt?\n  ' +
        styleText(
          'gray',
          "Oxfmt is Vite+'s built-in formatter that replaces Prettier with faster performance. Your configuration will be converted automatically.",
        ),
      initialValue: true,
    });
    if (prompts.isCancel(confirmed)) {
      cancelAndExit();
    }
    return confirmed;
  }
  prompts.log.info('Prettier configuration detected. Auto-migrating to Oxfmt...');
  return true;
}

async function promptPrettierMigration(
  projectPath: string,
  interactive: boolean,
  packages?: WorkspacePackage[],
): Promise<boolean> {
  const prettierProject = detectPrettierProject(projectPath, packages);
  if (!prettierProject.hasDependency) {
    return false;
  }
  if (!prettierProject.configFile) {
    // Packages have prettier but no root config → warn and skip
    warnPackageLevelPrettier();
    return false;
  }
  const confirmed = await confirmPrettierMigration(interactive);
  if (!confirmed) {
    return false;
  }
  const ok = await migratePrettierToOxfmt(
    projectPath,
    interactive,
    prettierProject.configFile,
    packages,
  );
  if (!ok) {
    cancelAndExit('Prettier migration failed. Fix the issue and re-run `vp migrate`.', 1);
  }
  return true;
}

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

const helpMessage = renderCliDoc({
  usage: 'vp migrate [PATH] [OPTIONS]',
  summary:
    'Migrate standalone Vite, Vitest, Oxlint, Oxfmt, and Prettier projects to unified Vite+.',
  documentationUrl: 'https://viteplus.dev/guide/migrate',
  sections: [
    {
      title: 'Arguments',
      rows: [
        {
          label: 'PATH',
          description: 'Target directory to migrate (default: current directory)',
        },
      ],
    },
    {
      title: 'Options',
      rows: [
        {
          label: '--agent NAME',
          description: 'Write coding agent instructions to AGENTS.md, CLAUDE.md, etc.',
        },
        { label: '--no-agent', description: 'Skip writing coding agent instructions' },
        {
          label: '--editor NAME',
          description: 'Write editor config files into the project.',
        },
        { label: '--no-editor', description: 'Skip writing editor config files' },
        {
          label: '--hooks',
          description: 'Set up pre-commit hooks (default in non-interactive mode)',
        },
        { label: '--no-hooks', description: 'Skip pre-commit hooks setup' },
        {
          label: '--no-interactive',
          description: 'Run in non-interactive mode (skip prompts and use defaults)',
        },
        { label: '-h, --help', description: 'Show this help message' },
      ],
    },
    {
      title: 'Examples',
      lines: [
        `  ${muted('# Migrate current package')}`,
        `  ${accent('vp migrate')}`,
        '',
        `  ${muted('# Migrate specific directory')}`,
        `  ${accent('vp migrate my-app')}`,
        '',
        `  ${muted('# Non-interactive mode')}`,
        `  ${accent('vp migrate --no-interactive')}`,
      ],
    },
    {
      title: 'Migration Prompt',
      lines: [
        '  Give this to a coding agent when you want it to drive the migration:',
        '',
        '  Migrate this project to Vite+.',
        '  Vite+ replaces the current split tooling around runtime management, package',
        '  management, dev/build/test commands, linting, formatting, and packaging.',
        '  Run `vp help` and `vp help migrate` before making changes.',
        `  Use ${accent('vp migrate --no-interactive')} in the workspace root.`,
        '  Make sure the project is using Vite 8+ and Vitest 4.1+ before migrating.',
        '',
        '  After the migration:',
        '  - Confirm `vite` imports were rewritten to `vite-plus` where needed',
        '  - Confirm `vitest` imports were rewritten to `vite-plus/test` where needed',
        '  - Remove old `vite` and `vitest` dependencies only after those rewrites',
        '    are confirmed',
        '  - Move remaining tool-specific config into the appropriate blocks in',
        '    `vite.config.ts`',
        '',
        '  Command mapping:',
        '  - `vp run <script>` is the equivalent of `pnpm run <script>`',
        '  - `vp test` runs the built-in test command, while `vp run test` runs the',
        '    `test` script from `package.json`',
        '  - `vp install`, `vp add`, and `vp remove` delegate through the package',
        '    manager declared by `packageManager`',
        '  - `vp dev`, `vp build`, `vp preview`, `vp lint`, `vp fmt`, `vp check`,',
        '    and `vp pack` replace the corresponding standalone tools',
        '  - Prefer `vp check` for validation loops',
        '',
        '  Finally, verify the migration by running:',
        `  - ${accent('vp install')}`,
        `  - ${accent('vp check')}`,
        `  - ${accent('vp test')}`,
        `  - ${accent('vp build')}`,
        '',
        '  Summarize the migration at the end and report any manual follow-up still',
        '  required.',
      ],
    },
  ],
});

export interface MigrationOptions {
  interactive: boolean;
  help?: boolean;
  agent?: string | string[] | false;
  editor?: string | false;
  hooks?: boolean;
}

function parseArgs() {
  const args = process.argv.slice(3); // Skip 'node', 'vite', 'migrate'

  const parsed = mri<{
    help?: boolean;
    interactive?: boolean;
    agent?: string | string[] | false;
    editor?: string | false;
    hooks?: boolean;
  }>(args, {
    alias: { h: 'help' },
    boolean: ['help', 'interactive', 'hooks'],
    default: { interactive: defaultInteractive() },
  });
  const interactive = parsed.interactive;

  let projectPath = parsed._[0];
  if (projectPath) {
    projectPath = path.resolve(process.cwd(), projectPath);
  } else {
    projectPath = process.cwd();
  }

  return {
    projectPath,
    options: {
      interactive,
      help: parsed.help,
      agent: parsed.agent,
      editor: parsed.editor,
      hooks: parsed.hooks,
    } as MigrationOptions,
  };
}

interface MigrationPlan {
  packageManager: PackageManager;
  shouldSetupHooks: boolean;
  selectedAgentTargetPaths?: string[];
  agentConflictDecisions: Map<string, 'append' | 'skip'>;
  selectedEditor?: EditorId;
  editorConflictDecisions: Map<string, 'merge' | 'skip'>;
  migrateEslint: boolean;
  eslintConfigFile?: string;
  migratePrettier: boolean;
  prettierConfigFile?: string;
  migrateNodeVersionFile: boolean;
  nodeVersionDetection?: NodeVersionManagerDetection;
}

async function collectMigrationPlan(
  rootDir: string,
  detectedPackageManager: PackageManager | undefined,
  options: MigrationOptions,
  packages?: WorkspacePackage[],
): Promise<MigrationPlan> {
  // 1. Package manager selection
  const packageManager =
    detectedPackageManager ?? (await selectPackageManager(options.interactive, true));

  // 2. Git hooks (including preflight check)
  let shouldSetupHooks = await promptGitHooks(options);
  if (shouldSetupHooks) {
    const reason = preflightGitHooksSetup(rootDir);
    if (reason) {
      prompts.log.warn(`⚠ ${reason}`);
      shouldSetupHooks = false;
    }
  }

  // 3. Agent selection (auto-detect existing agent files to skip the selector prompt)
  const existingAgentTargetPaths =
    options.agent !== undefined || !options.interactive
      ? undefined
      : detectExistingAgentTargetPaths(rootDir);
  const selectedAgentTargetPaths =
    existingAgentTargetPaths !== undefined
      ? existingAgentTargetPaths
      : await selectAgentTargetPaths({
          interactive: options.interactive,
          agent: options.agent,
          onCancel: () => cancelAndExit(),
        });

  // 4. Agent conflict detection + prompting
  const agentConflicts = await detectAgentConflicts({
    projectRoot: rootDir,
    targetPaths: selectedAgentTargetPaths,
  });
  const agentConflictDecisions = new Map<string, 'append' | 'skip'>();
  for (const conflict of agentConflicts) {
    if (options.interactive) {
      const action = await prompts.select({
        message:
          `Agent instructions already exist at ${conflict.targetPath}.\n  ` +
          styleText(
            'gray',
            'The Vite+ template includes guidance on `vp` commands, the build pipeline, and project conventions.',
          ),
        options: [
          { label: 'Append', value: 'append' as const, hint: 'Add template content to the end' },
          { label: 'Skip', value: 'skip' as const, hint: 'Leave existing file unchanged' },
        ],
        initialValue: 'skip' as const,
      });
      if (prompts.isCancel(action)) {
        cancelAndExit();
      }
      agentConflictDecisions.set(conflict.targetPath, action);
    } else {
      agentConflictDecisions.set(conflict.targetPath, 'skip');
    }
  }

  // 5. Editor selection
  const selectedEditor = await selectEditor({
    interactive: options.interactive,
    editor: options.editor,
    onCancel: () => cancelAndExit(),
  });

  // 6. Editor conflict detection + prompting
  const editorConflicts = detectEditorConflicts({
    projectRoot: rootDir,
    editorId: selectedEditor,
  });
  const editorConflictDecisions = new Map<string, 'merge' | 'skip'>();
  for (const conflict of editorConflicts) {
    if (options.interactive) {
      const action = await prompts.select({
        message:
          `${conflict.displayPath} already exists.\n  ` +
          styleText(
            'gray',
            'Vite+ adds editor settings for the built-in linter and formatter. Merge adds new keys without overwriting existing ones.',
          ),
        options: [
          {
            label: 'Merge',
            value: 'merge' as const,
            hint: 'Merge new settings into existing file',
          },
          { label: 'Skip', value: 'skip' as const, hint: 'Leave existing file unchanged' },
        ],
        initialValue: 'skip' as const,
      });
      if (prompts.isCancel(action)) {
        cancelAndExit();
      }
      editorConflictDecisions.set(conflict.fileName, action);
    } else {
      editorConflictDecisions.set(conflict.fileName, 'merge');
    }
  }

  // 7. ESLint detection + prompt
  const eslintProject = detectEslintProject(rootDir, packages);
  let migrateEslint = false;
  if (eslintProject.hasDependency && !eslintProject.configFile && eslintProject.legacyConfigFile) {
    warnLegacyEslintConfig(eslintProject.legacyConfigFile);
  } else if (eslintProject.hasDependency && eslintProject.configFile) {
    migrateEslint = await confirmEslintMigration(options.interactive);
  } else if (eslintProject.hasDependency) {
    warnPackageLevelEslint();
  }

  // 9. Prettier detection + prompt
  const prettierProject = detectPrettierProject(rootDir, packages);
  let migratePrettier = false;
  if (prettierProject.hasDependency && prettierProject.configFile) {
    migratePrettier = await confirmPrettierMigration(options.interactive);
  } else if (prettierProject.hasDependency) {
    warnPackageLevelPrettier();
  }

  // 10. Node version manager file detection + prompt
  const nodeVersionDetection = detectNodeVersionManagerFile(rootDir);
  let migrateNodeVersionFile = false;
  if (nodeVersionDetection) {
    migrateNodeVersionFile = await confirmNodeVersionFileMigration(
      options.interactive,
      nodeVersionDetection,
    );
  }

  const plan: MigrationPlan = {
    packageManager,
    shouldSetupHooks,
    selectedAgentTargetPaths,
    agentConflictDecisions,
    selectedEditor,
    editorConflictDecisions,
    migrateEslint,
    eslintConfigFile: eslintProject.configFile,
    migratePrettier,
    prettierConfigFile: prettierProject.configFile,
    migrateNodeVersionFile,
    nodeVersionDetection,
  };

  return plan;
}

function formatDuration(durationMs: number) {
  if (durationMs < 1000) {
    return `${Math.max(1, durationMs)}ms`;
  }
  const durationSeconds = durationMs / 1000;
  if (durationSeconds < 10) {
    return `${durationSeconds.toFixed(1)}s`;
  }
  return `${Math.round(durationSeconds)}s`;
}

function showMigrationSummary(options: {
  projectRoot: string;
  packageManager: string;
  packageManagerVersion: string;
  installDurationMs: number;
  report: MigrationReport;
  updatedExistingVitePlus?: boolean;
}) {
  const {
    projectRoot,
    packageManager,
    packageManagerVersion,
    installDurationMs,
    report,
    updatedExistingVitePlus,
  } = options;
  const projectLabel = displayRelative(projectRoot) || '.';
  const configUpdates =
    report.createdViteConfigCount +
    report.mergedConfigCount +
    report.mergedStagedConfigCount +
    report.inlinedLintStagedConfigCount +
    report.removedConfigCount +
    report.tsdownImportCount;

  log(
    `${styleText('magenta', '◇')} ${updatedExistingVitePlus ? 'Updated' : 'Migrated'} ${accent(projectLabel)}${
      updatedExistingVitePlus ? '' : ' to Vite+'
    }`,
  );
  log(
    `${styleText('gray', '•')} Node ${process.versions.node}  ${packageManager} ${packageManagerVersion}`,
  );
  if (installDurationMs > 0) {
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
  if (report.eslintMigrated) {
    log(`${styleText('gray', '•')} ESLint rules migrated to Oxlint`);
  }
  if (report.prettierMigrated) {
    log(`${styleText('gray', '•')} Prettier migrated to Oxfmt`);
  }
  if (report.nodeVersionFileMigrated) {
    log(`${styleText('gray', '•')} Node version manager file migrated to .node-version`);
  }
  if (report.gitHooksConfigured) {
    log(`${styleText('gray', '•')} Git hooks configured`);
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

async function checkRolldownCompatibility(rootDir: string, report: MigrationReport): Promise<void> {
  try {
    const { resolveConfig } = await import('../index.js');
    const { checkManualChunksCompat } = await import('./compat.js');
    // Use 'runner' configLoader to avoid Rolldown bundling the config file,
    // which prints UNRESOLVED_IMPORT warnings that cannot be suppressed via logLevel.
    const config = await resolveConfig(
      { root: rootDir, logLevel: 'silent', configLoader: 'runner' },
      'build',
    );
    checkManualChunksCompat(config.build?.rollupOptions?.output, report);
  } catch {
    // Config resolution may fail — skip compatibility check silently
  }
}

async function executeMigrationPlan(
  workspaceInfoOptional: WorkspaceInfoOptional,
  plan: MigrationPlan,
  interactive: boolean,
): Promise<{
  installDurationMs: number;
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
  updateMigrationProgress('Preparing migration');
  const downloadResult = await downloadPackageManager(
    plan.packageManager,
    workspaceInfoOptional.packageManagerVersion,
    interactive,
    true,
  );
  const workspaceInfo: WorkspaceInfo = {
    ...workspaceInfoOptional,
    packageManager: plan.packageManager,
    downloadPackageManager: downloadResult,
  };

  // 2. Upgrade yarn if needed, or validate PM version
  if (
    plan.packageManager === PackageManager.yarn &&
    semver.satisfies(downloadResult.version, '>=4.0.0 <4.10.0')
  ) {
    updateMigrationProgress('Upgrading Yarn');
    await upgradeYarn(workspaceInfo.rootDir, interactive, true);
  } else if (
    plan.packageManager === PackageManager.pnpm &&
    semver.satisfies(downloadResult.version, '< 9.5.0')
  ) {
    failMigrationProgress('Migration failed');
    prompts.log.error(
      `✘ pnpm@${downloadResult.version} is not supported by auto migration, please upgrade pnpm to >=9.5.0 first`,
    );
    cancelAndExit('Vite+ cannot automatically migrate this project yet.', 1);
  } else if (
    plan.packageManager === PackageManager.npm &&
    semver.satisfies(downloadResult.version, '< 8.3.0')
  ) {
    failMigrationProgress('Migration failed');
    prompts.log.error(
      `✘ npm@${downloadResult.version} is not supported by auto migration, please upgrade npm to >=8.3.0 first`,
    );
    cancelAndExit('Vite+ cannot automatically migrate this project yet.', 1);
  }

  // 3. Migrate node version manager file → .node-version (independent of vite version)
  if (plan.migrateNodeVersionFile && plan.nodeVersionDetection) {
    updateMigrationProgress('Migrating node version file');
    migrateNodeVersionManagerFile(workspaceInfo.rootDir, plan.nodeVersionDetection, report);
  }

  // 4. Run vp install to ensure the project is ready
  updateMigrationProgress('Installing dependencies');
  const initialInstallSummary = await runViteInstall(
    workspaceInfo.rootDir,
    interactive,
    undefined,
    {
      silent: true,
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
  updateMigrationProgress('Checking config compatibility');
  await checkRolldownCompatibility(workspaceInfo.rootDir, report);
  if (workspaceInfo.packages) {
    for (const pkg of workspaceInfo.packages) {
      await checkRolldownCompatibility(path.join(workspaceInfo.rootDir, pkg.path), report);
    }
  }

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

  // 6. Skip staged migration when hooks are disabled (--no-hooks or preflight failed).
  // Without hooks, lint-staged config must stay in package.json so existing
  // .husky/pre-commit scripts that invoke `npx lint-staged` keep working.
  const skipStagedMigration = !plan.shouldSetupHooks;

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
    installGitHooks(workspaceInfo.rootDir, true, report);
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
  });

  // 11. Reinstall after migration
  // npm needs --force to re-resolve packages with newly added overrides,
  // otherwise the stale lockfile prevents override resolution.
  const installArgs =
    plan.packageManager === PackageManager.npm || plan.packageManager === PackageManager.bun
      ? ['--force']
      : undefined;
  updateMigrationProgress('Installing dependencies');
  const finalInstallSummary = await runViteInstall(
    workspaceInfo.rootDir,
    interactive,
    installArgs,
    { silent: true },
  );

  clearMigrationProgress();
  return {
    installDurationMs: initialInstallSummary.durationMs + finalInstallSummary.durationMs,
    packageManagerVersion: downloadResult.version,
    report,
  };
}

async function main() {
  const { projectPath, options } = parseArgs();

  if (options.help) {
    log(vitePlusHeader() + '\n');
    log(helpMessage);
    return;
  }

  log(`${vitePlusHeader()}\n`);

  const workspaceInfoOptional = await detectWorkspace(projectPath);
  const resolvedPackageManager = workspaceInfoOptional.packageManager ?? 'unknown';

  // Early return if already using Vite+ (only ESLint/hooks migration may be needed)
  // In force-override mode (file: tgz overrides), skip this check and run full migration
  const rootPkg = readNearestPackageJson<PackageDependencies>(workspaceInfoOptional.rootDir);
  if (hasVitePlusDependency(rootPkg) && !isForceOverrideMode()) {
    let didMigrate = false;
    let installDurationMs = 0;
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

    // Check if ESLint migration is needed
    const eslintMigrated = await promptEslintMigration(
      workspaceInfoOptional.rootDir,
      options.interactive,
      workspaceInfoOptional.packages,
    );

    // Check if Prettier migration is needed
    const prettierMigrated = await promptPrettierMigration(
      workspaceInfoOptional.rootDir,
      options.interactive,
      workspaceInfoOptional.packages,
    );

    // Check if node version manager file migration is needed
    const nodeVersionDetection = detectNodeVersionManagerFile(workspaceInfoOptional.rootDir);
    if (nodeVersionDetection) {
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

    // Merge configs and reinstall once if any tool migration happened
    if (eslintMigrated || prettierMigrated) {
      updateMigrationProgress('Rewriting configs');
      mergeViteConfigFiles(workspaceInfoOptional.rootDir, true, report);
      updateMigrationProgress('Installing dependencies');
      const installSummary = await runViteInstall(
        workspaceInfoOptional.rootDir,
        options.interactive,
        undefined,
        {
          silent: true,
        },
      );
      installDurationMs += installSummary.durationMs;
      didMigrate = true;
      report.eslintMigrated = eslintMigrated;
      report.prettierMigrated = prettierMigrated;
    }

    // Check if husky/lint-staged migration is needed
    const hasHooksToMigrate =
      rootPkg?.devDependencies?.husky ||
      rootPkg?.dependencies?.husky ||
      rootPkg?.devDependencies?.['lint-staged'] ||
      rootPkg?.dependencies?.['lint-staged'];
    if (hasHooksToMigrate) {
      const shouldSetupHooks = await promptGitHooks(options);
      if (shouldSetupHooks) {
        updateMigrationProgress('Configuring git hooks');
      }
      if (shouldSetupHooks && installGitHooks(workspaceInfoOptional.rootDir, true, report)) {
        didMigrate = true;
      }
    }

    // Check for Rolldown-incompatible config patterns (root + workspace packages)
    await checkRolldownCompatibility(workspaceInfoOptional.rootDir, report);
    if (workspaceInfoOptional.packages) {
      for (const pkg of workspaceInfoOptional.packages) {
        await checkRolldownCompatibility(
          path.join(workspaceInfoOptional.rootDir, pkg.path),
          report,
        );
      }
    }

    if (didMigrate || report.warnings.length > 0) {
      clearMigrationProgress();
      showMigrationSummary({
        projectRoot: workspaceInfoOptional.rootDir,
        packageManager: resolvedPackageManager,
        packageManagerVersion: workspaceInfoOptional.packageManagerVersion,
        installDurationMs,
        report,
        updatedExistingVitePlus: true,
      });
    } else {
      prompts.outro(`This project is already using Vite+! ${accent(`Happy coding!`)}`);
    }
    return;
  }

  // Phase 1: Collect all user decisions upfront
  const plan = await collectMigrationPlan(
    workspaceInfoOptional.rootDir,
    workspaceInfoOptional.packageManager,
    options,
    workspaceInfoOptional.packages,
  );

  // Phase 2: Execute without prompts
  const result = await executeMigrationPlan(workspaceInfoOptional, plan, options.interactive);
  showMigrationSummary({
    projectRoot: workspaceInfoOptional.rootDir,
    packageManager: plan.packageManager,
    packageManagerVersion: result.packageManagerVersion,
    installDurationMs: result.installDurationMs,
    report: result.report,
  });
}

main().catch((err) => {
  prompts.log.error(err.message);
  console.error(err);
  process.exit(1);
});
