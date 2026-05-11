import assert from 'node:assert';
import path from 'node:path';

import type { WorkspaceInfo } from '../../types/index.ts';
import type { ExecutionWithProjectDir } from '../command.ts';
import { copyDir, setPackageName } from '../utils.ts';
import type { BuiltinTemplateInfo } from './types.ts';

/**
 * Scaffold a bundled template by copying the pre-extracted directory at
 * `localPath` into `workspaceInfo.rootDir/targetDir`.
 */
export async function executeBundledTemplate(
  workspaceInfo: WorkspaceInfo,
  templateInfo: BuiltinTemplateInfo,
): Promise<ExecutionWithProjectDir> {
  assert(templateInfo.localPath, 'localPath is required for bundled templates');
  assert(templateInfo.targetDir, 'targetDir is required');
  assert(templateInfo.packageName, 'packageName is required');

  const destDir = path.join(workspaceInfo.rootDir, templateInfo.targetDir);
  try {
    copyDir(templateInfo.localPath, destDir);
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
      throw new Error(`bundled template directory not found: ${templateInfo.localPath}`, {
        cause: error,
      });
    }
    throw error;
  }

  try {
    setPackageName(destDir, templateInfo.packageName);
  } catch {
    // Template without a valid package.json — leave files as-is.
  }

  return { exitCode: 0, projectDir: templateInfo.targetDir };
}
