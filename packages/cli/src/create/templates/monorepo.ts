import assert from 'node:assert';
import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { rewriteMonorepoProject } from '../../migration/migrator.ts';
import { PackageManager, type WorkspaceInfo } from '../../types/index.ts';
import { editJsonFile } from '../../utils/json.ts';
import { templatesDir } from '../../utils/path.ts';
import { editYamlFile, readYamlFile } from '../../utils/yaml.ts';
import type { ExecutionWithProjectDir } from '../command.ts';
import { discoverTemplate } from '../discovery.ts';
import { copyDir, formatDisplayTargetDir, renameFiles, setPackageName } from '../utils.ts';
import { runRemoteTemplateCommand } from './remote.ts';
import { type BuiltinTemplateInfo, LibraryTemplateRepo } from './types.ts';

export const InitialMonorepoAppDir = 'apps/website';

// Execute vite:monorepo - copy from templates/monorepo
export async function executeMonorepoTemplate(
  workspaceInfo: WorkspaceInfo,
  templateInfo: BuiltinTemplateInfo,
  options?: { silent?: boolean },
): Promise<ExecutionWithProjectDir> {
  assert(templateInfo.packageName, 'packageName is required');
  assert(templateInfo.targetDir, 'targetDir is required');

  workspaceInfo.monorepoScope = getScopeFromPackageName(templateInfo.packageName);
  const fullPath = path.join(workspaceInfo.rootDir, templateInfo.targetDir);

  if (!options?.silent) {
    prompts.log.info(`Target directory: ${formatDisplayTargetDir(templateInfo.targetDir)}`);
    prompts.log.step('Creating Vite+ monorepo...');
  }

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
    // npm or bun: both use package.json workspaces field
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

  if (!options?.silent) {
    prompts.log.success('Monorepo template created');
  }

  // Automatically create a default application in apps/website
  if (!options?.silent) {
    prompts.log.step('Creating default application in apps/website...');
  }

  const appTemplateInfo = discoverTemplate(
    'create-vite@latest',
    [InitialMonorepoAppDir, '--template', 'vanilla-ts', '--no-interactive'],
    workspaceInfo,
  );
  const appResult = await runRemoteTemplateCommand(
    workspaceInfo,
    fullPath,
    appTemplateInfo,
    false,
    options?.silent ?? false,
  );

  if (appResult.exitCode !== 0) {
    prompts.log.error(`Failed to create default application: ${appResult.exitCode}`);
    return appResult;
  }

  const appPackageName = workspaceInfo.monorepoScope
    ? `${workspaceInfo.monorepoScope}/website`
    : 'website';
  const appProjectPath = path.join(fullPath, InitialMonorepoAppDir);
  setPackageName(appProjectPath, appPackageName);
  // Perform auto-migration on the created app
  rewriteMonorepoProject(
    appProjectPath,
    workspaceInfo.packageManager,
    undefined,
    options?.silent ?? false,
  );
  // Drop the migrator's aliased vite/vitest devDeps for npm/yarn/bun (pnpm
  // keeps them so its workspace override stays effective; see the helper).
  dropAliasedRuntimeDevDeps(appProjectPath, workspaceInfo.packageManager);

  // Automatically create a default library in packages/utils
  if (!options?.silent) {
    prompts.log.step('Creating default library in packages/utils...');
  }
  const libraryDir = 'packages/utils';
  const libraryTemplateInfo = discoverTemplate(LibraryTemplateRepo, [libraryDir], workspaceInfo);
  const libraryResult = await runRemoteTemplateCommand(
    workspaceInfo,
    fullPath,
    libraryTemplateInfo,
    false,
    options?.silent ?? false,
  );
  if (libraryResult.exitCode !== 0) {
    prompts.log.error(`Failed to create default library, exit code: ${libraryResult.exitCode}`);
    return libraryResult;
  }

  const libraryPackageName = workspaceInfo.monorepoScope
    ? `${workspaceInfo.monorepoScope}/utils`
    : 'utils';
  const libraryProjectPath = path.join(fullPath, libraryDir);
  setPackageName(libraryProjectPath, libraryPackageName);
  // Perform auto-migration on the created library
  rewriteMonorepoProject(
    libraryProjectPath,
    workspaceInfo.packageManager,
    undefined,
    options?.silent ?? false,
  );

  alignMonorepoTypeScriptVersion(fullPath, appProjectPath, libraryProjectPath);

  return { exitCode: 0, projectDir: templateInfo.targetDir };
}

/**
 * Keep every scaffolded workspace member on the same TypeScript version.
 *
 * The app and library come from independently updated remote templates. If
 * their TypeScript ranges resolve to different versions, `typescript` — an
 * optional peer of vite-plus — resolves differently per member, so package
 * managers either create separate vite-plus peer instances (pnpm) or hoist a
 * compiler the other workspace member did not ask for (npm/Yarn). The library
 * template is the compatibility baseline because its declaration build may
 * require a newer compiler, so the app adopts its TypeScript range instead of
 * silently downgrading the library. The workspace catalog entry follows the
 * same baseline so members that reference `typescript: "catalog:"` (e.g. a
 * later `vite:generator` scaffold) stay on the workspace's compiler.
 */
export function alignMonorepoTypeScriptVersion(
  workspaceRootPath: string,
  appProjectPath: string,
  libraryProjectPath: string,
): void {
  const libraryPackage = JSON.parse(
    fs.readFileSync(path.join(libraryProjectPath, 'package.json'), 'utf8'),
  ) as {
    devDependencies?: Record<string, string>;
  };
  const typescriptVersion = libraryPackage.devDependencies?.typescript;
  if (!typescriptVersion) {
    return;
  }

  editJsonFile<{ devDependencies?: Record<string, string> }>(
    path.join(appProjectPath, 'package.json'),
    (pkg) => {
      if (
        !pkg.devDependencies?.typescript ||
        pkg.devDependencies.typescript === typescriptVersion
      ) {
        return undefined;
      }
      pkg.devDependencies.typescript = typescriptVersion;
      return pkg;
    },
  );

  for (const catalogFile of ['pnpm-workspace.yaml', '.yarnrc.yml']) {
    const catalogPath = path.join(workspaceRootPath, catalogFile);
    if (!fs.existsSync(catalogPath)) {
      continue;
    }
    const workspaceConfig = readYamlFile(catalogPath) as {
      catalog?: Record<string, unknown>;
    } | null;
    const catalogEntry = workspaceConfig?.catalog?.typescript;
    if (catalogEntry === undefined || catalogEntry === typescriptVersion) {
      continue;
    }
    editYamlFile(catalogPath, (doc) => {
      doc.setIn(['catalog', 'typescript'], typescriptVersion);
    });
  }
}

/**
 * Drop the aliased `vite` / `vitest` devDeps that `create-vite` leaves on a
 * scaffolded sub-package. After migration its scripts already use `vp ...` and
 * nothing imports `'vite'` directly, so `vite-plus` provides them transitively.
 *
 * pnpm is the exception and keeps them: pnpm only surfaces the
 * pnpm-workspace.yaml `overrides.vite: catalog:` entry through a package that
 * directly depends on `vite`, so keeping the aliased devDep lets `vp why vite`
 * reflect the override (resolving to @voidzero-dev/vite-plus-core). npm, yarn,
 * and bun redirect the transitive/peer vite via their root
 * overrides/resolutions regardless of a direct dep, so the aliased keys are
 * dead weight and are dropped.
 */
export function dropAliasedRuntimeDevDeps(
  appProjectPath: string,
  packageManager: PackageManager,
): void {
  // pnpm keeps the aliased vite/vitest so the pnpm-workspace.yaml override has
  // a direct consumer to redirect; see the doc comment above.
  if (packageManager === PackageManager.pnpm) {
    return;
  }
  editJsonFile<{ devDependencies?: Record<string, string> }>(
    path.join(appProjectPath, 'package.json'),
    (pkg) => {
      let changed = false;
      for (const name of ['vite', 'vitest']) {
        if (pkg.devDependencies?.[name]) {
          delete pkg.devDependencies[name];
          changed = true;
        }
      }
      return changed ? pkg : undefined;
    },
  );
}

function getScopeFromPackageName(packageName: string) {
  if (packageName.startsWith('@')) {
    return packageName.split('/')[0];
  }
  return '';
}
