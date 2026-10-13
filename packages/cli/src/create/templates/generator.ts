import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { createCatalogDependencyResolver, supportsCatalog } from '../../migration/migrator.ts';
import type { WorkspaceInfo } from '../../types/index.ts';
import { editJsonFile } from '../../utils/json.ts';
import { templatesDir } from '../../utils/path.ts';
import { readYamlFile } from '../../utils/yaml.ts';
import type { ExecutionWithProjectDir } from '../command.ts';
import { copyDir } from '../utils.ts';
import type { BuiltinTemplateInfo } from './types.ts';

// Execute generator scaffold template
export async function executeGeneratorScaffold(
  workspaceInfo: WorkspaceInfo,
  templateInfo: BuiltinTemplateInfo,
  options?: { silent?: boolean },
): Promise<ExecutionWithProjectDir> {
  if (!options?.silent) {
    prompts.log.step('Creating generator scaffold...');
  }
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
  const catalogResolver = supportsCatalog(
    workspaceInfo.packageManager,
    workspaceInfo.downloadPackageManager.version,
    workspaceInfo.isMonorepo,
  )
    ? createCatalogDependencyResolver(workspaceInfo.rootDir, workspaceInfo.packageManager)
    : undefined;
  const { catalog } = readYamlFile(path.join(templatesDir, 'monorepo/pnpm-workspace.yaml')) as {
    catalog: Record<string, string>;
  };
  editJsonFile(path.join(fullPath, 'package.json'), (pkg) => {
    pkg.name = templateInfo.packageName;
    // Keep catalog references only when the target workspace can resolve them.
    // Missing entries use the same ranges as the bundled monorepo template.
    const devDependencies = pkg.devDependencies as Record<string, string>;
    for (const [name, spec] of Object.entries(devDependencies)) {
      if (spec === 'catalog:' && catalogResolver?.(spec, name) === undefined && catalog[name]) {
        devDependencies[name] = catalog[name];
      }
    }
    if (description) {
      pkg.description = description;
    }
    return pkg;
  });

  if (!options?.silent) {
    prompts.log.success('Generator scaffold created');
  }
  return { exitCode: 0, projectDir: templateInfo.targetDir };
}
