import fs from 'node:fs';
import path from 'node:path';

import { PackageManager, type WorkspaceInfo, type WorkspacePackage } from '../../types/index.ts';
import { VITE_PLUS_NAME, VITE_PLUS_VERSION, isForceOverrideMode } from '../../utils/constants.ts';
import { editJsonFile } from '../../utils/json.ts';
import {
  applyBuildAllowanceToPackageJsonPnpm,
  applyYarnWorkspaceHoistingFix,
  cleanupDeprecatedTsconfigOptions,
  collectInjectedProviderNames,
  collectProviderSourceModes,
  collectVitestEcosystemInstallDependencyNames,
  createCatalogDependencyResolver,
  dropRemovePackageOverrideKeys,
  ensureDirectViteForPnpm,
  ensurePnpmWorkspaceExoticSubdepsSetting,
  findYarnWorkspaceHoisting,
  hasDirectVitePlusInstallEntry,
  hasOwnWebdriverioDependency,
  injectFmtDefaults,
  injectLintTypeCheckDefaults,
  managedOverridePackages,
  mergeStagedConfigToViteConfig,
  mergeTsdownConfigFile,
  mergeViteConfigFiles,
  migratePnpmOverridesToWorkspaceYaml,
  migratePnpmSettingsToWorkspaceYaml,
  pnpmSupportsWorkspaceSettings,
  projectListsRequiredVitestPeer,
  projectUsesVitestDirectly,
  pruneLegacyWrapperAliases,
  removeLintStagedFromPackageJson,
  removeManagedVitestEntry,
  removeVitestPeerDependencyRule,
  rewriteAllImports,
  rewriteBunCatalog,
  rewriteLintStagedConfigFile,
  rewritePackageJson,
  rewritePnpmWorkspaceYaml,
  rewriteRootWorkspacePackageJson,
  rewriteTsconfigTypes,
  rewriteYarnrcYml,
  setDirectViteEdge,
  setPackageManager,
  sourceTreeReferencesRetainedVitestModule,
  takePnpmWorkspaceSettings,
  usesVitestBrowserMode,
  usesWebdriverioProvider,
  workspaceUsesVitestDirectly,
  workspaceUsesWebdriverio,
  wrapLazyPluginsInViteConfig,
  yarnSupportsCatalog,
} from '../migrator.ts';
import { type MigrationReport } from '../report.ts';
import {
  PROVIDER_OVERRIDE_DROP_NAMES,
  pnpmMajor,
  type CatalogDependencyResolver,
  type PnpmPackageJsonSettings,
} from './shared.ts';

export function rewriteStandaloneProject(
  projectPath: string,
  workspaceInfo: WorkspaceInfo,
  skipStagedMigration?: boolean,
  silent = false,
  report?: MigrationReport,
): void {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }

  const packageManager = workspaceInfo.packageManager;
  const catalogDependencyResolver = createCatalogDependencyResolver(projectPath, packageManager);
  const vitestEcosystemPackages = collectVitestEcosystemInstallDependencyNames(projectPath);
  // Source-tree scan signals are computed once here and reused below (and inside
  // projectUsesVitestDirectly / collectInjectedProviderNames) so the source tree
  // is traversed once each instead of repeatedly. They do not depend on
  // package.json contents and no scanned source files are mutated before they
  // are consumed, so the values match the previous lazy per-call scans exactly.
  const providerSourceModes = collectProviderSourceModes(projectPath);
  const browserMode = usesVitestBrowserMode(projectPath);
  const retainedVitestModule = sourceTreeReferencesRetainedVitestModule(projectPath);
  const providerCatalogAdditions = collectInjectedProviderNames(
    projectPath,
    undefined,
    new Map([[projectPath, providerSourceModes]]),
  );
  const pnpmMajorVersion = pnpmMajor(workspaceInfo.downloadPackageManager.version);
  let extractedStagedConfig: Record<string, string | string[]> | null = null;
  let movedPnpmSettings: Record<string, unknown> | undefined;
  let shouldRewritePnpmWorkspaceYaml = false;
  let shouldAddPnpmWorkspaceVitePlusOverride = false;
  let shouldAllowBrowserProviderBuilds = false;
  // Whether the project uses vitest directly (a required-peer consumer, an
  // upstream module reference, or browser mode). Computed inside the callback and
  // hoisted so the post-callback pnpm-workspace.yaml writer sees it too.
  let usesVitest = false;
  // Pure function of the pinned pnpm version, so it is computed up front and
  // reused both inside the callback (the bun direct-`vite` edge and pnpm sinks)
  // and by the post-callback pnpm-workspace.yaml writer.
  const usePnpmWorkspaceYaml =
    packageManager === PackageManager.pnpm &&
    pnpmSupportsWorkspaceSettings(workspaceInfo.downloadPackageManager.version);
  // Whether the active toolchain edges use catalog references (`catalog:`) rather
  // than concrete aliases. Bun standalone projects never manage a catalog
  // (`rewriteBunCatalog` runs only on monorepo roots), so this stays false for
  // bun and the direct `vite` edge resolves to the concrete core alias. Yarn
  // catalogs require Yarn >= 4.10.0 (older Yarn cannot resolve `catalog:`), so a
  // project resolving to an older Yarn falls back to concrete specs.
  const supportCatalog =
    usePnpmWorkspaceYaml ||
    (packageManager === PackageManager.yarn &&
      yarnSupportsCatalog(workspaceInfo.downloadPackageManager.version));
  editJsonFile<{
    overrides?: Record<string, string>;
    resolutions?: Record<string, string>;
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
    peerDependencies?: Record<string, string>;
    optionalDependencies?: Record<string, string>;
    scripts?: Record<string, string>;
    pnpm?: PnpmPackageJsonSettings;
  }>(packageJsonPath, (pkg) => {
    shouldAllowBrowserProviderBuilds =
      hasOwnWebdriverioDependency(pkg) || usesWebdriverioProvider(projectPath);
    const requiredVitestPeer = projectListsRequiredVitestPeer(projectPath, pkg);
    usesVitest = projectUsesVitestDirectly(projectPath, pkg, requiredVitestPeer, true, {
      browserMode,
      retainedModule: retainedVitestModule,
    });
    const managed = managedOverridePackages(usesVitest);
    // Strip stale `vite-plus-test` wrapper aliases before injecting new overrides
    // so the deleted wrapper doesn't survive migration in any sink.
    pruneLegacyWrapperAliases(pkg.resolutions);
    pruneLegacyWrapperAliases(pkg.overrides);
    pruneLegacyWrapperAliases(pkg.pnpm?.overrides);
    // Drop stale provider overrides/resolutions (REMOVE_PACKAGES + the now
    // user-owned opt-in providers, webdriverio/playwright) from the npm/bun
    // `overrides` and yarn `resolutions` sinks before re-merging managed
    // overrides. A leftover pin would conflict with the migrated direct
    // `@vitest/browser-webdriverio` / `@vitest/browser-playwright` dep — npm
    // hard-fails with EOVERRIDE, and yarn/bun would force the stale version over
    // the bundled-vitest-aligned 4.1.9. (The pnpm sinks are pruned below.)
    dropRemovePackageOverrideKeys(pkg.resolutions);
    dropRemovePackageOverrideKeys(pkg.overrides);
    // Common case (no direct vitest): strip a lingering managed `vitest` from
    // the npm/bun `overrides` and yarn `resolutions` sinks so it isn't re-pinned.
    if (!usesVitest) {
      removeManagedVitestEntry(pkg.resolutions);
      removeManagedVitestEntry(pkg.overrides);
    }
    if (packageManager === PackageManager.yarn) {
      pkg.resolutions = {
        ...pkg.resolutions,
        ...managed,
      };
    } else if (packageManager === PackageManager.npm || packageManager === PackageManager.bun) {
      pkg.overrides = {
        ...pkg.overrides,
        ...managed,
      };
      if (packageManager === PackageManager.bun) {
        // Bun walks transitive peer-deps before resolving overrides; vitest
        // 4.1.9 declares peer `vite ^6 || ^7 || ^8` and aborts with
        // "vite@... failed to resolve" if `vite` isn't a direct dep somewhere
        // in the tree, even when the override would redirect it. Mirror the
        // override as a devDep so bun's resolver sees `vite` immediately. A
        // standalone bun project has no catalog (supportCatalog=false), so the
        // shared helper resolves this to the concrete core alias. Verified on
        // bun 1.3.11. See https://github.com/oven-sh/bun/issues/8406.
        setDirectViteEdge(pkg, supportCatalog, catalogDependencyResolver);
      }
    } else if (packageManager === PackageManager.pnpm) {
      if (usePnpmWorkspaceYaml) {
        shouldRewritePnpmWorkspaceYaml = true;
        shouldAddPnpmWorkspaceVitePlusOverride = isForceOverrideMode();
      }
      const overrideKeys = Object.keys(managed);
      if (!usePnpmWorkspaceYaml) {
        // Strip selector-shaped overrides (e.g. `parent>@vitest/browser-playwright`)
        // whose target is a removed package, before re-merging the user's
        // overrides into the new pnpm config.
        dropRemovePackageOverrideKeys(pkg.pnpm?.overrides);
        // Common case: drop a lingering managed `vitest` override + its peer
        // rules before re-merging.
        if (!usesVitest) {
          removeManagedVitestEntry(pkg.pnpm?.overrides);
          if (pkg.pnpm?.peerDependencyRules) {
            removeVitestPeerDependencyRule(pkg.pnpm.peerDependencyRules);
          }
        }
        // Project already has pnpm config in package.json -- keep using it.
        pkg.pnpm = {
          ...pkg.pnpm,
          overrides: {
            ...pkg.pnpm?.overrides,
            ...managed,
            ...(isForceOverrideMode() ? { [VITE_PLUS_NAME]: VITE_PLUS_VERSION } : {}),
          },
          peerDependencyRules: {
            ...pkg.pnpm?.peerDependencyRules,
            allowAny: [
              ...new Set([...(pkg.pnpm?.peerDependencyRules?.allowAny ?? []), ...overrideKeys]),
            ],
            allowedVersions: {
              ...pkg.pnpm?.peerDependencyRules?.allowedVersions,
              ...Object.fromEntries(overrideKeys.map((key) => [key, '*'])),
            },
          },
        };
      } else {
        movedPnpmSettings = takePnpmWorkspaceSettings(pkg);
      }
      // remove dependency selectors targeting vite (e.g. "vite-plugin-svgr>vite")
      for (const key in pkg.pnpm?.overrides) {
        if (key.includes('>')) {
          const splits = key.split('>');
          if (splits[splits.length - 1].trim() === 'vite') {
            delete pkg.pnpm.overrides[key];
          }
        }
      }
      // remove packages from `resolutions` field if they exist
      // https://pnpm.io/9.x/package_json#resolutions
      for (const key of [...overrideKeys, ...PROVIDER_OVERRIDE_DROP_NAMES]) {
        if (pkg.resolutions?.[key]) {
          delete pkg.resolutions[key];
        }
      }
      if (!usePnpmWorkspaceYaml && pnpmMajorVersion !== undefined && pkg.pnpm) {
        applyBuildAllowanceToPackageJsonPnpm(
          pkg.pnpm,
          pnpmMajorVersion,
          shouldAllowBrowserProviderBuilds,
        );
      }
    }

    extractedStagedConfig = rewritePackageJson(
      pkg,
      packageManager,
      supportCatalog,
      skipStagedMigration,
      catalogDependencyResolver,
      browserMode,
      providerSourceModes,
      usesVitest,
      retainedVitestModule,
      requiredVitestPeer,
    );

    // ensure vite-plus is in devDependencies — but only when it isn't already a
    // direct dependency/devDependency, so a project that declares vite-plus in
    // `dependencies` is not duplicated into `devDependencies`. Force-override
    // still re-pins a pre-existing devDependencies entry in place.
    const forceRepinExistingDevEntry =
      isForceOverrideMode() && pkg.devDependencies?.[VITE_PLUS_NAME] !== undefined;
    if (!hasDirectVitePlusInstallEntry(pkg) || forceRepinExistingDevEntry) {
      const existingVitePlusSpec = pkg.devDependencies?.[VITE_PLUS_NAME];
      const version =
        supportCatalog && !VITE_PLUS_VERSION.startsWith('file:')
          ? existingVitePlusSpec?.startsWith('catalog:')
            ? existingVitePlusSpec
            : (catalogDependencyResolver?.preferredCatalogSpec ?? 'catalog:')
          : VITE_PLUS_VERSION;
      pkg.devDependencies = {
        ...pkg.devDependencies,
        [VITE_PLUS_NAME]: version,
      };
    }
    // This caller injects vite-plus after rewritePackageJson returned, so the
    // direct-`vite` pass must run here too.
    ensureDirectViteForPnpm(pkg, packageManager, supportCatalog, catalogDependencyResolver);
    return pkg;
  });

  migratePnpmSettingsToWorkspaceYaml(projectPath, movedPnpmSettings);

  if (shouldRewritePnpmWorkspaceYaml) {
    rewritePnpmWorkspaceYaml(
      projectPath,
      pnpmMajorVersion,
      shouldAllowBrowserProviderBuilds,
      usesVitest,
      vitestEcosystemPackages,
      true,
      providerCatalogAdditions,
    );
  }

  if (shouldAddPnpmWorkspaceVitePlusOverride) {
    migratePnpmOverridesToWorkspaceYaml(projectPath, {
      [VITE_PLUS_NAME]: VITE_PLUS_VERSION,
    });
  }

  if (packageManager === PackageManager.pnpm) {
    ensurePnpmWorkspaceExoticSubdepsSetting(projectPath);
  }

  if (packageManager === PackageManager.yarn) {
    rewriteYarnrcYml(
      projectPath,
      usesVitest,
      vitestEcosystemPackages,
      providerCatalogAdditions,
      supportCatalog,
    );
  }

  // Merge extracted staged config into vite.config.ts, then remove lint-staged from package.json
  if (extractedStagedConfig) {
    if (mergeStagedConfigToViteConfig(projectPath, extractedStagedConfig, silent, report)) {
      removeLintStagedFromPackageJson(packageJsonPath);
    }
  }

  if (!skipStagedMigration) {
    rewriteLintStagedConfigFile(projectPath, report);
  }
  cleanupDeprecatedTsconfigOptions(projectPath, silent, report);
  rewriteTsconfigTypes(projectPath, silent, report);
  mergeViteConfigFiles(projectPath, silent, report, workspaceInfo.packages);
  injectLintTypeCheckDefaults(projectPath, silent, report);
  injectFmtDefaults(projectPath, silent, report);
  mergeTsdownConfigFile(projectPath, silent, report);
  // rewrite imports in all TypeScript/JavaScript files before lazy plugin import merging
  rewriteAllImports(projectPath, silent, report, true);
  wrapLazyPluginsInViteConfig(projectPath, silent, report);
  // set package manager
  setPackageManager(projectPath, workspaceInfo.downloadPackageManager);
}

/**
 * Rewrite monorepo to add vite-plus dependencies
 * @param workspaceInfo - The workspace info
 */
export function rewriteMonorepo(
  workspaceInfo: WorkspaceInfo,
  skipStagedMigration?: boolean,
  silent = false,
  report?: MigrationReport,
): void {
  const catalogDependencyResolver = createCatalogDependencyResolver(
    workspaceInfo.rootDir,
    workspaceInfo.packageManager,
  );
  const pnpmMajorVersion = pnpmMajor(workspaceInfo.downloadPackageManager.version);
  const usePnpmWorkspaceSettings = pnpmSupportsWorkspaceSettings(
    workspaceInfo.downloadPackageManager.version,
  );
  const workspaceShouldAllowBrowserBuilds = workspaceUsesWebdriverio(
    workspaceInfo.rootDir,
    workspaceInfo.packages,
  );
  // The SHARED workspace sinks (catalog / overrides / peer rules) keep `vitest`
  // managed iff ANY package in the workspace uses vitest directly.
  const workspaceUsesVitest = workspaceUsesVitestDirectly(
    workspaceInfo.rootDir,
    workspaceInfo.packages,
    true,
  );
  const vitestEcosystemPackages = collectVitestEcosystemInstallDependencyNames(
    workspaceInfo.rootDir,
    workspaceInfo.packages,
  );
  const providerCatalogAdditions = collectInjectedProviderNames(
    workspaceInfo.rootDir,
    workspaceInfo.packages,
  );
  // Yarn catalogs require Yarn >= 4.10.0; a workspace resolving to an older Yarn
  // falls back to concrete specs. pnpm/bun keep their existing catalog behavior
  // (driven by their own pnpm-workspace.yaml / bun catalog sinks).
  const supportCatalog =
    workspaceInfo.packageManager === PackageManager.yarn
      ? yarnSupportsCatalog(workspaceInfo.downloadPackageManager.version)
      : true;
  // rewrite root workspace
  if (workspaceInfo.packageManager === PackageManager.yarn) {
    rewriteYarnrcYml(
      workspaceInfo.rootDir,
      workspaceUsesVitest,
      vitestEcosystemPackages,
      providerCatalogAdditions,
      supportCatalog,
    );
  } else if (workspaceInfo.packageManager === PackageManager.bun) {
    rewriteBunCatalog(workspaceInfo.rootDir, workspaceUsesVitest, vitestEcosystemPackages);
  }
  rewriteRootWorkspacePackageJson(
    workspaceInfo.rootDir,
    workspaceInfo.packageManager,
    skipStagedMigration,
    catalogDependencyResolver,
    workspaceInfo.packages,
    pnpmMajorVersion,
    workspaceInfo.downloadPackageManager.version,
    workspaceShouldAllowBrowserBuilds,
    workspaceUsesVitest,
    supportCatalog,
  );
  if (workspaceInfo.packageManager === PackageManager.pnpm) {
    rewritePnpmWorkspaceYaml(
      workspaceInfo.rootDir,
      pnpmMajorVersion,
      workspaceShouldAllowBrowserBuilds,
      workspaceUsesVitest,
      vitestEcosystemPackages,
      usePnpmWorkspaceSettings,
      providerCatalogAdditions,
    );
    if (usePnpmWorkspaceSettings && isForceOverrideMode()) {
      migratePnpmOverridesToWorkspaceYaml(workspaceInfo.rootDir, {
        [VITE_PLUS_NAME]: VITE_PLUS_VERSION,
      });
    }
  }
  // (mergeViteConfigFiles below will sanitize the merged lint config
  // against this workspace's full package set.)

  // rewrite packages — pass workspace context so the per-package
  // sanitizer can see hoisted deps that live elsewhere in the
  // workspace, not just this sub-package's own `package.json`.
  const workspaceContext = {
    rootDir: workspaceInfo.rootDir,
    packages: workspaceInfo.packages,
  };
  // Yarn `node-modules` + an isolating `nmHoistingLimits` would give each
  // vite-plus-receiving workspace its own physical `vitest` copy, splitting the
  // runner across two `@vitest/runner` instances. `rewriteMonorepoProject` detects
  // the layout per workspace (reading the root `.yarnrc.yml` itself) and auto-fixes
  // or warns — see `applyYarnWorkspaceHoistingFix`.
  for (const pkg of workspaceInfo.packages) {
    rewriteMonorepoProject(
      path.join(workspaceInfo.rootDir, pkg.path),
      workspaceInfo.packageManager,
      skipStagedMigration,
      silent,
      report,
      catalogDependencyResolver,
      workspaceContext,
      true,
      supportCatalog,
    );
  }

  if (!skipStagedMigration) {
    rewriteLintStagedConfigFile(workspaceInfo.rootDir, report);
  }
  cleanupDeprecatedTsconfigOptions(workspaceInfo.rootDir, silent, report);
  rewriteTsconfigTypes(workspaceInfo.rootDir, silent, report);
  mergeViteConfigFiles(workspaceInfo.rootDir, silent, report, workspaceInfo.packages);
  injectLintTypeCheckDefaults(workspaceInfo.rootDir, silent, report);
  injectFmtDefaults(workspaceInfo.rootDir, silent, report);
  mergeTsdownConfigFile(workspaceInfo.rootDir, silent, report);
  // rewrite imports in all TypeScript/JavaScript files before lazy plugin import merging
  rewriteAllImports(workspaceInfo.rootDir, silent, report, true);
  wrapLazyPluginsInViteConfig(workspaceInfo.rootDir, silent, report);
  for (const pkg of workspaceInfo.packages) {
    wrapLazyPluginsInViteConfig(path.join(workspaceInfo.rootDir, pkg.path), silent, report);
  }
  // set package manager
  setPackageManager(workspaceInfo.rootDir, workspaceInfo.downloadPackageManager);
}

/**
 * Rewrite monorepo project to add vite-plus dependencies
 * @param projectPath - The path to the project
 * @param workspaceContext - Full workspace info, used so the lint-config
 *   sanitizer can see hoisted deps living elsewhere in the workspace,
 *   not just this sub-package's own `package.json`. `rootDir` is the
 *   workspace root (paths in `packages` are relative to it); `packages`
 *   is the workspace package list.
 */
export function rewriteMonorepoProject(
  projectPath: string,
  packageManager: PackageManager,
  skipStagedMigration?: boolean,
  silent = false,
  report?: MigrationReport,
  catalogDependencyResolver?: CatalogDependencyResolver,
  workspaceContext?: { rootDir: string; packages: WorkspacePackage[] },
  deferLazyPluginWrapping = false,
  // Whether this workspace's edges use catalog references. Yarn catalogs require
  // Yarn >= 4.10.0; pnpm/bun monorepos always manage a catalog. Defaults to true
  // so the `vp create` callers (always a catalog-capable bundled PM) are covered.
  supportCatalog = true,
): void {
  cleanupDeprecatedTsconfigOptions(projectPath, silent, report);
  rewriteTsconfigTypes(projectPath, silent, report);
  mergeViteConfigFiles(
    projectPath,
    silent,
    report,
    workspaceContext?.packages,
    workspaceContext?.rootDir,
  );
  mergeTsdownConfigFile(projectPath, silent, report);

  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }

  // Yarn `nmHoistingLimits` for this workspace's project, found by walking up to the
  // root `.yarnrc.yml`. Derived here (not threaded as an arg) so EVERY caller — full
  // monorepo migration, a direct `rewriteMonorepoProject` call, and `vp create`
  // integrating a package into an existing monorepo — is covered. undefined for
  // non-Yarn repos.
  const yarnHoisting =
    packageManager === PackageManager.yarn
      ? findYarnWorkspaceHoisting(workspaceContext?.rootDir ?? projectPath)
      : undefined;

  let extractedStagedConfig: Record<string, string | string[]> | null = null;
  editJsonFile<{
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
    peerDependencies?: Record<string, string>;
    optionalDependencies?: Record<string, string>;
    scripts?: Record<string, string>;
    installConfig?: { hoistingLimits?: string };
  }>(packageJsonPath, (pkg) => {
    const requiredVitestPeer = projectListsRequiredVitestPeer(projectPath, pkg);
    // Compute the browser-mode and retained-module source scans once and reuse
    // them across rewritePackageJson and projectUsesVitestDirectly: the scans do
    // not depend on package.json and nothing mutates the source tree between
    // these reads, so this is identical to the previous per-call scans.
    const browserMode = usesVitestBrowserMode(projectPath);
    const retainedVitestModule = sourceTreeReferencesRetainedVitestModule(projectPath);
    // rewrite scripts in package.json
    extractedStagedConfig = rewritePackageJson(
      pkg,
      packageManager,
      supportCatalog,
      skipStagedMigration,
      catalogDependencyResolver,
      browserMode,
      collectProviderSourceModes(projectPath),
      projectUsesVitestDirectly(projectPath, pkg, requiredVitestPeer, true, {
        browserMode,
        retainedModule: retainedVitestModule,
      }),
      retainedVitestModule,
      requiredVitestPeer,
    );
    // If this SUB-workspace now depends on `vite-plus` and Yarn isolates its
    // hoisting (via the root `nmHoistingLimits` OR the workspace's own
    // `installConfig.hoistingLimits`), dedupe the bundled `vitest` family to the
    // single shared root copy (avoids the dual-`@vitest/runner` "reading 'config'"
    // crash), or warn when the split cannot be fixed from package.json. The monorepo
    // root itself is skipped (`projectPath === yarnHoisting.rootDir`): its deps
    // already hoist to the top level, so it never needs an opt-out.
    if (
      yarnHoisting &&
      path.resolve(projectPath) !== yarnHoisting.rootDir &&
      hasDirectVitePlusInstallEntry(pkg)
    ) {
      applyYarnWorkspaceHoistingFix(
        pkg,
        yarnHoisting.limit,
        yarnHoisting.nodeLinker,
        path.relative(yarnHoisting.rootDir, projectPath) || projectPath,
        report,
      );
    }
    return pkg;
  });

  // Merge extracted staged config into vite.config.ts, then remove lint-staged from package.json
  if (extractedStagedConfig) {
    if (mergeStagedConfigToViteConfig(projectPath, extractedStagedConfig, silent, report)) {
      removeLintStagedFromPackageJson(packageJsonPath);
    }
  }

  if (!deferLazyPluginWrapping) {
    wrapLazyPluginsInViteConfig(projectPath, silent, report);
  }
}
