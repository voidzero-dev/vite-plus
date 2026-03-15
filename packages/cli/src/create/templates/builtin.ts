import assert from 'node:assert';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import colors from 'picocolors';

import type { WorkspaceInfo } from '../../types/index.js';
import type { ExecutionResult } from '../command.js';
import { setPackageName } from '../utils.js';
import { executeGeneratorScaffold } from './generator.js';
import { runRemoteTemplateCommand } from './remote.js';
import { BuiltinTemplate, type BuiltinTemplateInfo } from './types.js';

export async function executeBuiltinTemplate(
  workspaceInfo: WorkspaceInfo,
  templateInfo: BuiltinTemplateInfo,
  options?: { silent?: boolean },
): Promise<ExecutionResult> {
  assert(templateInfo.targetDir, 'targetDir is required');
  assert(templateInfo.packageName, 'packageName is required');

  if (templateInfo.command === BuiltinTemplate.generator) {
    return await executeGeneratorScaffold(workspaceInfo, templateInfo, options);
  }

  if (templateInfo.command === BuiltinTemplate.application) {
    templateInfo.command = 'create-vite@latest';
    if (!templateInfo.interactive) {
      templateInfo.args.push('--no-interactive');
    }
  } else if (templateInfo.command === BuiltinTemplate.library) {
    templateInfo.command = 'create-tsdown@latest';
    // create-tsdown doesn't support non-interactive mode;
    // add default template when running silently to prevent hang on piped stdin
    if (!templateInfo.interactive || options?.silent) {
      if (!templateInfo.args.some((arg) => arg.startsWith('--template') || arg.startsWith('-t'))) {
        templateInfo.args.push('--template', 'default');
      }
    }
  }

  templateInfo.args.unshift(templateInfo.targetDir);

  // Handle remote/external templates with fspy monitoring
  const result = await runRemoteTemplateCommand(
    workspaceInfo,
    workspaceInfo.rootDir,
    templateInfo,
    false,
    options?.silent ?? false,
  );
  if (result.exitCode !== 0) {
    prompts.log.error('Failed to create project');

    if (templateInfo.command === 'create-tsdown@latest') {
      prompts.log.info(colors.yellow('\nTroubleshooting:'));
      prompts.log.info(`  ${colors.gray('•')} Check your internet connection`);
      prompts.log.info(`  ${colors.gray('•')} Verify that api.github.com is accessible`);
      prompts.log.info(`  ${colors.gray('•')} Check your hosts file for incorrect DNS entries`);
      prompts.log.info(
        `  ${colors.gray('•')} Try running: ${colors.gray('pnpm dlx create-tsdown@latest --template default <project-name>')}`,
      );
    }

    return result;
  }

  const fullPath = path.join(workspaceInfo.rootDir, templateInfo.targetDir);
  // set package name in the project directory
  setPackageName(fullPath, templateInfo.packageName);

  return {
    ...result,
    projectDir: templateInfo.targetDir,
  };
}
