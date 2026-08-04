import assert from 'node:assert';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import colors from 'picocolors';

import type { WorkspaceInfo } from '../../types/index.ts';
import type { ExecutionWithProjectDir } from '../command.ts';
import { discoverTemplate } from '../discovery.ts';
import { setPackageName } from '../utils.ts';
import { executeGeneratorScaffold } from './generator.ts';
import { runRemoteTemplateCommand } from './remote.ts';
import { BuiltinTemplate, type BuiltinTemplateInfo, LibraryTemplateRepo } from './types.ts';

export async function executeBuiltinTemplate(
  workspaceInfo: WorkspaceInfo,
  templateInfo: BuiltinTemplateInfo,
  options?: { silent?: boolean },
): Promise<ExecutionWithProjectDir> {
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
    templateInfo.args.unshift(templateInfo.targetDir);
  } else if (templateInfo.command === BuiltinTemplate.library) {
    // Use degit to download the template directly from GitHub
    const libraryTemplateInfo = discoverTemplate(
      LibraryTemplateRepo,
      [templateInfo.targetDir],
      workspaceInfo,
    );
    const result = await runRemoteTemplateCommand(
      workspaceInfo,
      workspaceInfo.rootDir,
      libraryTemplateInfo,
      false,
      options?.silent ?? false,
    );
    if (result.exitCode !== 0) {
      return { exitCode: result.exitCode };
    }
    const fullPath = path.join(workspaceInfo.rootDir, templateInfo.targetDir);
    setPackageName(fullPath, templateInfo.packageName);
    return { ...result, projectDir: templateInfo.targetDir };
  }

  // Unknown vite: template (e.g. vite:test) — application was already rewritten to create-vite@latest
  if (templateInfo.command.startsWith('vite:')) {
    if (!options?.silent) {
      prompts.log.error(
        `Unknown builtin template "${templateInfo.command}". Run ${colors.yellow('vp create --list')} to see available templates.`,
      );
    }
    return { exitCode: 1 };
  }

  // Handle remote/external templates with fspy monitoring
  const result = await runRemoteTemplateCommand(
    workspaceInfo,
    workspaceInfo.rootDir,
    templateInfo,
    false,
    options?.silent ?? false,
  );
  if (result.exitCode !== 0) {
    return { exitCode: result.exitCode };
  }
  const fullPath = path.join(workspaceInfo.rootDir, templateInfo.targetDir);
  // set package name in the project directory
  setPackageName(fullPath, templateInfo.packageName);

  return {
    ...result,
    projectDir: templateInfo.targetDir,
  };
}
