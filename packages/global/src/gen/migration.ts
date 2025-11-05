import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@clack/prompts';
import colors from 'picocolors';

import type { WorkspaceInfo } from './types.ts';
import { editJsonFile, editOrCreateFile, readJsonFile, templatesDir } from './utils.ts';

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

// Migrate standalone vite tools to vite-plus
export function migrateToVitePlus(projectDir: string, cwd: string, isMonorepo: boolean): void {
  const packageJsonPath = path.join(cwd, projectDir, 'package.json');
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

    // Update scripts if needed
    // TODO: use ast-grep to update scripts
    if (pkg.scripts) {
      // Update common script patterns
      if (pkg.scripts.dev === 'vite') {
        pkg.scripts.dev = 'vite dev';
      }
      if (pkg.scripts.dev === 'tsdown --watch') {
        pkg.scripts.dev = 'vite lib --watch';
      }
      if (pkg.scripts.build === 'tsdown') {
        pkg.scripts.build = 'vite lib';
      }
      if (pkg.scripts.test === 'vitest' || pkg.scripts.test === 'vitest run') {
        pkg.scripts.test = 'vite test';
      }
      if (pkg.scripts.lint === 'oxlint') {
        pkg.scripts.lint = 'vite lint';
      }
      if (pkg.scripts.format === 'oxfmt') {
        pkg.scripts.format = 'vite fmt';
      }
    }
    return pkg;
  });
}

// Perform auto-migration with prompts and feedback
export async function performAutoMigration(
  workspaceInfo: WorkspaceInfo,
  projectDir: string,
) {
  const standaloneTools = detectStandaloneViteTools(projectDir, workspaceInfo.rootDir);
  if (standaloneTools.length === 0) {
    return; // No migration needed
  }

  migrateToVitePlus(projectDir, workspaceInfo.rootDir, workspaceInfo.isMonorepo);
  prompts.log.success(`Migrated to vite-plus ${gray('✓')}`);
  prompts.log.info(`  ${gray('•')} Removed: ${standaloneTools.join(', ')}`);
  prompts.log.info(
    `  ${gray('•')} Added: vite ${
      gray(workspaceInfo.isMonorepo ? '(catalog:)' : '(npm:@voidzero-dev/vite-plus@latest)')
    }`,
  );
}
