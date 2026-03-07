import path from 'node:path';
import { styleText } from 'node:util';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import mri from 'mri';
import colors from 'picocolors';
import semver from 'semver';

import { vitePlusHeader } from '../../binding/index.js';
import {
  PackageManager,
  type WorkspaceInfo,
  type WorkspaceInfoOptional,
  type WorkspacePackage,
} from '../types/index.js';
import {
  detectAgentConflicts,
  selectAgentTargetPaths,
  writeAgentInstructions,
} from '../utils/agent.js';
import {
  detectEditorConflicts,
  EDITORS,
  type EditorId,
  selectEditor,
  writeEditorConfigs,
} from '../utils/editor.js';
import { renderCliDoc } from '../utils/help.js';
import { hasVitePlusDependency, readNearestPackageJson } from '../utils/package.js';
import {
  cancelAndExit,
  defaultInteractive,
  downloadPackageManager,
  promptGitHooks,
  runViteInstall,
  selectPackageManager,
  upgradeYarn,
} from '../utils/prompts.js';
import { accent, log, muted } from '../utils/terminal.js';
import type { PackageDependencies } from '../utils/types.js';
import { detectWorkspace } from '../utils/workspace.js';
import {
  checkVitestVersion,
  checkViteVersion,
  detectEslintProject,
  installGitHooks,
  mergeViteConfigFiles,
  migrateEslintToOxlint,
  preflightGitHooksSetup,
  rewriteMonorepo,
  rewriteStandaloneProject,
} from './migrator.js';

const { green } = colors;

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
      message: 'Migrate ESLint rules to Oxlint using @oxlint/migrate?',
      initialValue: true,
    });
    if (prompts.isCancel(confirmed)) {
      cancelAndExit();
    }
    return !!confirmed;
  }
  prompts.log.info('ESLint configuration detected. Auto-migrating to Oxlint...');
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

const helpMessage = renderCliDoc({
  usage: 'vp migrate [PATH] [OPTIONS]',
  summary: 'Migrate standalone Vite, Vitest, Oxlint, and Oxfmt projects to unified Vite+.',
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
          description:
            'Write agent instructions file into the project (e.g. chatgpt, claude, opencode).',
        },
        { label: '--no-agent', description: 'Skip writing agent instructions file' },
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
}

async function collectMigrationPlan(
  rootDir: string,
  detectedPackageManager: PackageManager | undefined,
  options: MigrationOptions,
  packages?: WorkspacePackage[],
): Promise<MigrationPlan> {
  // 1. Confirm migration
  if (options.interactive) {
    const approved = await prompts.confirm({
      message: 'Migrate this project to Vite+?',
      initialValue: true,
    });
    if (prompts.isCancel(approved) || !approved) {
      cancelAndExit('Migration cancelled');
    }
  }

  // 2. Package manager selection
  const packageManager =
    detectedPackageManager ?? (await selectPackageManager(options.interactive));

  // 3. Git hooks (including preflight check)
  let shouldSetupHooks = await promptGitHooks(options);
  if (shouldSetupHooks) {
    const reason = preflightGitHooksSetup(rootDir);
    if (reason) {
      prompts.log.warn(`⚠ ${reason}`);
      shouldSetupHooks = false;
    }
  }

  // 4. Agent selection
  const selectedAgentTargetPaths = await selectAgentTargetPaths({
    interactive: options.interactive,
    agent: options.agent,
    onCancel: () => cancelAndExit(),
  });

  // 5. Agent conflict detection + prompting
  const agentConflicts = await detectAgentConflicts({
    projectRoot: rootDir,
    targetPaths: selectedAgentTargetPaths,
  });
  const agentConflictDecisions = new Map<string, 'append' | 'skip'>();
  for (const conflict of agentConflicts) {
    if (options.interactive) {
      const action = await prompts.select({
        message: `Agent instructions already exist at ${conflict.targetPath}.`,
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

  // 6. Editor selection
  const selectedEditor = await selectEditor({
    interactive: options.interactive,
    editor: options.editor,
    onCancel: () => cancelAndExit(),
  });

  // 7. Editor conflict detection + prompting
  const editorConflicts = detectEditorConflicts({
    projectRoot: rootDir,
    editorId: selectedEditor,
  });
  const editorConflictDecisions = new Map<string, 'merge' | 'skip'>();
  for (const conflict of editorConflicts) {
    if (options.interactive) {
      const action = await prompts.select({
        message: `${conflict.displayPath} already exists.`,
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

  // 8. ESLint detection + prompt
  const eslintProject = detectEslintProject(rootDir, packages);
  let migrateEslint = false;
  if (eslintProject.hasDependency && !eslintProject.configFile && eslintProject.legacyConfigFile) {
    warnLegacyEslintConfig(eslintProject.legacyConfigFile);
  } else if (eslintProject.hasDependency && eslintProject.configFile) {
    migrateEslint = await confirmEslintMigration(options.interactive);
  } else if (eslintProject.hasDependency) {
    warnPackageLevelEslint();
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
  };

  // 9. Display migration plan summary
  if (options.interactive) {
    displayMigrationSummary(plan);
  }

  return plan;
}

function displayMigrationSummary(plan: MigrationPlan) {
  const lines: string[] = [
    `- Install ${plan.packageManager} and dependencies`,
    '- Rewrite configs and dependencies for Vite+',
  ];

  if (plan.migrateEslint) {
    lines.push('- Migrate ESLint rules to Oxlint');
  }

  if (plan.shouldSetupHooks) {
    lines.push('- Set up pre-commit hooks');
  }

  if (plan.selectedAgentTargetPaths && plan.selectedAgentTargetPaths.length > 0) {
    const parts = plan.selectedAgentTargetPaths.map((tp) => {
      const action = plan.agentConflictDecisions.get(tp);
      return action ? `${tp}, ${action}` : tp;
    });
    lines.push(`- Write agent instructions (${parts.join('; ')})`);
  }

  if (plan.selectedEditor) {
    const editorConfig = EDITORS.find((e) => e.id === plan.selectedEditor);
    const targetDir = editorConfig?.targetDir ?? plan.selectedEditor;
    const decisions = [...plan.editorConflictDecisions.values()];
    const uniqueActions = [...new Set(decisions)];
    const actionStr = uniqueActions.length > 0 ? `, ${uniqueActions.join('/')}` : '';
    lines.push(`- Write editor config (${targetDir}/${actionStr})`);
  }

  prompts.log.info([styleText('bold', 'Migration plan:'), ...lines].join('\n') + '\n');
}

async function executeMigrationPlan(
  workspaceInfoOptional: WorkspaceInfoOptional,
  plan: MigrationPlan,
  interactive: boolean,
) {
  // 1. Download package manager + version validation
  const downloadResult = await downloadPackageManager(
    plan.packageManager,
    workspaceInfoOptional.packageManagerVersion,
    interactive,
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
    await upgradeYarn(workspaceInfo.rootDir, interactive);
  } else if (
    plan.packageManager === PackageManager.pnpm &&
    semver.satisfies(downloadResult.version, '< 9.5.0')
  ) {
    prompts.log.error(
      `✘ pnpm@${downloadResult.version} is not supported by auto migration, please upgrade pnpm to >=9.5.0 first`,
    );
    cancelAndExit('Vite+ cannot automatically migrate this project yet.', 1);
  } else if (
    plan.packageManager === PackageManager.npm &&
    semver.satisfies(downloadResult.version, '< 8.3.0')
  ) {
    prompts.log.error(
      `✘ npm@${downloadResult.version} is not supported by auto migration, please upgrade npm to >=8.3.0 first`,
    );
    cancelAndExit('Vite+ cannot automatically migrate this project yet.', 1);
  }

  // 3. Run vp install to ensure the project is ready
  await runViteInstall(workspaceInfo.rootDir, interactive);

  // 4. Check vite and vitest version is supported by migration
  const isViteSupported = checkViteVersion(workspaceInfo.rootDir);
  const isVitestSupported = checkVitestVersion(workspaceInfo.rootDir);
  if (!isViteSupported || !isVitestSupported) {
    cancelAndExit('Vite+ cannot automatically migrate this project yet.', 1);
  }

  // 5. ESLint → Oxlint migration (before main rewrite so .oxlintrc.json gets picked up)
  if (plan.migrateEslint) {
    const eslintOk = await migrateEslintToOxlint(
      workspaceInfo.rootDir,
      interactive,
      plan.eslintConfigFile,
      workspaceInfo.packages,
    );
    if (!eslintOk) {
      cancelAndExit('ESLint migration failed. Fix the issue and re-run `vp migrate`.', 1);
    }
  }

  // 6. Skip staged migration when hooks are disabled (--no-hooks or preflight failed).
  // Without hooks, lint-staged config must stay in package.json so existing
  // .husky/pre-commit scripts that invoke `npx lint-staged` keep working.
  const skipStagedMigration = !plan.shouldSetupHooks;

  // 7. Rewrite configs
  if (workspaceInfo.isMonorepo) {
    rewriteMonorepo(workspaceInfo, skipStagedMigration);
  } else {
    rewriteStandaloneProject(workspaceInfo.rootDir, workspaceInfo, skipStagedMigration);
  }

  // 8. Install git hooks
  if (plan.shouldSetupHooks) {
    installGitHooks(workspaceInfo.rootDir);
  }

  // 9. Write agent instructions (using pre-resolved decisions)
  await writeAgentInstructions({
    projectRoot: workspaceInfo.rootDir,
    targetPaths: plan.selectedAgentTargetPaths,
    interactive,
    conflictDecisions: plan.agentConflictDecisions,
  });

  // 10. Write editor configs (using pre-resolved decisions)
  await writeEditorConfigs({
    projectRoot: workspaceInfo.rootDir,
    editorId: plan.selectedEditor,
    interactive,
    conflictDecisions: plan.editorConflictDecisions,
  });

  // 11. Reinstall after migration
  // npm needs --force to re-resolve packages with newly added overrides,
  // otherwise the stale lockfile prevents override resolution.
  const installArgs = plan.packageManager === PackageManager.npm ? ['--force'] : undefined;
  await runViteInstall(workspaceInfo.rootDir, interactive, installArgs);
  prompts.outro(green('✔ Migration completed!'));
}

async function main() {
  const { projectPath, options } = parseArgs();

  if (options.help) {
    log(vitePlusHeader() + '\n');
    log(helpMessage);
    return;
  }

  prompts.intro(vitePlusHeader());

  const workspaceInfoOptional = await detectWorkspace(projectPath);

  // Early return if already using Vite+ (only ESLint/hooks migration may be needed)
  const rootPkg = readNearestPackageJson<PackageDependencies>(workspaceInfoOptional.rootDir);
  if (hasVitePlusDependency(rootPkg)) {
    let didMigrate = false;

    // Check if ESLint migration is needed
    const eslintMigrated = await promptEslintMigration(
      workspaceInfoOptional.rootDir,
      options.interactive,
      workspaceInfoOptional.packages,
    );
    if (eslintMigrated) {
      mergeViteConfigFiles(workspaceInfoOptional.rootDir);
      await runViteInstall(workspaceInfoOptional.rootDir, options.interactive);
      didMigrate = true;
    }

    // Check if husky/lint-staged migration is needed
    const hasHooksToMigrate =
      rootPkg?.devDependencies?.husky ||
      rootPkg?.dependencies?.husky ||
      rootPkg?.devDependencies?.['lint-staged'] ||
      rootPkg?.dependencies?.['lint-staged'];
    if (hasHooksToMigrate) {
      const shouldSetupHooks = await promptGitHooks(options);
      if (shouldSetupHooks && installGitHooks(workspaceInfoOptional.rootDir)) {
        didMigrate = true;
      }
    }

    if (didMigrate) {
      prompts.outro(green('✔ Migration completed!'));
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
  await executeMigrationPlan(workspaceInfoOptional, plan, options.interactive);
}

main().catch((err) => {
  prompts.log.error(err.message);
  console.error(err);
  process.exit(1);
});
