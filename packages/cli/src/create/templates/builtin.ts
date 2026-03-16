import assert from 'node:assert';
import path from 'node:path';

import type { WorkspaceInfo } from '../../types/index.js';
import type { ExecutionResult } from '../command.js';
import { discoverTemplate } from '../discovery.js';
import { setPackageName } from '../utils.js';
import { executeGeneratorScaffold } from './generator.js';
import { runRemoteTemplateCommand } from './remote.js';
import { BuiltinTemplate, type BuiltinTemplateInfo, LibraryTemplateRepo } from './types.js';

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
    const fullPath = path.join(workspaceInfo.rootDir, templateInfo.targetDir);
    setPackageName(fullPath, templateInfo.packageName);
    return { ...result, projectDir: templateInfo.targetDir };
  }

  // Handle remote/external templates with fspy monitoring
  const result = await runRemoteTemplateCommand(
    workspaceInfo,
    workspaceInfo.rootDir,
    templateInfo,
    false,
    options?.silent ?? false,
  );
  const fullPath = path.join(workspaceInfo.rootDir, templateInfo.targetDir);
  // set package name in the project directory
  setPackageName(fullPath, templateInfo.packageName);

  return {
    ...result,
    projectDir: templateInfo.targetDir,
  };
}
