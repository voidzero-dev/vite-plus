import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@clack/prompts';
import { rewritePackageJsonScripts } from '@voidzero-dev/vite-plus/binding';
import colors from 'picocolors';

import type { WorkspaceInfo } from './types.ts';
import { editJsonFile, editOrCreateFile, pkgRoot, readJsonFile, templatesDir } from './utils.ts';

const rulesDir = path.join(pkgRoot, 'rules');
const { gray } = colors;
const viteTools = ['vite', 'vitest', 'oxlint', 'oxfmt', 'tsdown'];

// Detect standalone vite-related tools in project
export function detectStandaloneViteTools(projectDir: string, cwd: string): string[] {
  const packageJsonPath = path.join(cwd, projectDir, 'package.json');

  if (!fs.existsSync(packageJsonPath)) {
    return [];
  }

  const pkg = readJsonFile(packageJsonPath);
  const allDeps = {
    ...pkg.dependencies,
    ...pkg.devDependencies,
  };

  const detected: string[] = [];
  for (const tool of viteTools) {
    if (allDeps[tool]) {
      detected.push(tool);
    }
  }
  return detected;
}

export async function migratePackageJson(jsonFile: string): Promise<boolean> {
  const rulesYamlPath = path.join(rulesDir, 'package-json-scripts.yml');
  return await rewritePackageJsonScripts(jsonFile, rulesYamlPath);
}

// Migrate standalone vite tools to vite-plus
export async function migrateToVitePlus(
  projectDir: string,
  cwd: string,
  isMonorepo: boolean,
): Promise<void> {
  const packageJsonFile = path.join(projectDir, 'package.json');
  const packageJsonPath = path.join(cwd, packageJsonFile);
  editJsonFile(packageJsonPath, (pkg) => {
    // Track where vite was originally located (dependencies or devDependencies)
    const viteInDependencies = !!pkg.dependencies?.['vite'];

    // Remove standalone tools
    for (const tool of viteTools) {
      if (pkg.dependencies?.[tool]) {
        delete pkg.dependencies[tool];
      }
      if (pkg.devDependencies?.[tool]) {
        delete pkg.devDependencies[tool];
      }
    }

    // Add vite-plus to the same location where vite was originally
    if (isMonorepo) {
      // Use catalog for monorepo
      if (viteInDependencies) {
        pkg.dependencies['vite'] = 'catalog:';
      } else {
        if (!pkg.devDependencies) pkg.devDependencies = {};
        pkg.devDependencies['vite'] = 'catalog:';
      }
      if (pkg.devDependencies?.['typescript']) {
        pkg.devDependencies['typescript'] = 'catalog:';
      }
    } else {
      // Use npm alias for standalone
      // TODO: use stable version of vite-plus after released to npm
      if (viteInDependencies) {
        pkg.dependencies['vite'] = 'npm:@voidzero-dev/vite-plus@latest';
      } else {
        if (!pkg.devDependencies) pkg.devDependencies = {};
        pkg.devDependencies['vite'] = 'npm:@voidzero-dev/vite-plus@latest';
      }

      // set .npmrc to use vite-plus
      editOrCreateFile(path.join(cwd, projectDir, '.npmrc'), (content) => {
        const npmrc = fs.readFileSync(path.join(templatesDir, 'config/_npmrc'), 'utf-8');
        return content ? `${content.trimEnd()}\n${npmrc}` : npmrc;
      });
    }

    return pkg;
  });

  const updated = await migratePackageJson(packageJsonPath);
  if (updated) {
    prompts.log.info(`  ${gray('•')} Updated ${packageJsonFile} scripts`);
  }
}

// Perform auto-migration with prompts and feedback
export async function performAutoMigration(workspaceInfo: WorkspaceInfo, projectDir: string) {
  const standaloneTools = detectStandaloneViteTools(projectDir, workspaceInfo.rootDir);
  if (standaloneTools.length === 0) {
    return; // No migration needed
  }

  await migrateToVitePlus(projectDir, workspaceInfo.rootDir, workspaceInfo.isMonorepo);
  prompts.log.success(`Migrated to vite-plus ${gray('✓')}`);
  prompts.log.info(`  ${gray('•')} Removed: ${standaloneTools.join(', ')}`);
  prompts.log.info(
    `  ${gray('•')} Added: vite ${gray(
      workspaceInfo.isMonorepo ? '(catalog:)' : '(npm:@voidzero-dev/vite-plus@latest)',
    )}`,
  );
}
