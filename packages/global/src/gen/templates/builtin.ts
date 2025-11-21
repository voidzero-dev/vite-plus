import assert from 'node:assert';
import path from 'node:path';

import type { WorkspaceInfo } from '../../types/index.ts';
import type { ExecutionResult } from '../command.ts';
import { setPackageName } from '../utils.ts';
import { executeGeneratorScaffold } from './generator.ts';
import { runRemoteTemplateCommand } from './remote.ts';
import { BuiltinTemplate, type BuiltinTemplateInfo } from './types.ts';

export async function executeBuiltinTemplate(
  workspaceInfo: WorkspaceInfo,
  templateInfo: BuiltinTemplateInfo,
): Promise<ExecutionResult> {
  assert(templateInfo.targetDir, 'targetDir is required');
  assert(templateInfo.packageName, 'packageName is required');

  if (templateInfo.command === BuiltinTemplate.generator) {
    return await executeGeneratorScaffold(workspaceInfo, templateInfo);
  }

  if (templateInfo.command === BuiltinTemplate.application) {
    templateInfo.command = 'create-vite@latest';
  }

  if (templateInfo.command === BuiltinTemplate.library) {
    templateInfo.command = 'create-tsdown@latest';
    if (!templateInfo.interactive) {
      // set default template for tsdown
      if (!templateInfo.args.find((arg) => arg.startsWith('--template') || arg.startsWith('-t'))) {
        templateInfo.args.push('--template', 'default');
      }
    }
  }

  templateInfo.args.unshift(templateInfo.targetDir);
  if (!templateInfo.interactive) {
    templateInfo.args.push('--no-interactive');
  }

  // Handle remote/external templates with fspy monitoring
  const result = await runRemoteTemplateCommand(
    workspaceInfo,
    workspaceInfo.rootDir,
    templateInfo,
    false,
  );
  const fullPath = path.join(workspaceInfo.rootDir, templateInfo.targetDir);
  // set package name in the project directory
  setPackageName(fullPath, templateInfo.packageName);

  return {
    ...result,
    projectDir: templateInfo.targetDir,
  };
}
