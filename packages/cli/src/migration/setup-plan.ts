import { styleText } from 'node:util';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { PackageManager, type WorkspacePackage } from '../types/index.ts';
import {
  detectAgentConflicts,
  detectExistingAgentTargetPaths,
  selectAgentTargetPaths,
} from '../utils/agent.ts';
import { detectEditorConflicts, type EditorId, selectEditor } from '../utils/editor.ts';
import { cancelAndExit, promptGitHooks } from '../utils/prompts.ts';
import {
  confirmEslintMigration,
  detectEslintProject,
  detectIncompatibleEslintIntegration,
  preflightGitHooksSetup,
  warnIncompatibleEslintIntegration,
  warnLegacyEslintConfig,
  warnPackageLevelEslint,
} from './migrator.ts';
import type { MigrationOptions } from './options.ts';

export interface MigrationSetupPlan {
  shouldSetupHooks: boolean;
  selectedAgentTargetPaths?: string[];
  agentConflictDecisions: Map<string, 'append' | 'skip'>;
  selectedEditor?: EditorId;
  editorConflictDecisions: Map<string, 'merge' | 'skip'>;
  migrateEslint: boolean;
  eslintConfigFile?: string;
}

async function collectGitHooksDecision(
  rootDir: string,
  packageManager: PackageManager | undefined,
  options: MigrationOptions,
): Promise<boolean> {
  let shouldSetupHooks = await promptGitHooks(options);
  if (shouldSetupHooks) {
    const reason = preflightGitHooksSetup(rootDir, packageManager);
    if (reason) {
      prompts.log.warn(`⚠ ${reason}`);
      shouldSetupHooks = false;
    }
  }
  return shouldSetupHooks;
}

async function collectAgentInstructionPlan(
  rootDir: string,
  options: MigrationOptions,
): Promise<{
  selectedAgentTargetPaths?: string[];
  agentConflictDecisions: Map<string, 'append' | 'skip'>;
}> {
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

  return { selectedAgentTargetPaths, agentConflictDecisions };
}

async function collectEditorConfigPlan(
  rootDir: string,
  options: MigrationOptions,
): Promise<{
  selectedEditor?: EditorId;
  editorConflictDecisions: Map<string, 'merge' | 'skip'>;
}> {
  const selectedEditor = await selectEditor({
    interactive: options.interactive,
    editor: options.editor,
    onCancel: () => cancelAndExit(),
  });

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

  return { selectedEditor, editorConflictDecisions };
}

async function collectEslintMigrationDecision(
  rootDir: string,
  options: MigrationOptions,
  packages?: WorkspacePackage[],
): Promise<{ migrateEslint: boolean; eslintConfigFile?: string }> {
  const eslintProject = detectEslintProject(rootDir, packages);
  const incompatibleEslintIntegration = detectIncompatibleEslintIntegration(rootDir, packages);
  let migrateEslint = false;
  if (incompatibleEslintIntegration) {
    // e.g. `@nuxt/eslint` — skip the entire ESLint migration; preserve
    // the user's current ESLint setup and let them migrate by hand.
    warnIncompatibleEslintIntegration(incompatibleEslintIntegration);
  } else if (
    eslintProject.hasDependency &&
    !eslintProject.configFile &&
    eslintProject.legacyConfigFile
  ) {
    warnLegacyEslintConfig(eslintProject.legacyConfigFile);
  } else if (eslintProject.hasDependency && eslintProject.configFile) {
    migrateEslint = await confirmEslintMigration(options.interactive);
  } else if (eslintProject.hasDependency) {
    warnPackageLevelEslint();
  }

  return { migrateEslint, eslintConfigFile: eslintProject.configFile };
}

export async function collectMigrationSetupPlan(
  rootDir: string,
  packageManager: PackageManager | undefined,
  options: MigrationOptions,
  packages?: WorkspacePackage[],
  includeEslint = true,
): Promise<MigrationSetupPlan> {
  const shouldSetupHooks = await collectGitHooksDecision(rootDir, packageManager, options);
  const agentPlan = await collectAgentInstructionPlan(rootDir, options);
  const editorPlan = await collectEditorConfigPlan(rootDir, options);
  const eslintPlan = includeEslint
    ? await collectEslintMigrationDecision(rootDir, options, packages)
    : { migrateEslint: false };

  return {
    shouldSetupHooks,
    ...agentPlan,
    ...editorPlan,
    ...eslintPlan,
  };
}
