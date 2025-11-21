import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@clack/prompts';

import type { WorkspaceInfo } from '../../types/index.ts';
import type { ExecutionResult } from '../command.ts';
import { copyDir, editJsonFile, templatesDir } from '../utils.ts';
import type { BuiltinTemplateInfo } from './types.ts';

// Execute generator scaffold template
export async function executeGeneratorScaffold(
  workspaceInfo: WorkspaceInfo,
  templateInfo: BuiltinTemplateInfo,
): Promise<ExecutionResult> {
  prompts.log.step('Creating generator scaffold...');
  let description: string | undefined;
  if (templateInfo.interactive) {
    const defaultDescription = 'Generate new components for our monorepo';
    const descPrompt = await prompts.text({
      message: 'Description:',
      placeholder: defaultDescription,
      defaultValue: defaultDescription,
    });

    if (!prompts.isCancel(descPrompt)) {
      description = descPrompt;
    }
  }

  const fullPath = path.join(workspaceInfo.rootDir, templateInfo.targetDir);
  // Copy template files
  const templateDir = path.join(templatesDir, 'generator');
  copyDir(templateDir, fullPath);
  fs.chmodSync(path.join(fullPath, 'bin/index.ts'), '755');
  editJsonFile(path.join(fullPath, 'package.json'), (pkg) => {
    pkg.name = templateInfo.packageName;
    if (description) {
      pkg.description = description;
    }
    return pkg;
  });

  prompts.log.success('Generator scaffold created');
  return { exitCode: 0, projectDir: templateInfo.targetDir };
}
