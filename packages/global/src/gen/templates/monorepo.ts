import assert from 'node:assert';
import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@clack/prompts';
import spawn from 'cross-spawn';

import { discoverTemplate } from '../discovery.ts';
import { migrateToVitePlus } from '../migration.ts';
import { type BuiltinTemplateInfo, type ExecutionResult, PackageManager, type WorkspaceInfo } from '../types.ts';
import {
  copyDir,
  editJsonFile,
  getScopeFromPackageName,
  renameFiles,
  setPackageManager,
  setPackageName,
  templatesDir,
} from '../utils.ts';

import { runRemoteTemplateCommand } from './remote.ts';

// Execute vite:monorepo - copy from templates/monorepo
export async function executeMonorepoTemplate(
  workspaceInfo: WorkspaceInfo,
  templateInfo: BuiltinTemplateInfo,
  interactive: boolean,
): Promise<ExecutionResult> {
  prompts.log.step('Creating vite+ monorepo...');
  assert(templateInfo.packageName, 'packageName is required');
  assert(templateInfo.targetDir, 'targetDir is required');

  workspaceInfo.monorepoScope = getScopeFromPackageName(templateInfo.packageName);
  const fullPath = path.join(workspaceInfo.rootDir, templateInfo.targetDir);

  // Copy template files
  const templateDir = path.join(templatesDir, 'monorepo');
  copyDir(templateDir, fullPath);
  renameFiles(fullPath);

  // set project name
  editJsonFile(path.join(fullPath, 'package.json'), (pkg) => {
    pkg.name = templateInfo.packageName;
    return pkg;
  });

  // Adjust package.json based on package manager
  if (workspaceInfo.packageManager === PackageManager.pnpm) {
    // remove workspaces field
    editJsonFile(path.join(fullPath, 'package.json'), (pkg) => {
      pkg.workspaces = undefined;
      // remove resolutions field
      pkg.resolutions = undefined;
      return pkg;
    });
    const yarnrcPath = path.join(fullPath, '.yarnrc.yml');
    if (fs.existsSync(yarnrcPath)) {
      fs.unlinkSync(yarnrcPath);
    }
  } else if (workspaceInfo.packageManager === PackageManager.yarn) {
    // remove pnpm field
    editJsonFile(path.join(fullPath, 'package.json'), (pkg) => {
      pkg.pnpm = undefined;
      return pkg;
    });
    const pnpmWorkspacePath = path.join(fullPath, 'pnpm-workspace.yaml');
    if (fs.existsSync(pnpmWorkspacePath)) {
      fs.unlinkSync(pnpmWorkspacePath);
    }
  } else {
    // npm
    // remove pnpm field
    editJsonFile(path.join(fullPath, 'package.json'), (pkg) => {
      pkg.pnpm = undefined;
      return pkg;
    });
    const pnpmWorkspacePath = path.join(fullPath, 'pnpm-workspace.yaml');
    if (fs.existsSync(pnpmWorkspacePath)) {
      fs.unlinkSync(pnpmWorkspacePath);
    }
    const yarnrcPath = path.join(fullPath, '.yarnrc.yml');
    if (fs.existsSync(yarnrcPath)) {
      fs.unlinkSync(yarnrcPath);
    }
  }

  setPackageManager(fullPath, workspaceInfo.downloadPackageManager);
  prompts.log.success('Monorepo template created');

  // Ask user to init git repository or auto-init if --no-interactive
  let initGit = true; // Default to yes
  if (interactive) {
    const selected = await prompts.confirm({
      message: `Initialize git repository:`,
      initialValue: true,
    });
    if (prompts.isCancel(selected)) {
      prompts.log.info('Operation cancelled. Skipping git initialization');
      initGit = false;
    } else {
      initGit = selected;
    }
  } else {
    prompts.log.info(`Initializing git repository (default: yes)`);
  }

  if (initGit) {
    const gitResult = spawn.sync('git', ['init'], {
      stdio: 'pipe',
      cwd: fullPath,
    });

    if (gitResult.status === 0) {
      prompts.log.success('Git repository initialized');
    } else {
      prompts.log.warn('Failed to initialize git repository');
      if (gitResult.stderr) {
        prompts.log.info(gitResult.stderr.toString());
      }
    }
  }

  // Automatically create a default application in apps/website
  prompts.log.step('Creating default application in apps/website...');

  const appDir = 'apps/website';
  const appTemplateInfo = discoverTemplate('create-vite@latest', [
    appDir,
    '--template',
    'vanilla-ts',
    '--no-interactive',
  ], workspaceInfo);
  const appResult = await runRemoteTemplateCommand(
    workspaceInfo,
    fullPath,
    appTemplateInfo,
  );

  if (appResult.exitCode !== 0) {
    prompts.log.error(`Failed to create default application: ${appResult.exitCode}`);
    return appResult;
  }

  const appPackageName = workspaceInfo.monorepoScope ? `${workspaceInfo.monorepoScope}/website` : 'website';
  setPackageName(path.join(fullPath, appDir), appPackageName);
  // Perform auto-migration on the created app
  await migrateToVitePlus(
    appDir,
    fullPath, // The monorepo directory
    true, // Always in monorepo context
  );

  // Automatically create a default library in packages/utils
  prompts.log.step('Creating default library in packages/utils...');
  const libraryDir = 'packages/utils';
  const libraryTemplateInfo = discoverTemplate('create-tsdown@latest', [
    libraryDir,
    '--template',
    'default',
    '--no-interactive',
  ], workspaceInfo);
  const libraryResult = await runRemoteTemplateCommand(
    workspaceInfo,
    fullPath,
    libraryTemplateInfo,
  );
  if (libraryResult.exitCode !== 0) {
    prompts.log.error(`Failed to create default library, exit code: ${libraryResult.exitCode}`);
    return libraryResult;
  }

  const libraryPackageName = workspaceInfo.monorepoScope ? `${workspaceInfo.monorepoScope}/utils` : 'utils';
  setPackageName(path.join(fullPath, libraryDir), libraryPackageName);
  // Perform auto-migration on the created library
  await migrateToVitePlus(
    libraryDir,
    fullPath, // The monorepo directory
    true, // Always in monorepo context
  );

  return { exitCode: 0, projectDir: templateInfo.targetDir };
}
