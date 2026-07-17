import fs from 'node:fs';
import path from 'node:path';

import { PackageManager, type WorkspaceInfo, type WorkspacePackage } from '../../types/index.ts';
import {
  VITEST_VERSION,
  VITE_PLUS_NAME,
  VITE_PLUS_OVERRIDE_PACKAGES,
  VITE_PLUS_VERSION,
  isForceOverrideMode,
} from '../../utils/constants.ts';
import { editJsonFile, readJsonFile } from '../../utils/json.ts';
import { type NpmWorkspaces } from '../../utils/workspace.ts';
import { editYamlFile, readYamlFile, type YamlDocument } from '../../utils/yaml.ts';
import {
  alignVitestEcosystemPackages,
  applyBuildAllowanceToPackageJsonPnpm,
  applyYarnWorkspaceHoistingFix,
  collectProviderSourceModes,
  collectVitestEcosystemInstallDependencyNames,
  createCatalogDependencyResolver,
  ensureDirectViteForPnpm,
  ensurePnpmWorkspacePackages,
  findYarnWorkspaceHoisting,
  getAlignedVitestEcosystemDependencySpec,
  getCatalogDependencySpec,
  isAlignableVitestEcosystemPackage,
  isLegacyWrapperSpec,
  isProtocolPinnedSpec,
  managedOverridePackages,
  migratePnpmSettingsToWorkspaceYaml,
  normalizeVitestPeerCatalogSpec,
  pnpmPackageJsonSettingsPending,
  pnpmSupportsWorkspaceSettings,
  supportsCatalog,
  pnpmWorkspaceMinimumReleaseAgeExemptionsPending,
  projectUsesVitestDirectly,
  pruneLegacyWrapperAliases,
  readBunCatalogDependencyResolver,
  readPnpmWorkspaceCatalogDependencyResolver,
  readPnpmWorkspaceOverrides,
  readPnpmWorkspacePeerDependencyRules,
  removeManagedVitestEntry,
  rewriteBunCatalog,
  rewritePnpmWorkspaceYaml,
  rewriteYarnrcYml,
  setDirectViteEdge,
  setPackageManager,
  takePnpmWorkspaceSettings,
  vitestEcosystemCatalogReferencesPending,
  workspaceUsesVitestDirectly,
  workspaceUsesWebdriverio,
  yarnrcSatisfiesVitePlus,
} from '../migrator.ts';
import { type DependencyVersionChange, type MigrationReport } from '../report.ts';
import {
  BROWSER_PROVIDER_PEER_DEPS,
  resolveProviderPeerSpec,
  OPT_IN_BROWSER_PROVIDERS,
  REMOVE_PACKAGES,
  VITEST_IS_MANAGED_OVERRIDE,
  pnpmMajor,
  type CatalogDependencyResolver,
  type PnpmPackageJsonSettings,
} from './shared.ts';

// Bun resolves `catalog:` references (and a top-level `catalog` field) ONLY
// inside a workspace: a root package.json declaring a non-empty `workspaces`
// (the array form, or the object `{ packages: [...] }` form). A standalone bun
// project has nowhere to resolve a catalog from, so converting its edges to
// `catalog:` makes `bun install` abort with "vite@catalog: failed to resolve".
// The upgrade path must keep standalone bun on concrete specs, mirroring the
// fresh path: `supportCatalog` is false for standalone bun (orchestrators.ts)
// and `rewriteBunCatalog` runs only on monorepo roots (orchestrators.ts).
function bunWorkspaceDeclaresPackages(workspaces: NpmWorkspaces | undefined): boolean {
  if (Array.isArray(workspaces)) {
    return workspaces.length > 0;
  }
  if (workspaces && typeof workspaces === 'object') {
    return Array.isArray(workspaces.packages) && workspaces.packages.length > 0;
  }
  return false;
}

export type BootstrapPackageJson = {
  overrides?: Record<string, string>;
  resolutions?: Record<string, string>;
  devDependencies?: Record<string, string>;
  dependencies?: Record<string, string>;
  peerDependencies?: Record<string, string>;
  optionalDependencies?: Record<string, string>;
  pnpm?: PnpmPackageJsonSettings;
  packageManager?: string;
  devEngines?: { packageManager?: unknown; [key: string]: unknown };
};

export type VitePlusBootstrapResult = {
  changed: boolean;
  packageJson: boolean;
  packageManagerConfig: boolean;
  packageManagerField: boolean;
};

function isSemanticVitePlusOverrideSpec(dependencyName: string, spec: string | undefined): boolean {
  if (!spec) {
    return false;
  }
  // A spec still pointing at the deleted `@voidzero-dev/vite-plus-test` wrapper
  // is stale, NOT satisfied: this release ships upstream vitest directly, so the
  // wrapper must be rewritten/pruned to the bundled vitest rather than accepted
  // (otherwise `detectVitePlusBootstrapPending` skips writing the new
  // `vitest: VITEST_VERSION` and the override keeps installing the dead wrapper).
  if (isLegacyWrapperSpec(spec)) {
    return false;
  }
  if (spec === VITE_PLUS_OVERRIDE_PACKAGES[dependencyName]) {
    return true;
  }
  return false;
}

function overrideSpecSatisfiesVitePlus(
  dependencyName: string,
  spec: string | undefined,
  catalogDependencyResolver?: CatalogDependencyResolver,
): boolean {
  if (!spec) {
    return false;
  }
  if (isSemanticVitePlusOverrideSpec(dependencyName, spec)) {
    return true;
  }
  if (!spec.startsWith('catalog:')) {
    return false;
  }
  return isSemanticVitePlusOverrideSpec(
    dependencyName,
    catalogDependencyResolver?.(spec, dependencyName),
  );
}

export function overridesSatisfyVitePlus(
  overrides: Record<string, string> | undefined,
  usesVitest: boolean,
  catalogDependencyResolver?: CatalogDependencyResolver,
): boolean {
  // Common case: a lingering managed `vitest` override is NOT satisfied — it
  // must be removed, so the bootstrap stays pending until it is.
  if (!usesVitest && VITEST_IS_MANAGED_OVERRIDE && typeof overrides?.vitest === 'string') {
    return false;
  }
  return Object.keys(managedOverridePackages(usesVitest)).every((dependencyName) =>
    overrideSpecSatisfiesVitePlus(
      dependencyName,
      overrides?.[dependencyName],
      catalogDependencyResolver,
    ),
  );
}

function hasPackageManagerPin(pkg: BootstrapPackageJson): boolean {
  return Boolean(pkg.packageManager || pkg.devEngines?.packageManager);
}

function pinnedPackageManagerVersion(pkg: BootstrapPackageJson): string | undefined {
  if (typeof pkg.packageManager === 'string') {
    const separator = pkg.packageManager.indexOf('@');
    if (separator !== -1) {
      return pkg.packageManager.slice(separator + 1);
    }
  }
  const devEngine = pkg.devEngines?.packageManager;
  if (
    typeof devEngine === 'object' &&
    devEngine !== null &&
    !Array.isArray(devEngine) &&
    'version' in devEngine &&
    typeof devEngine.version === 'string'
  ) {
    return devEngine.version;
  }
  return undefined;
}

function vitePlusDependencyNeedsConcreteVersion(pkg: BootstrapPackageJson): boolean {
  const dependencyGroups = [pkg.devDependencies, pkg.dependencies, pkg.optionalDependencies];
  return dependencyGroups.some(
    (dependencies) => dependencies?.[VITE_PLUS_NAME]?.startsWith('catalog:') ?? false,
  );
}

function catalogVitePlusDependencyPending(
  pkg: BootstrapPackageJson,
  catalogDependencyResolver: CatalogDependencyResolver | undefined,
): boolean {
  const dependencyGroups = [pkg.devDependencies, pkg.dependencies, pkg.optionalDependencies];
  return dependencyGroups.some((dependencies) => {
    const spec = dependencies?.[VITE_PLUS_NAME];
    if (!spec?.startsWith('catalog:')) {
      return false;
    }
    return catalogDependencyResolver?.(spec, VITE_PLUS_NAME) !== VITE_PLUS_VERSION;
  });
}

function pnpmPeerDependencyRulesSatisfyVitePlus(
  peerDependencyRules:
    | { allowAny?: string[]; allowedVersions?: Record<string, string> }
    | undefined,
  usesVitest: boolean,
): boolean {
  const allowAny = new Set(peerDependencyRules?.allowAny ?? []);
  const allowedVersions = peerDependencyRules?.allowedVersions ?? {};
  // Common case: a lingering managed `vitest` peer rule is NOT satisfied.
  if (
    !usesVitest &&
    VITEST_IS_MANAGED_OVERRIDE &&
    (allowAny.has('vitest') || allowedVersions.vitest !== undefined)
  ) {
    return false;
  }
  const overrideKeys = Object.keys(managedOverridePackages(usesVitest));
  return overrideKeys.every((key) => allowAny.has(key) && allowedVersions[key] === '*');
}

function npmVitePlusManagedDependenciesPending(
  pkg: BootstrapPackageJson,
  usesVitest: boolean,
): boolean {
  const dependencyGroups = [pkg.devDependencies, pkg.dependencies, pkg.optionalDependencies];
  // Common case: a lingering managed `vitest` install dep is pending removal.
  if (
    !usesVitest &&
    VITEST_IS_MANAGED_OVERRIDE &&
    dependencyGroups.some((dependencies) => dependencies?.vitest !== undefined)
  ) {
    return true;
  }
  return Object.keys(managedOverridePackages(usesVitest)).some((dependencyName) =>
    dependencyGroups.some(
      (dependencies) =>
        dependencies?.[dependencyName] !== undefined &&
        !overrideSpecSatisfiesVitePlus(dependencyName, dependencies[dependencyName]),
    ),
  );
}

function forceOverrideUsesExoticPnpmSpec(): boolean {
  if (!isForceOverrideMode()) {
    return false;
  }
  return [VITE_PLUS_VERSION, ...Object.values(VITE_PLUS_OVERRIDE_PACKAGES)].some((spec) =>
    /^(?:file|https?):/.test(spec),
  );
}

function pnpmWorkspaceExoticSubdepsSettingSatisfied(projectPath: string): boolean {
  if (!forceOverrideUsesExoticPnpmSpec()) {
    return true;
  }
  const pnpmWorkspaceYamlPath = path.join(projectPath, 'pnpm-workspace.yaml');
  if (!fs.existsSync(pnpmWorkspaceYamlPath)) {
    return false;
  }
  const doc = readYamlFile(pnpmWorkspaceYamlPath) as { blockExoticSubdeps?: boolean } | null;
  return doc?.blockExoticSubdeps === false;
}

export function ensurePnpmExoticSubdepsSetting(doc: YamlDocument): boolean {
  if (!forceOverrideUsesExoticPnpmSpec() || doc.get('blockExoticSubdeps') === false) {
    return false;
  }
  doc.set('blockExoticSubdeps', false);
  return true;
}

export function ensurePnpmWorkspaceExoticSubdepsSetting(projectPath: string): boolean {
  if (!forceOverrideUsesExoticPnpmSpec()) {
    return false;
  }
  const pnpmWorkspaceYamlPath = path.join(projectPath, 'pnpm-workspace.yaml');
  if (!fs.existsSync(pnpmWorkspaceYamlPath)) {
    fs.writeFileSync(pnpmWorkspaceYamlPath, '');
  }
  let changed = false;
  editYamlFile(pnpmWorkspaceYamlPath, (doc) => {
    changed = ensurePnpmExoticSubdepsSetting(doc);
  });
  return changed;
}

/**
 * Reconcile the install dependencies in one package during an existing-Vite+
 * bootstrap. Package-manager overrides are intentionally handled separately at
 * the workspace root; this function owns only dependency fields so it can also
 * be applied to every workspace package.
 */
function reconcileVitePlusBootstrapPackage(
  projectPath: string,
  pkg: BootstrapPackageJson,
  vitePlusVersion: string,
  packageManager: PackageManager,
  supportCatalog: boolean,
  ensureVitePlus: boolean,
  catalogDependencyResolver?: CatalogDependencyResolver,
  // Opt-in browser providers the workspace catalog is gaining (some package uses
  // one only through source/a shim). An already-installed copy of such a provider
  // must REFERENCE that catalog entry, not pin a concrete version. See #2005.
  providerCatalogAdditions: ReadonlySet<string> = new Set(),
): boolean {
  const before = JSON.stringify(pkg);
  const usesVitest = projectUsesVitestDirectly(projectPath, pkg, undefined, true);
  ensureVitePlusDependencySpecs(pkg, vitePlusVersion, ensureVitePlus);

  const installGroups = [pkg.devDependencies, pkg.dependencies, pkg.optionalDependencies];
  const dependencyGroups = [...installGroups, pkg.peerDependencies];

  // Remove every dependency alias to the deleted wrapper before deciding
  // whether this package needs a direct upstream vitest peer provider.
  for (const dependencies of dependencyGroups) {
    pruneLegacyWrapperAliases(dependencies);
  }

  // Normalize direct Vite install entries as well as the shared override. Keep
  // named catalog references intact; plain/behind aliases move to the active
  // default catalog or the current core alias.
  for (const dependencies of installGroups) {
    if (dependencies?.vite !== undefined) {
      dependencies.vite = getCatalogDependencySpec(
        dependencies.vite,
        VITE_PLUS_OVERRIDE_PACKAGES.vite,
        supportCatalog,
        { preferredCatalogSpec: catalogDependencyResolver?.preferredCatalogSpec },
      );
    }
  }

  alignVitestEcosystemPackages(pkg, packageManager, supportCatalog, catalogDependencyResolver);
  normalizeVitestPeerCatalogSpec(pkg.peerDependencies, catalogDependencyResolver);

  const providerSourceModes = collectProviderSourceModes(projectPath);
  let usesAnyOptInProvider = false;
  for (const provider of OPT_IN_BROWSER_PROVIDERS) {
    const usesProvider =
      providerSourceModes[provider] ||
      dependencyGroups.some((dependencies) => dependencies?.[provider] !== undefined);
    if (!usesProvider) {
      continue;
    }
    usesAnyOptInProvider = true;
    const installGroupEntry = [
      { dependencyField: 'devDependencies' as const, dependencies: pkg.devDependencies },
      { dependencyField: 'dependencies' as const, dependencies: pkg.dependencies },
      {
        dependencyField: 'optionalDependencies' as const,
        dependencies: pkg.optionalDependencies,
      },
    ].find(({ dependencies }) => dependencies?.[provider] !== undefined);
    if (installGroupEntry?.dependencies) {
      if (VITEST_IS_MANAGED_OVERRIDE) {
        // When the workspace catalog is gaining this provider (another package
        // uses it source-only), reference the catalog entry — excluding standalone
        // bun, which has no catalog — instead of pinning a concrete version that
        // would leave the entry unused. Otherwise align it normally. See #2005.
        installGroupEntry.dependencies[provider] = getAlignedVitestEcosystemDependencySpec(
          installGroupEntry.dependencies[provider],
          provider,
          installGroupEntry.dependencyField,
          packageManager,
          providerCatalogAdditions.has(provider)
            ? supportCatalog && packageManager !== PackageManager.bun
            : supportCatalog,
          catalogDependencyResolver,
        );
      }
    } else {
      pkg.devDependencies ??= {};
      pkg.devDependencies[provider] = getCatalogDependencySpec(
        undefined,
        VITEST_VERSION,
        supportCatalog && packageManager !== PackageManager.bun,
        { preferredCatalogSpec: catalogDependencyResolver?.preferredCatalogSpec },
      );
    }
    const frameworkPeer = BROWSER_PROVIDER_PEER_DEPS[provider];
    const frameworkPresent = dependencyGroups.some(
      (dependencies) => dependencies?.[frameworkPeer] !== undefined,
    );
    if (frameworkPeer && !frameworkPresent) {
      pkg.devDependencies ??= {};
      pkg.devDependencies[frameworkPeer] = resolveProviderPeerSpec(
        pkg,
        frameworkPeer,
        supportCatalog,
        catalogDependencyResolver,
      );
    }
  }

  // The full bundled set (oxlint/oxlint-tsgolint/oxfmt/tsdown plus the base
  // `@vitest/browser`/preview runtime) is owned by vite-plus, so strip every
  // REMOVE_PACKAGES entry from this package's install groups, matching the
  // catalog removal (catalog.ts) and the fresh path (package-json.ts). Leaving
  // any of them behind keeps a `catalog:` reference whose catalog entry the
  // catalog rewrite just deleted, dangling the next `pnpm install`. A plain
  // delete is correct here: none of these carry a project-owned peer (the
  // browser-provider peer logic is for the opt-in `@vitest/browser-*`
  // providers, which are deliberately NOT in REMOVE_PACKAGES).
  for (const bundledPackage of REMOVE_PACKAGES) {
    for (const dependencies of installGroups) {
      if (dependencies?.[bundledPackage] !== undefined) {
        delete dependencies[bundledPackage];
      }
    }
  }

  if (usesAnyOptInProvider && packageManager === PackageManager.npm) {
    const viteAlreadyDirect = installGroups.some(
      (dependencies) => dependencies?.vite !== undefined,
    );
    if (!viteAlreadyDirect) {
      // npm has no catalog (supportCatalog=false), so the shared helper resolves
      // the direct edge to the concrete core alias, placed in sorted order.
      setDirectViteEdge(pkg, supportCatalog, catalogDependencyResolver);
    }
  }

  if (packageManager === PackageManager.bun) {
    // Bun resolves vitest's `vite ^6 || ^7 || ^8` peer before applying the
    // override that redirects `vite` to vite-plus-core, and aborts with
    // "vite@... failed to resolve" unless `vite` is a direct dependency. Mirror
    // the full-migration path (rewriteStandaloneProject) so the idempotent
    // bootstrap path also produces an installable bun project. Only the PRESENCE
    // of a direct `vite` edge matters for #8406: a `catalog:` reference satisfies
    // it just as well as a concrete alias because catalog refs resolve during the
    // dependency-graph build (unlike overrides). The shared helper gives
    // catalog-capable bun a `catalog:` edge (matching the catalog/override sinks)
    // and the concrete core alias otherwise. Verified on bun 1.3.11.
    // See https://github.com/oven-sh/bun/issues/8406.
    //
    // Only the vitest peer drags `vite` in, so gate the injection to packages that
    // actually pull vitest — a direct `vite-plus` (bundles vitest), a direct/used
    // vitest, or an opt-in browser provider (a vitest peer) — mirroring the
    // pnpm/npm branches. Injecting `vite` into every workspace package would dirty
    // unrelated workspaces that don't depend on the vitest family.
    const needsDirectVite =
      hasDirectVitePlusInstallEntry(pkg) || usesVitest || usesAnyOptInProvider;
    const viteAlreadyDirect = installGroups.some(
      (dependencies) => dependencies?.vite !== undefined,
    );
    if (needsDirectVite && !viteAlreadyDirect) {
      setDirectViteEdge(pkg, supportCatalog, catalogDependencyResolver);
    }
  }

  if (usesVitest) {
    // A direct @vitest/*/integration dependency with a required vitest peer
    // cannot use the copy nested under its sibling `vite-plus` dependency under
    // Yarn PnP or strict pnpm. Provide the peer from this package and keep it on
    // the same exact version as the Vite+ runner.
    const existingGroup = installGroups.find((dependencies) => dependencies?.vitest !== undefined);
    if (existingGroup) {
      if (VITEST_IS_MANAGED_OVERRIDE) {
        existingGroup.vitest = getCatalogDependencySpec(
          existingGroup.vitest,
          VITEST_VERSION,
          supportCatalog,
          { preferredCatalogSpec: catalogDependencyResolver?.preferredCatalogSpec },
        );
      }
    } else {
      pkg.devDependencies ??= {};
      pkg.devDependencies.vitest = getCatalogDependencySpec(
        undefined,
        VITEST_VERSION,
        supportCatalog,
        { preferredCatalogSpec: catalogDependencyResolver?.preferredCatalogSpec },
      );
    }
  } else {
    // Bare vitest is not itself a usage signal: older migrations injected it
    // into every project. Remove that stale install pin when no remaining peer,
    // source import, or browser-mode signal needs it.
    for (const dependencies of installGroups) {
      removeManagedVitestEntry(dependencies);
    }
  }

  // #1932: the full-migration path injects a direct pnpm `vite` edge via
  // rewriteRootWorkspacePackageJson / rewriteMonorepoProject; the existing-Vite+
  // upgrade (bootstrap/re-pin) path reaches package.json only through here, so it
  // must add the same edge (the npm-opt-in and bun branches above cover those).
  // See ensureDirectViteForPnpm for why the direct edge is required under pnpm.
  ensureDirectViteForPnpm(pkg, packageManager, supportCatalog, catalogDependencyResolver);

  return before !== JSON.stringify(pkg);
}

export function bootstrapProjectPaths(
  rootDir: string,
  packages: WorkspacePackage[] | undefined,
): string[] {
  return [rootDir, ...(packages ?? []).map((pkg) => path.join(rootDir, pkg.path))];
}

export function collectInjectedProviderNames(
  rootDir: string,
  packages?: WorkspacePackage[],
  // Optional precomputed provider source-scan results keyed by absolute package
  // path. Lets a caller that already scanned a path reuse the result instead of
  // re-traversing the source tree; unknown paths fall back to a fresh scan.
  precomputedSourceModes?: ReadonlyMap<string, Record<string, boolean>>,
): Set<string> {
  const names = new Set<string>();
  for (const packagePath of bootstrapProjectPaths(rootDir, packages)) {
    const packageJsonPath = path.join(packagePath, 'package.json');
    if (!fs.existsSync(packageJsonPath)) {
      continue;
    }
    const pkg = readJsonFile(packageJsonPath) as BootstrapPackageJson;
    const sourceModes =
      precomputedSourceModes?.get(packagePath) ?? collectProviderSourceModes(packagePath);
    const installGroups = [pkg.devDependencies, pkg.dependencies, pkg.optionalDependencies];
    const dependencyGroups = [...installGroups, pkg.peerDependencies];
    for (const provider of OPT_IN_BROWSER_PROVIDERS) {
      const used =
        sourceModes[provider] ||
        dependencyGroups.some((dependencies) => dependencies?.[provider] !== undefined);
      const installed = installGroups.some(
        (dependencies) => dependencies?.[provider] !== undefined,
      );
      if (used && !installed) {
        names.add(provider);
      }
    }
  }
  return names;
}

// Test a predicate against every bootstrap project package.json (root +
// workspace packages), short-circuiting on the first match. Missing manifests
// are skipped. Shared scaffold for the workspace-wide catalog pending checks: a
// pnpm workspace is only satisfied once EVERY package's catalog reference
// resolves to the target version, not just the root manifest's.
function someBootstrapProjectPackageJson(
  rootDir: string,
  packages: WorkspacePackage[] | undefined,
  predicate: (pkg: BootstrapPackageJson) => boolean,
): boolean {
  return bootstrapProjectPaths(rootDir, packages).some((packagePath) => {
    const packageJsonPath = path.join(packagePath, 'package.json');
    if (!fs.existsSync(packageJsonPath)) {
      return false;
    }
    return predicate(readJsonFile(packageJsonPath));
  });
}

function workspaceVitestEcosystemCatalogReferencesPending(
  rootDir: string,
  packages: WorkspacePackage[] | undefined,
  catalogDependencyResolver?: CatalogDependencyResolver,
): boolean {
  return someBootstrapProjectPackageJson(rootDir, packages, (pkg) =>
    vitestEcosystemCatalogReferencesPending(pkg, catalogDependencyResolver),
  );
}

// A non-root package can pin a different named catalog (e.g. `catalog:legacy`)
// whose entry still points at an old Vite+, so checking the root alone lets a
// stale workspace slip through the "already on Vite+" early exit.
function workspaceCatalogVitePlusDependencyPending(
  rootDir: string,
  packages: WorkspacePackage[] | undefined,
  catalogDependencyResolver?: CatalogDependencyResolver,
): boolean {
  return someBootstrapProjectPackageJson(rootDir, packages, (pkg) =>
    catalogVitePlusDependencyPending(pkg, catalogDependencyResolver),
  );
}

// True when an existing Vite+ Yarn monorepo still needs the workspace-hoisting
// opt-out that `ensureVitePlusBootstrap` writes. Mirrors the silent auto-fix in
// `applyYarnWorkspaceHoistingFix`: under `node-modules` + `nmHoistingLimits:
// workspaces`, a non-root workspace that depends on `vite-plus` (which bundles
// vitest) but lacks an explicit `installConfig.hoistingLimits` keeps its own split
// `@vitest/runner` copy unless opted out to `none`. Only the silently-fixable
// layout is reported pending; the warn-only layouts (root `dependencies`, or an
// explicit per-workspace limit) cannot be fixed from package.json, so flagging
// them would make the bootstrap pending forever.
function yarnWorkspaceHoistingOptOutPending(
  rootDir: string,
  packageManager: PackageManager,
  packages: WorkspacePackage[] | undefined,
): boolean {
  if (packageManager !== PackageManager.yarn || !packages?.length) {
    return false;
  }
  const hoisting = findYarnWorkspaceHoisting(rootDir);
  if (!hoisting || hoisting.nodeLinker !== 'node-modules' || hoisting.limit !== 'workspaces') {
    return false;
  }
  return packages.some((workspacePackage) => {
    const packagePath = path.join(rootDir, workspacePackage.path);
    if (path.resolve(packagePath) === hoisting.rootDir) {
      return false;
    }
    const childPackageJsonPath = path.join(packagePath, 'package.json');
    if (!fs.existsSync(childPackageJsonPath)) {
      return false;
    }
    const childPkg = readJsonFile(childPackageJsonPath) as BootstrapPackageJson & {
      installConfig?: { hoistingLimits?: string };
    };
    return (
      hasDirectVitePlusInstallEntry(childPkg) &&
      childPkg.installConfig?.hoistingLimits === undefined
    );
  });
}

export function detectVitePlusBootstrapPending(
  projectPath: string,
  packageManager: PackageManager | undefined,
  packages?: WorkspacePackage[],
  packageManagerVersion?: string,
): boolean {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return false;
  }
  const pkg = readJsonFile(packageJsonPath) as BootstrapPackageJson & {
    workspaces?: NpmWorkspaces;
    catalog?: Record<string, string>;
    catalogs?: Record<string, Record<string, string>>;
  };

  // vite-plus counts as installed when it's a direct dependency/devDependency,
  // so a project that declares it in `dependencies` isn't reported as pending a
  // (duplicate) devDependencies entry.
  if (!hasDirectVitePlusInstallEntry(pkg) || !hasPackageManagerPin(pkg)) {
    return true;
  }

  if (packageManager === undefined) {
    return true;
  }

  const resolvedPackageManagerVersion =
    packageManagerVersion ?? pinnedPackageManagerVersion(pkg) ?? '';
  const usePnpmWorkspaceYaml =
    packageManager === PackageManager.pnpm &&
    pnpmSupportsWorkspaceSettings(resolvedPackageManagerVersion);
  if (usePnpmWorkspaceYaml && pnpmPackageJsonSettingsPending(pkg)) {
    return true;
  }
  const supportCatalog =
    !VITE_PLUS_VERSION.startsWith('file:') &&
    supportsCatalog(
      packageManager,
      resolvedPackageManagerVersion,
      packageManager === PackageManager.bun && bunWorkspaceDeclaresPackages(pkg.workspaces),
    );
  const catalogDependencyResolver = createCatalogDependencyResolver(projectPath, packageManager);
  const canonicalVitePlusSpec = supportCatalog
    ? (catalogDependencyResolver?.preferredCatalogSpec ?? 'catalog:')
    : VITE_PLUS_VERSION;
  if (
    workspaceVitestEcosystemCatalogReferencesPending(
      projectPath,
      packages,
      catalogDependencyResolver,
    )
  ) {
    return true;
  }
  // Match the actual reconcile so an already-installed provider whose catalog
  // entry the migration will add is reported pending (its spec must become
  // `catalog:`). See #2005.
  const providerCatalogAdditions = collectInjectedProviderNames(projectPath, packages);
  for (const [index, packagePath] of bootstrapProjectPaths(projectPath, packages).entries()) {
    const childPackageJsonPath = path.join(packagePath, 'package.json');
    if (!fs.existsSync(childPackageJsonPath)) {
      continue;
    }
    const childPkg = readJsonFile(childPackageJsonPath) as BootstrapPackageJson;
    const candidate = JSON.parse(JSON.stringify(childPkg)) as BootstrapPackageJson;
    if (
      reconcileVitePlusBootstrapPackage(
        packagePath,
        candidate,
        canonicalVitePlusSpec,
        packageManager,
        supportCatalog,
        index === 0,
        catalogDependencyResolver,
        providerCatalogAdditions,
      )
    ) {
      return true;
    }
  }

  // Shared override/catalog sinks must keep vitest managed when any package in
  // the workspace needs it. The direct dependency itself is localized above.
  const usesVitest = workspaceUsesVitestDirectly(projectPath, packages, true);

  if (packageManager === PackageManager.yarn) {
    return (
      !overridesSatisfyVitePlus(pkg.resolutions, usesVitest) ||
      !yarnrcSatisfiesVitePlus(projectPath, usesVitest, supportCatalog) ||
      yarnWorkspaceHoistingOptOutPending(projectPath, packageManager, packages)
    );
  }
  if (packageManager === PackageManager.npm) {
    return (
      vitePlusDependencyNeedsConcreteVersion(pkg) ||
      !overridesSatisfyVitePlus(pkg.overrides, usesVitest) ||
      npmVitePlusManagedDependenciesPending(pkg, usesVitest)
    );
  }
  if (packageManager === PackageManager.bun) {
    // A standalone bun project (`supportCatalog === false`) cannot resolve
    // `catalog:` references, so the catalog resolver must NOT be consulted —
    // otherwise a dangling `catalog:` override resolves through the dead catalog
    // field and is mistaken for satisfied, masking the pending rewrite.
    return !overridesSatisfyVitePlus(
      pkg.overrides,
      usesVitest,
      supportCatalog ? readBunCatalogDependencyResolver(pkg) : undefined,
    );
  }
  if (packageManager === PackageManager.pnpm) {
    if (!pnpmWorkspaceExoticSubdepsSettingSatisfied(projectPath)) {
      return true;
    }
    if (!usePnpmWorkspaceYaml) {
      // pnpm 9.5–10.6.1: settings stay in package.json but catalogs resolve,
      // and the reconcile deliberately keeps `catalog:` toolchain specs and
      // overrides. Judge them through the workspace catalog instead of
      // flagging every `catalog:` spec, or a fully migrated project would
      // never converge to the "already using Vite+" fast exit.
      if (supportCatalog) {
        return (
          catalogVitePlusDependencyPending(pkg, catalogDependencyResolver) ||
          !overridesSatisfyVitePlus(pkg.pnpm?.overrides, usesVitest, catalogDependencyResolver) ||
          !pnpmPeerDependencyRulesSatisfyVitePlus(pkg.pnpm?.peerDependencyRules, usesVitest)
        );
      }
      return (
        vitePlusDependencyNeedsConcreteVersion(pkg) ||
        !overridesSatisfyVitePlus(pkg.pnpm?.overrides, usesVitest) ||
        !pnpmPeerDependencyRulesSatisfyVitePlus(pkg.pnpm?.peerDependencyRules, usesVitest)
      );
    }
    const resolver = readPnpmWorkspaceCatalogDependencyResolver(projectPath);
    return (
      workspaceCatalogVitePlusDependencyPending(projectPath, packages, resolver) ||
      !overridesSatisfyVitePlus(readPnpmWorkspaceOverrides(projectPath), usesVitest, resolver) ||
      !pnpmPeerDependencyRulesSatisfyVitePlus(
        readPnpmWorkspacePeerDependencyRules(projectPath),
        usesVitest,
      ) ||
      pnpmWorkspaceMinimumReleaseAgeExemptionsPending(projectPath)
    );
  }

  return false;
}

// vite-plus counts as already installed when it lives directly in
// `dependencies` OR `devDependencies`. `optionalDependencies` is deliberately
// excluded: an optional-only entry may be skipped at install time, so the
// package should still receive a guaranteed `devDependencies` entry.
export function hasDirectVitePlusInstallEntry(pkg: {
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
}): boolean {
  return (
    pkg.dependencies?.[VITE_PLUS_NAME] !== undefined ||
    pkg.devDependencies?.[VITE_PLUS_NAME] !== undefined
  );
}

// `collectToolchainVersionChanges` mirrors this pinning rule (the `vitePlusChanges`
// predicate) to decide whether the summary shows a vite-plus version row; keep the
// two in sync.
function ensureVitePlusDependencySpecs(
  pkg: BootstrapPackageJson,
  version: string,
  ensurePresent = true,
): boolean {
  let changed = false;
  // Re-pin a pre-existing vite-plus spec to the migrating toolchain target so
  // the lockfile moves off an old resolution (e.g. `^0.1.24`). Mirrors the
  // full-migration rule at `shouldNormalizeExistingVitePlus`/`canonicalVitePlusSpec`:
  // only vanilla version ranges are rewritten; deliberate protocol pins
  // (workspace:, link:, file:, npm:, github:, git, http) are preserved.
  const dependencyGroups = [pkg.devDependencies, pkg.dependencies, pkg.optionalDependencies];
  for (const dependencies of dependencyGroups) {
    if (dependencies === undefined) {
      continue;
    }
    const spec = dependencies[VITE_PLUS_NAME];
    if (spec === undefined || spec === version) {
      continue;
    }
    // Catalog writers update every existing managed entry in place. Keep a
    // package's deliberate named/default reference instead of collapsing all
    // packages onto the workspace's preferred catalog, including preview
    // force-override runs.
    if (version.startsWith('catalog:') && spec.startsWith('catalog:')) {
      continue;
    }
    // Concrete target (e.g. `latest`): also rewrite an existing `catalog:`
    // pin onto the concrete version — `isProtocolPinnedSpec` matches
    // `catalog:`, so handle it explicitly before the generic plain-range case.
    if (!version.startsWith('catalog:') && spec.startsWith('catalog:')) {
      dependencies[VITE_PLUS_NAME] = version;
      changed = true;
      continue;
    }
    // Plain (non-protocol-pinned) range like `^0.1.24` → rewrite to the target
    // (`catalog:` for catalog-supporting projects, otherwise the concrete
    // version). Already-`catalog:` / other protocol pins are left untouched,
    // except in force-override mode where ecosystem/preview validation must
    // replace every prior target with the requested artifact.
    if (isForceOverrideMode() || !isProtocolPinnedSpec(spec)) {
      dependencies[VITE_PLUS_NAME] = version;
      changed = true;
    }
  }
  if (hasDirectVitePlusInstallEntry(pkg) || !ensurePresent) {
    return changed;
  }
  pkg.devDependencies = {
    ...pkg.devDependencies,
    [VITE_PLUS_NAME]: version,
  };
  return true;
}

// Raw file contents, or undefined if the file does not exist. Used for
// byte-level before/after change detection on pnpm-workspace.yaml / .yarnrc.yml.
function readTextFileIfExists(filePath: string): string | undefined {
  return fs.existsSync(filePath) ? fs.readFileSync(filePath, 'utf-8') : undefined;
}

function ensureOverrideEntries(
  overrides: Record<string, string> | undefined,
  usesVitest: boolean,
  catalogDependencyResolver?: CatalogDependencyResolver,
): { overrides: Record<string, string>; changed: boolean } {
  const next = { ...overrides };
  let changed = false;
  // Common case: drop a lingering managed `vitest` override.
  if (!usesVitest && removeManagedVitestEntry(next)) {
    changed = true;
  }
  for (const [dependencyName, overrideSpec] of Object.entries(
    managedOverridePackages(usesVitest),
  )) {
    if (
      !overrideSpecSatisfiesVitePlus(
        dependencyName,
        next[dependencyName],
        catalogDependencyResolver,
      )
    ) {
      next[dependencyName] = overrideSpec;
      changed = true;
    }
  }
  return { overrides: next, changed };
}

function ensurePnpmPeerDependencyRules(pkg: BootstrapPackageJson, usesVitest: boolean): boolean {
  const overrideKeys = Object.keys(managedOverridePackages(usesVitest));
  pkg.pnpm ??= {};
  // Common case: drop a lingering managed `vitest` peer rule from the source
  // shape before re-deriving the managed rules.
  const seed = { ...pkg.pnpm.peerDependencyRules } as {
    allowAny?: string[];
    allowedVersions?: Record<string, string>;
  };
  if (!usesVitest && VITEST_IS_MANAGED_OVERRIDE) {
    if (Array.isArray(seed.allowAny)) {
      seed.allowAny = seed.allowAny.filter((key) => key !== 'vitest');
    }
    if (seed.allowedVersions) {
      seed.allowedVersions = { ...seed.allowedVersions };
      delete seed.allowedVersions.vitest;
    }
  }
  const peerDependencyRules = {
    ...seed,
    allowAny: [...new Set([...(seed.allowAny ?? []), ...overrideKeys])],
    allowedVersions: {
      ...seed.allowedVersions,
      ...Object.fromEntries(overrideKeys.map((key) => [key, '*'])),
    },
  };
  const changed =
    JSON.stringify(pkg.pnpm.peerDependencyRules ?? {}) !== JSON.stringify(peerDependencyRules);
  pkg.pnpm.peerDependencyRules = peerDependencyRules;
  return changed;
}

export function ensureVitePlusBootstrap(
  workspaceInfo: WorkspaceInfo,
  report?: MigrationReport,
): VitePlusBootstrapResult {
  const projectPath = workspaceInfo.rootDir;
  const packageJsonPath = path.join(projectPath, 'package.json');
  const result: VitePlusBootstrapResult = {
    changed: false,
    packageJson: false,
    packageManagerConfig: false,
    packageManagerField: false,
  };
  if (!fs.existsSync(packageJsonPath)) {
    return result;
  }

  // Shared override/catalog sinks are workspace-wide, so keep vitest managed
  // when any package needs it. Each package's direct vitest dependency is
  // reconciled independently below.
  const usesVitest = workspaceUsesVitestDirectly(projectPath, workspaceInfo.packages, true);
  const pnpmMajorVersion = pnpmMajor(workspaceInfo.downloadPackageManager.version);
  const shouldAllowBrowserBuilds = workspaceUsesWebdriverio(projectPath, workspaceInfo.packages);
  const usePnpmWorkspaceYaml =
    workspaceInfo.packageManager === PackageManager.pnpm &&
    pnpmSupportsWorkspaceSettings(workspaceInfo.downloadPackageManager.version);
  // Catalogs only resolve inside a bun WORKSPACE (a root package.json with a
  // non-empty `workspaces`). Gate both the catalog edges (`supportCatalog`) and
  // the `rewriteBunCatalog` sink below on this so a standalone bun project keeps
  // concrete specs and writes no catalog field — otherwise `bun install` aborts
  // with "vite@catalog: failed to resolve".
  const isBunWorkspace =
    workspaceInfo.packageManager === PackageManager.bun &&
    bunWorkspaceDeclaresPackages(
      (readJsonFile(packageJsonPath) as { workspaces?: NpmWorkspaces }).workspaces,
    );
  const supportCatalog =
    !VITE_PLUS_VERSION.startsWith('file:') &&
    supportsCatalog(
      workspaceInfo.packageManager,
      workspaceInfo.downloadPackageManager.version,
      isBunWorkspace,
    );
  const catalogDependencyResolver = createCatalogDependencyResolver(
    projectPath,
    workspaceInfo.packageManager,
  );
  const canonicalVitePlusSpec = supportCatalog
    ? (catalogDependencyResolver?.preferredCatalogSpec ?? 'catalog:')
    : VITE_PLUS_VERSION;
  const ecosystemCatalogReferencesPending = workspaceVitestEcosystemCatalogReferencesPending(
    projectPath,
    workspaceInfo.packages,
    catalogDependencyResolver,
  );
  const vitestEcosystemPackages = collectVitestEcosystemInstallDependencyNames(
    projectPath,
    workspaceInfo.packages,
  );
  const providerCatalogAdditions = collectInjectedProviderNames(
    projectPath,
    workspaceInfo.packages,
  );
  let movedPnpmSettings: Record<string, unknown> | undefined;

  const pnpmWorkspaceYamlPath = path.join(projectPath, 'pnpm-workspace.yaml');
  // Below pnpm 10.6.2 the pnpm settings (overrides) stay in package.json, so the
  // `usePnpmWorkspaceYaml` block below skips the pnpm-workspace.yaml rewrite. But
  // catalogs work from 9.5.0, so rewrite the catalog NOW — before the package.json
  // overrides are reconciled — so a `catalog:` override resolves to the fresh
  // toolchain value (not the stale vite-plus-test wrapper) and stays `catalog:`,
  // and the catalog itself is rewritten off the wrappers. Catalog only
  // (`writeWorkspaceSettings: false`). A missing pnpm-workspace.yaml is created:
  // `canonicalVitePlusSpec` is already `catalog:` whenever `supportCatalog`, so
  // skipping the write would leave the reconciled specs pointing at a catalog
  // that does not exist and the post-migration install unresolvable (mirrors
  // the standalone fresh-migration path).
  // The snapshot doubles as the change baseline for the `else` branch below.
  let pnpmWorkspaceCatalogBefore: string | undefined;
  if (workspaceInfo.packageManager === PackageManager.pnpm && !usePnpmWorkspaceYaml) {
    pnpmWorkspaceCatalogBefore = readTextFileIfExists(pnpmWorkspaceYamlPath);
    if (supportCatalog) {
      rewritePnpmWorkspaceYaml(
        projectPath,
        pnpmMajorVersion,
        shouldAllowBrowserBuilds,
        usesVitest,
        vitestEcosystemPackages,
        false,
        providerCatalogAdditions,
      );
    }
  }

  editJsonFile<
    BootstrapPackageJson & {
      workspaces?: NpmWorkspaces;
      catalog?: Record<string, string>;
      catalogs?: Record<string, Record<string, string>>;
    }
  >(packageJsonPath, (pkg) => {
    let packageJsonChanged = reconcileVitePlusBootstrapPackage(
      projectPath,
      pkg,
      canonicalVitePlusSpec,
      workspaceInfo.packageManager,
      supportCatalog,
      true,
      catalogDependencyResolver,
      providerCatalogAdditions,
    );

    if (workspaceInfo.packageManager === PackageManager.yarn) {
      const ensured = ensureOverrideEntries(pkg.resolutions, usesVitest);
      if (ensured.changed) {
        pkg.resolutions = ensured.overrides;
        packageJsonChanged = true;
      }
    } else if (workspaceInfo.packageManager === PackageManager.npm) {
      const ensured = ensureOverrideEntries(pkg.overrides, usesVitest);
      if (ensured.changed) {
        pkg.overrides = ensured.overrides;
        packageJsonChanged = true;
      }
    } else if (workspaceInfo.packageManager === PackageManager.bun) {
      // Standalone bun (`supportCatalog === false`) has no catalog support, so a
      // `catalog:` override must be rewritten to the concrete core alias rather
      // than left dangling — the catalog resolver is only consulted for a real
      // bun workspace where `rewriteBunCatalog` maintains the catalog field.
      const ensured = ensureOverrideEntries(
        pkg.overrides,
        usesVitest,
        supportCatalog ? readBunCatalogDependencyResolver(pkg) : undefined,
      );
      if (ensured.changed) {
        pkg.overrides = ensured.overrides;
        packageJsonChanged = true;
      }
    } else if (workspaceInfo.packageManager === PackageManager.pnpm && !usePnpmWorkspaceYaml) {
      pkg.pnpm ??= {};
      // Below pnpm 10.6.2 the settings stay here, but catalogs (>= 9.5.0) still
      // work: keep `catalog:` overrides referencing the workspace catalog
      // (rewritten below) instead of inlining them, mirroring the bun branch.
      const ensured = ensureOverrideEntries(
        pkg.pnpm.overrides,
        usesVitest,
        supportCatalog ? readPnpmWorkspaceCatalogDependencyResolver(projectPath) : undefined,
      );
      if (ensured.changed) {
        pkg.pnpm.overrides = ensured.overrides;
        packageJsonChanged = true;
      }
      packageJsonChanged = ensurePnpmPeerDependencyRules(pkg, usesVitest) || packageJsonChanged;
      if (pnpmMajorVersion !== undefined && pkg.pnpm) {
        const beforePnpm = JSON.stringify(pkg.pnpm);
        applyBuildAllowanceToPackageJsonPnpm(pkg.pnpm, pnpmMajorVersion, shouldAllowBrowserBuilds);
        packageJsonChanged = beforePnpm !== JSON.stringify(pkg.pnpm) || packageJsonChanged;
      }
    } else if (workspaceInfo.packageManager === PackageManager.pnpm) {
      const hadPnpmField = pkg.pnpm !== undefined;
      movedPnpmSettings = takePnpmWorkspaceSettings(pkg);
      packageJsonChanged =
        movedPnpmSettings !== undefined ||
        (hadPnpmField && pkg.pnpm === undefined) ||
        packageJsonChanged;
    }

    result.packageJson = packageJsonChanged;
    return pkg;
  });

  // Existing Vite+ monorepos take this bootstrap path instead of the full
  // migration, so reconcile every workspace manifest as well as the root.
  //
  // Yarn `nmHoistingLimits` isolation (resolved from the root `.yarnrc.yml`
  // chain) splits the bundled vitest family per workspace. Mirror the full
  // migration (`rewriteMonorepoProject`) and opt each vite-plus workspace out so
  // its vitest dedupes to the single shared root copy, or warn when the split
  // cannot be fixed from package.json. The root itself never needs an opt-out (its
  // deps already hoist to the top), so `applyYarnWorkspaceHoistingFix` is gated on
  // a non-root workspace that depends on vite-plus.
  const yarnHoisting =
    workspaceInfo.packageManager === PackageManager.yarn
      ? findYarnWorkspaceHoisting(projectPath)
      : undefined;
  for (const workspacePackage of workspaceInfo.packages) {
    const packagePath = path.join(projectPath, workspacePackage.path);
    const childPackageJsonPath = path.join(packagePath, 'package.json');
    if (!fs.existsSync(childPackageJsonPath)) {
      continue;
    }
    let childChanged = false;
    editJsonFile<BootstrapPackageJson & { installConfig?: { hoistingLimits?: string } }>(
      childPackageJsonPath,
      (pkg) => {
        const before = JSON.stringify(pkg);
        reconcileVitePlusBootstrapPackage(
          packagePath,
          pkg,
          canonicalVitePlusSpec,
          workspaceInfo.packageManager,
          supportCatalog,
          false,
          catalogDependencyResolver,
          providerCatalogAdditions,
        );
        if (
          yarnHoisting &&
          path.resolve(packagePath) !== yarnHoisting.rootDir &&
          hasDirectVitePlusInstallEntry(pkg)
        ) {
          applyYarnWorkspaceHoistingFix(
            pkg,
            yarnHoisting.limit,
            yarnHoisting.nodeLinker,
            path.relative(yarnHoisting.rootDir, packagePath) || packagePath,
            report,
          );
        }
        childChanged = before !== JSON.stringify(pkg);
        return childChanged ? pkg : undefined;
      },
    );
    result.packageJson = result.packageJson || childChanged;
  }

  if (workspaceInfo.packageManager === PackageManager.pnpm) {
    if (usePnpmWorkspaceYaml) {
      const before = readTextFileIfExists(pnpmWorkspaceYamlPath);
      migratePnpmSettingsToWorkspaceYaml(projectPath, movedPnpmSettings);
      const catalogDependencyResolver = readPnpmWorkspaceCatalogDependencyResolver(projectPath);
      if (
        movedPnpmSettings !== undefined ||
        result.packageJson ||
        ecosystemCatalogReferencesPending ||
        !pnpmWorkspaceExoticSubdepsSettingSatisfied(projectPath) ||
        pnpmWorkspaceMinimumReleaseAgeExemptionsPending(projectPath) ||
        workspaceCatalogVitePlusDependencyPending(
          projectPath,
          workspaceInfo.packages,
          catalogDependencyResolver,
        ) ||
        !overridesSatisfyVitePlus(
          readPnpmWorkspaceOverrides(projectPath),
          usesVitest,
          catalogDependencyResolver,
        ) ||
        !pnpmPeerDependencyRulesSatisfyVitePlus(
          readPnpmWorkspacePeerDependencyRules(projectPath),
          usesVitest,
        )
      ) {
        rewritePnpmWorkspaceYaml(
          projectPath,
          pnpmMajorVersion,
          shouldAllowBrowserBuilds,
          usesVitest,
          vitestEcosystemPackages,
          true,
          providerCatalogAdditions,
        );
      }
      if (fs.existsSync(pnpmWorkspaceYamlPath)) {
        ensurePnpmWorkspacePackages(projectPath, workspaceInfo.workspacePatterns);
      }
      const after = readTextFileIfExists(pnpmWorkspaceYamlPath);
      result.packageManagerConfig = before !== after;
    } else {
      // The catalog was already rewritten before the reconcile (above). Only the
      // exotic-subdeps setting and packages field remain; track the net change
      // against the pre-rewrite snapshot.
      const exoticChanged = ensurePnpmWorkspaceExoticSubdepsSetting(projectPath);
      if (fs.existsSync(pnpmWorkspaceYamlPath)) {
        ensurePnpmWorkspacePackages(projectPath, workspaceInfo.workspacePatterns);
      }
      const after = readTextFileIfExists(pnpmWorkspaceYamlPath);
      result.packageManagerConfig = exoticChanged || pnpmWorkspaceCatalogBefore !== after;
    }
  } else if (workspaceInfo.packageManager === PackageManager.yarn) {
    const yarnrcYmlPath = path.join(projectPath, '.yarnrc.yml');
    const before = readTextFileIfExists(yarnrcYmlPath);
    rewriteYarnrcYml(
      projectPath,
      usesVitest,
      vitestEcosystemPackages,
      providerCatalogAdditions,
      supportCatalog,
    );
    const after = fs.readFileSync(yarnrcYmlPath, 'utf-8');
    result.packageManagerConfig = before !== after;
  } else if (isBunWorkspace) {
    // Only a bun WORKSPACE writes a catalog (matching the fresh path, where
    // rewriteBunCatalog runs only on monorepo roots). A standalone bun project
    // keeps the concrete overrides set by `ensureOverrideEntries` above.
    const before = fs.readFileSync(packageJsonPath, 'utf-8');
    rewriteBunCatalog(projectPath, usesVitest, vitestEcosystemPackages);
    const after = fs.readFileSync(packageJsonPath, 'utf-8');
    result.packageJson = result.packageJson || before !== after;
  }

  const beforePackageManager = fs.readFileSync(packageJsonPath, 'utf-8');
  setPackageManager(projectPath, workspaceInfo.downloadPackageManager);
  const afterPackageManager = fs.readFileSync(packageJsonPath, 'utf-8');
  result.packageManagerField = beforePackageManager !== afterPackageManager;
  result.changed = result.packageJson || result.packageManagerConfig || result.packageManagerField;
  if (result.changed && report) {
    report.packageManagerBootstrapConfigured = true;
  }
  return result;
}

type RawVitePackageJson = {
  name?: string;
  version?: string;
  bundledVersions?: { vite?: string };
};

function declaredRootSpec(pkg: BootstrapPackageJson, dependencyName: string): string | undefined {
  return pkg.dependencies?.[dependencyName] ?? pkg.devDependencies?.[dependencyName];
}

/**
 * Reduce a plain spec to a concrete-ish version string for display: a
 * `npm:@scope/name@<range>` alias (e.g. the legacy `@voidzero-dev/vite-plus-test`
 * vitest wrapper) keeps only its trailing range, then a single leading `^`/`~`/
 * `>=` operator is stripped. Any remaining range text is kept as-is.
 */
function concretizeRange(spec: string): string {
  let value = spec;
  if (value.startsWith('npm:')) {
    const versionAt = value.lastIndexOf('@');
    if (versionAt > 'npm:'.length) {
      value = value.slice(versionAt + 1);
    }
  }
  return value.replace(/^(?:\^|~|>=)\s*/, '');
}

/**
 * Resolve a pre-migration root dependency spec to a concrete version string for
 * display. A `catalog:`/`catalog:<name>` reference resolves through the project
 * catalog (then concretized, since a named catalog may itself hold an alias).
 */
function resolveDisplayFromSpec(
  spec: string,
  dependencyName: string,
  catalogDependencyResolver: CatalogDependencyResolver | undefined,
): string | undefined {
  if (spec.startsWith('catalog:')) {
    const resolved = catalogDependencyResolver?.(spec, dependencyName);
    return resolved === undefined ? undefined : concretizeRange(resolved);
  }
  return concretizeRange(spec);
}

/**
 * Read the RAW upstream Vite version installed under the project's own
 * `node_modules/vite`, best-effort. This is a deliberate project-local read (not
 * an ancestor-walking resolve) so a parent directory's vite cannot leak in as
 * the project's "from". When that copy is the `@voidzero-dev/vite-plus-core`
 * alias (the Vite+ bundle), the raw Vite version lives in its
 * `bundledVersions.vite`; otherwise it is a real upstream vite and `version` is
 * the raw value. Returns undefined when the file is missing (e.g. a cleared
 * install or Yarn PnP) or yields nothing.
 */
function readInstalledRawViteVersion(projectPath: string): string | undefined {
  const vitePackageJsonPath = path.join(projectPath, 'node_modules', 'vite', 'package.json');
  if (!fs.existsSync(vitePackageJsonPath)) {
    return undefined;
  }
  let pkgJson: RawVitePackageJson;
  try {
    pkgJson = readJsonFile(vitePackageJsonPath);
  } catch {
    return undefined;
  }
  if (pkgJson.name === '@voidzero-dev/vite-plus-core') {
    return pkgJson.bundledVersions?.vite;
  }
  return pkgJson.version;
}

/**
 * Capture the toolchain dependency version changes an existing-Vite+ upgrade
 * will apply, for the migrate summary table. Call this BEFORE the bootstrap
 * reconcile mutates the manifest so the `from` values still reflect the
 * pre-migration root package.json.
 *
 * `to` targets: `vite-plus` -> VITE_PLUS_VERSION, `vitest` and every declared
 * `@vitest/*` -> VITEST_VERSION, `vite` -> the RAW bundled upstream Vite version
 * (NOT the `@voidzero-dev/vite-plus-core` alias). An entry is included only when
 * `to` is defined and the version actually changes (or the package is freshly
 * added, i.e. `from` is undefined).
 */
export async function collectToolchainVersionChanges(
  projectPath: string,
): Promise<DependencyVersionChange[]> {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return [];
  }
  const pkg = readJsonFile(packageJsonPath) as BootstrapPackageJson;

  // The bundled raw Vite version is imported from `./versions.js` (dist root):
  // the tsdown `fix-versions-path` plugin rewrites this specifier and
  // versions.spec.ts guards that it resolves. Mirror migrator/eslint.ts EXACTLY.
  // @ts-expect-error — resolved at runtime from dist/ -> dist/versions.js
  const { versions } = await import('../versions.js');
  const rawViteVersion = versions.vite as string | undefined;

  const catalogDependencyResolver =
    readPnpmWorkspaceCatalogDependencyResolver(projectPath) ??
    createCatalogDependencyResolver(projectPath, PackageManager.yarn) ??
    createCatalogDependencyResolver(projectPath, PackageManager.bun);

  const fromFor = (dependencyName: string): string | undefined => {
    const spec = declaredRootSpec(pkg, dependencyName);
    if (spec === undefined) {
      return undefined;
    }
    return resolveDisplayFromSpec(spec, dependencyName, catalogDependencyResolver);
  };

  const changes: DependencyVersionChange[] = [];
  const pushChange = (name: string, to: string | undefined, from: string | undefined): void => {
    if (to === undefined || (from !== undefined && from === to)) {
      return;
    }
    changes.push(from === undefined ? { name, to } : { name, from, to });
  };

  // A deliberate NON-catalog protocol pin (file:/link:/workspace:/npm:/git/http)
  // is preserved by ensureVitePlusDependencySpecs, so it is not a version change
  // unless force-override mode rewrites every spec. A catalog ref or plain range
  // is re-pinned to the toolchain target.
  const vitePlusSpec = declaredRootSpec(pkg, VITE_PLUS_NAME);
  const vitePlusChanges =
    vitePlusSpec === undefined ||
    vitePlusSpec.startsWith('catalog:') ||
    isForceOverrideMode() ||
    !isProtocolPinnedSpec(vitePlusSpec);
  if (vitePlusChanges) {
    pushChange(VITE_PLUS_NAME, VITE_PLUS_VERSION, fromFor(VITE_PLUS_NAME));
  }
  pushChange('vite', rawViteVersion, readInstalledRawViteVersion(projectPath));
  // A declared-but-unused bare `vitest` is removed (not bumped) by the reconcile,
  // so only show it when the root actually uses vitest (the same signal the
  // reconcile uses to keep and align it).
  if (
    declaredRootSpec(pkg, 'vitest') !== undefined &&
    projectUsesVitestDirectly(projectPath, pkg, undefined, true)
  ) {
    pushChange('vitest', VITEST_VERSION, fromFor('vitest'));
  }
  // Only the `@vitest/*` packages the migrator actually aligns to the bundled
  // vitest version belong here. `@vitest/eslint-plugin` and `@vitest/coverage-c8`
  // version independently and are left untouched, so they are not "changes".
  const scopedVitestNames = new Set<string>();
  for (const group of [pkg.dependencies, pkg.devDependencies]) {
    for (const name of Object.keys(group ?? {})) {
      if (isAlignableVitestEcosystemPackage(name)) {
        scopedVitestNames.add(name);
      }
    }
  }
  for (const name of [...scopedVitestNames].toSorted()) {
    pushChange(name, VITEST_VERSION, fromFor(name));
  }

  return changes;
}
