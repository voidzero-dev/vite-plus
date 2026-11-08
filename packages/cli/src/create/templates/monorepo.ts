import assert from 'node:assert';
import fs from 'node:fs';
import path from 'node:path';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { removeConfigKey } from '../../../binding/index.js';
import { rewriteMonorepoProject, supportsCatalog } from '../../migration/migrator.ts';
import { PackageManager, type WorkspaceInfo } from '../../types/index.ts';
import { editJsonFile } from '../../utils/json.ts';
import { getScopeFromPackageName } from '../../utils/package.ts';
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
  removeNestedLibraryToolConfig(libraryProjectPath);
  // Perform auto-migration on the created library
  rewriteMonorepoProject(
    libraryProjectPath,
    workspaceInfo.packageManager,
    undefined,
    options?.silent ?? false,
  );

  alignMonorepoTypeScriptVersion(
    fullPath,
    appProjectPath,
    libraryProjectPath,
    workspaceInfo.packageManager,
    workspaceInfo.downloadPackageManager.version,
  );

  return { exitCode: 0, projectDir: templateInfo.targetDir };
}

/**
 * Remove the root-only lint and format options shipped by the standalone library template.
 *
 * The same remote template is also used by `vite:library`, where this config is
 * valid. A library created as a workspace member, however, gets its lint and
 * format config from the monorepo root, so retaining them here creates invalid
 * nested config.
 */
export function removeNestedLibraryToolConfig(projectPath: string): void {
  const configPath = path.join(projectPath, 'vite.config.ts');
  if (!fs.existsSync(configPath)) {
    return;
  }

  for (const configKey of ['lint', 'fmt']) {
    const result = removeConfigKey(configPath, configKey);
    if (result.updated) {
      fs.writeFileSync(configPath, result.content);
    }
  }
}

/**
 * Keep every scaffolded workspace member on the same toolchain versions and,
 * where supported, point those dependencies at the workspace catalog.
 *
 * The app and library come from independently updated remote templates. The
 * library is the compatibility baseline for TypeScript, and its Node types and
 * compiler declarations become the catalog versions for catalog-capable PMs.
 */
export function alignMonorepoTypeScriptVersion(
  workspaceRootPath: string,
  appProjectPath: string,
  libraryProjectPath: string,
  packageManager = PackageManager.pnpm,
  packageManagerVersion = '12.0.0',
): void {
  const appPackagePath = path.join(appProjectPath, 'package.json');
  const libraryPackagePath = path.join(libraryProjectPath, 'package.json');
  const libraryPackage = JSON.parse(fs.readFileSync(libraryPackagePath, 'utf8')) as {
    dependencies?: Record<string, string>;
    devDependencies?: Record<string, string>;
    optionalDependencies?: Record<string, string>;
    peerDependencies?: Record<string, string>;
  };
  const typescriptVersion = libraryPackage.devDependencies?.typescript;
  const catalogCapable = supportsCatalog(packageManager, packageManagerVersion, true);

  let nodeTypesVersion: string | undefined;
  for (const field of [
    'devDependencies',
    'dependencies',
    'optionalDependencies',
    'peerDependencies',
  ] as const) {
    const version = libraryPackage[field]?.['@types/node'];
    if (version) {
      nodeTypesVersion = version;
      break;
    }
  }

  if (typescriptVersion) {
    editJsonFile<{ devDependencies?: Record<string, string> }>(appPackagePath, (pkg) => {
      if (
        pkg.devDependencies?.typescript &&
        pkg.devDependencies.typescript !== typescriptVersion
      ) {
        pkg.devDependencies.typescript = typescriptVersion;
        return pkg;
      }
      return undefined;
    });
  }

  const catalogVersions: Record<string, string> = {};
  if (typescriptVersion) {
    catalogVersions.typescript = typescriptVersion;
  }
  if (nodeTypesVersion) {
    catalogVersions['@types/node'] = nodeTypesVersion;
  }

  const catalogNames = new Set<string>();
  const catalogFile =
    packageManager === PackageManager.pnpm
      ? 'pnpm-workspace.yaml'
      : packageManager === PackageManager.yarn
        ? '.yarnrc.yml'
        : undefined;
  if (catalogCapable && catalogFile) {
    const catalogPath = path.join(workspaceRootPath, catalogFile);
    if (fs.existsSync(catalogPath)) {
      const workspaceConfig = readYamlFile(catalogPath) as {
        catalog?: Record<string, unknown>;
      } | null;
      for (const [name, version] of Object.entries(catalogVersions)) {
        if (workspaceConfig?.catalog?.[name] !== undefined) {
          catalogNames.add(name);
          editYamlFile(catalogPath, (doc) => {
            doc.setIn(['catalog', name], version);
          });
        }
      }
      if (!typescriptVersion && workspaceConfig?.catalog?.typescript !== undefined) {
        editYamlFile(catalogPath, (doc) => {
          doc.deleteIn(['catalog', 'typescript']);
        });
      }
      if (!nodeTypesVersion && workspaceConfig?.catalog?.['@types/node'] !== undefined) {
        editYamlFile(catalogPath, (doc) => {
          doc.deleteIn(['catalog', '@types/node']);
        });
      }
    }
  }

  if (!catalogCapable) {
    return;
  }

  const packagePaths = [appPackagePath, libraryPackagePath];
  for (const packagePath of packagePaths) {
    editJsonFile<{
      dependencies?: Record<string, string>;
      devDependencies?: Record<string, string>;
      optionalDependencies?: Record<string, string>;
      peerDependencies?: Record<string, string>;
    }>(packagePath, (pkg) => {
      let changed = false;
      for (const field of [
        'dependencies',
        'devDependencies',
        'optionalDependencies',
        'peerDependencies',
      ] as const) {
        for (const name of catalogNames) {
          if (pkg[field]?.[name] !== undefined && pkg[field][name] !== 'catalog:') {
            pkg[field][name] = 'catalog:';
            changed = true;
          }
        }
      }
      return changed ? pkg : undefined;
    });
  }
}

/**
 * Drop the aliased `vite` / `vitest` devDeps that `create-vite` leaves on a
 * scaffolded sub-package. After migration its scripts already use `vp ...` and
 * nothing imports `'vite'` directly, so `vite-plus` provides them transitively.
 *
 * pnpm is the exception and keeps them: a package needs a DIRECT `vite` edge for
 * `vite` to resolve to @voidzero-dev/vite-plus-core there (and for `vp why vite`
 * to show it) rather than pnpm auto-installing an upstream Vite to satisfy
 * Vitest's peer. Migration points that edge at the workspace catalog, which owns
 * the alias; the `vite@*` workspace override covers the transitive and peer
 * declarations instead (see `pnpmOverrideKey`). npm, yarn, and bun redirect the
 * transitive/peer vite via their root overrides/resolutions regardless of a
 * direct dep, so the aliased keys are dead weight and are dropped.
 */
export function dropAliasedRuntimeDevDeps(
  appProjectPath: string,
  packageManager: PackageManager,
): void {
  // pnpm keeps the aliased vite/vitest so the package has a direct `vite` edge
  // to point at the workspace catalog; see the doc comment above.
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
