import assert from 'node:assert';
import path from 'node:path';

import type { WorkspaceInfo } from '../../types/index.ts';
import type { ExecutionResult } from '../command.ts';
import { copyDir, setPackageName } from '../utils.ts';
import type { BuiltinTemplateInfo } from './types.ts';

/**
 * Scaffold a bundled subdirectory template — a `./templates/<name>` entry
 * from an `@org/create` manifest. The tarball has already been extracted
 * to `templateInfo.localPath` by `org-tarball.ts`; this function simply
 * copies that directory into the workspace and normalizes the
 * `package.json` name.
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

  // Best-effort: update the scaffolded package.json to use the user-chosen
  // name. Templates without a package.json (rare but legal) are left
  // untouched.
  try {
    setPackageName(destDir, templateInfo.packageName);
  } catch {
    // The scaffolded tree doesn't have a package.json (or it isn't valid
    // JSON). The user will see the raw file contents; we don't need to
    // fail the scaffold.
  }

  return {
    exitCode: 0,
    projectDir: templateInfo.targetDir,
  };
}
