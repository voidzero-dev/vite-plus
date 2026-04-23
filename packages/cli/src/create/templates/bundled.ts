import assert from 'node:assert';
import path from 'node:path';

import type { WorkspaceInfo } from '../../types/index.ts';
import type { ExecutionResult } from '../command.ts';
import { copyDir, setPackageName } from '../utils.ts';
import type { BuiltinTemplateInfo } from './types.ts';

/**
 * Scaffold a bundled template by copying the pre-extracted directory at
 * `localPath` into `workspaceInfo.rootDir/targetDir`.
 */
export async function executeBundledTemplate(
  workspaceInfo: WorkspaceInfo,
  templateInfo: BuiltinTemplateInfo,
): Promise<ExecutionResult> {
  assert(templateInfo.localPath, 'localPath is required for bundled templates');
  assert(templateInfo.targetDir, 'targetDir is required');
  assert(templateInfo.packageName, 'packageName is required');

  const destDir = path.join(workspaceInfo.rootDir, templateInfo.targetDir);
  copyDir(templateInfo.localPath, destDir);

  try {
    setPackageName(destDir, templateInfo.packageName);
  } catch {
    // Template without a valid package.json — leave files as-is.
  }

  return { exitCode: 0, projectDir: templateInfo.targetDir };
}
