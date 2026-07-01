import fs from 'node:fs';
import path from 'node:path';

import semver from 'semver';
import { Scalar, YAMLMap, YAMLSeq } from 'yaml';

import { PackageManager, type WorkspacePackage } from '../../types/index.ts';
import {
  VITEST_AGE_GATE_EXEMPT_PACKAGES,
  VITEST_VERSION,
  VITE_PLUS_NAME,
  VITE_PLUS_OVERRIDE_PACKAGES,
  VITE_PLUS_VERSION,
  isForceOverrideMode,
} from '../../utils/constants.ts';
import { editJsonFile, readJsonFile } from '../../utils/json.ts';
import { type NpmWorkspaces } from '../../utils/workspace.ts';
import { editYamlFile, readYamlFile, scalarString, type YamlDocument } from '../../utils/yaml.ts';
import {
  dropRemovePackageOverrideKeys,
  ensurePnpmExoticSubdepsSetting,
  hasDirectVitePlusInstallEntry,
  isAlignableVitestEcosystemPackage,
  isLegacyWrapperSpec,
  managedOverridePackages,
  pruneLegacyWrapperAliases,
  removeManagedVitestEntry,
  removeVitestPeerDependencyRule,
  removeYamlMapVitestEntry,
  rewriteMonorepoProject,
  shouldDropProviderOverrideKey,
} from '../migrator.ts';
import {
  LEGACY_WRAPPER_FALLBACK_VERSIONS,
  PROVIDER_OVERRIDE_DROP_NAMES,
  REMOVE_PACKAGES,
  VITEST_IS_MANAGED_OVERRIDE,
  isPlainRecord,
  type CatalogDependencyResolver,
  type PackageJsonDependencyField,
  type PnpmPackageJsonSettings,
} from './shared.ts';

// Transitive packages with postinstall scripts that vite-plus's deps drag in
// via `@vitest/browser-webdriverio` → `webdriverio` → `@wdio/utils`. pnpm v10
// refuses to run these without explicit approval, so `vp migrate` records the
// allow/deny decision up front: deny by default (the user isn't using
// webdriverio), allow when the user actually depends on webdriverio.
const BROWSER_PROVIDER_POSTINSTALL_PACKAGES = ['edgedriver', 'geckodriver'] as const;

const PUBLIC_PEER_DEPENDENCY_FALLBACKS: Record<string, string> = {
  vite: '*',
  vitest: '*',
};

// Package-name patterns the migrator exempts from pnpm's `minimumReleaseAge`
// gate. Vite+ pins `vitest` to an exact (sometimes freshly published) version and
// the in-tree @vitest/* siblings install transitively at that version, so the age
// gate would otherwise quarantine the Vite+-managed family and break `vp install`.
// Shared between the writer (`rewritePnpmWorkspaceYaml`) and the pending check
// (`pnpmWorkspaceMinimumReleaseAgeExemptionsPending`) so they cannot drift.
const PNPM_MINIMUM_RELEASE_AGE_EXCLUDES = [
  'vite-plus',
  '@voidzero-dev/*',
  'oxlint',
  '@oxlint/*',
  'oxlint-tsgolint',
  '@oxlint-tsgolint/*',
  'oxfmt',
  '@oxfmt/*',
  ...VITEST_AGE_GATE_EXEMPT_PACKAGES,
] as const;

const PNPM_WORKSPACE_SETTINGS_MIN_VERSION = '10.6.2';

// pnpm 10.5 started reading package.json#pnpm settings from
// pnpm-workspace.yaml, but overrides and peerDependencyRules needed fixes in
// 10.5.1 and 10.6.2 respectively. Use the latter as the atomic migration
// boundary so the complete object can move without splitting its ownership.
export function pnpmSupportsWorkspaceSettings(version: string): boolean {
  const coerced = semver.coerce(version);
  if (coerced) {
    return semver.gte(coerced, PNPM_WORKSPACE_SETTINGS_MIN_VERSION);
  }
  return version === 'latest' || version === 'next';
}

const PNPM_CATALOG_MIN_VERSION = '9.5.0';

// pnpm catalogs (the `catalog:` protocol and the pnpm-workspace.yaml
// `catalog`/`catalogs` fields) shipped in pnpm 9.5.0 as a minor change ("Added
// support for catalogs", https://github.com/pnpm/pnpm/releases/tag/v9.5.0), and
// every release from 9.5.0 onward supports them. This is a SEPARATE, EARLIER
// feature than moving package.json#pnpm settings into pnpm-workspace.yaml
// (`pnpmSupportsWorkspaceSettings`, 10.6.2). `supportCatalog` must gate on THIS,
// not on workspace-settings support: otherwise a pnpm 9.5–10.6.1 project that
// already uses catalogs has its reconciled toolchain edges (vite/vite-plus and
// the vitest ecosystem) inlined to concrete versions instead of kept `catalog:`.
export function pnpmSupportsCatalog(version: string): boolean {
  const coerced = semver.coerce(version);
  if (coerced) {
    return semver.gte(coerced, PNPM_CATALOG_MIN_VERSION);
  }
  return version === 'latest' || version === 'next';
}

const YARN_CATALOG_MIN_VERSION = '4.10.0';

// Yarn's `catalog:` protocol (and the `.yarnrc.yml` `catalog`/`catalogs` fields)
// ships ENABLED BY DEFAULT only from Yarn 4.10.0. `vp migrate` auto-upgrades an
// in-range Yarn (`>=4.0.0 <4.10.0`) to the latest stable, but the version this
// helper receives is the RECORDED (pre-upgrade) `downloadPackageManager.version`,
// and an older Yarn (1.x/2.x/3.x) is left untouched. Gating catalog emission on
// the recorded version keeps the migration safe in every case: a project that is
// not (yet) provably on a catalog-capable Yarn gets concrete specs instead of
// `catalog:` references it cannot resolve. Mirrors `pnpmSupportsWorkspaceSettings`.
export function yarnSupportsCatalog(version: string): boolean {
  const coerced = semver.coerce(version);
  if (coerced) {
    return semver.gte(coerced, YARN_CATALOG_MIN_VERSION);
  }
  return version === 'latest' || version === 'next' || version === 'stable';
}

// Whether a `catalog:` reference resolves for this package manager and version:
// pnpm >= 9.5.0, Yarn >= 4.10.0, bun only inside a workspace, npm never. The
// force-override `file:` guard is layered on by the bootstrap callers.
export function supportsCatalog(
  packageManager: PackageManager,
  version: string,
  isBunWorkspace = false,
): boolean {
  switch (packageManager) {
    case PackageManager.pnpm:
      return pnpmSupportsCatalog(version);
    case PackageManager.yarn:
      return yarnSupportsCatalog(version);
    case PackageManager.bun:
      return isBunWorkspace;
    default:
      return false;
  }
}

// These are the root package.json#pnpm settings pnpm 10.6.2+ accepts at the
// top level of pnpm-workspace.yaml. Unknown keys may belong to third-party
// tooling and stay in package.json.
const PNPM_WORKSPACE_SETTING_KEYS = [
  'allowNonAppliedPatches',
  'allowBuilds',
  'allowUnusedPatches',
  'allowedDeprecatedVersions',
  'auditConfig',
  'configDependencies',
  'executionEnv',
  'ignorePatchFailures',
  'ignoredBuiltDependencies',
  'ignoredOptionalDependencies',
  'neverBuiltDependencies',
  'onlyBuiltDependencies',
  'onlyBuiltDependenciesFile',
  'overrides',
  'packageExtensions',
  'patchedDependencies',
  'peerDependencyRules',
  'requiredScripts',
  'supportedArchitectures',
  'updateConfig',
] as const;

function hasPnpmWorkspaceSettings(pkg: { pnpm?: PnpmPackageJsonSettings }): boolean {
  return PNPM_WORKSPACE_SETTING_KEYS.some((key) => Object.hasOwn(pkg.pnpm ?? {}, key));
}

export function pnpmPackageJsonSettingsPending(pkg: { pnpm?: PnpmPackageJsonSettings }): boolean {
  return (
    hasPnpmWorkspaceSettings(pkg) || (pkg.pnpm !== undefined && Object.keys(pkg.pnpm).length === 0)
  );
}

export function takePnpmWorkspaceSettings(pkg: {
  pnpm?: PnpmPackageJsonSettings;
}): Record<string, unknown> | undefined {
  if (!pkg.pnpm) {
    return undefined;
  }
  const settings: Record<string, unknown> = {};
  for (const key of PNPM_WORKSPACE_SETTING_KEYS) {
    if (!Object.hasOwn(pkg.pnpm, key)) {
      continue;
    }
    settings[key] = pkg.pnpm[key];
    delete pkg.pnpm[key];
  }
  if (Object.keys(pkg.pnpm).length === 0) {
    delete pkg.pnpm;
  }
  return Object.keys(settings).length > 0 ? settings : undefined;
}

/**
 * Preserve workspace-level siblings while moving the effective package.json
 * pnpm settings into pnpm-workspace.yaml. Package values win at scalar leaves,
 * while objects merge recursively and arrays retain unique entries from both
 * locations.
 */
function mergePnpmWorkspaceSetting(existing: unknown, incoming: unknown): unknown {
  if (Array.isArray(existing) && Array.isArray(incoming)) {
    const seen = new Set<string | undefined>();
    return [...existing, ...incoming].filter((value) => {
      const key = JSON.stringify(value);
      if (seen.has(key)) {
        return false;
      }
      seen.add(key);
      return true;
    });
  }
  if (isPlainRecord(existing) && isPlainRecord(incoming)) {
    const merged: Record<string, unknown> = { ...existing };
    for (const [key, value] of Object.entries(incoming)) {
      merged[key] = Object.hasOwn(existing, key)
        ? mergePnpmWorkspaceSetting(existing[key], value)
        : value;
    }
    return merged;
  }
  return incoming;
}

export function migratePnpmSettingsToWorkspaceYaml(
  projectPath: string,
  settings: Record<string, unknown> | undefined,
): void {
  if (!settings || Object.keys(settings).length === 0) {
    return;
  }
  const pnpmWorkspaceYamlPath = path.join(projectPath, 'pnpm-workspace.yaml');
  if (!fs.existsSync(pnpmWorkspaceYamlPath)) {
    fs.writeFileSync(pnpmWorkspaceYamlPath, '');
  }
  editYamlFile(pnpmWorkspaceYamlPath, (doc) => {
    const workspace = (doc.toJS() ?? {}) as Record<string, unknown>;
    for (const [key, value] of Object.entries(settings)) {
      // package.json#pnpm was the effective source before migration. Preserve
      // that precedence at conflicting leaves while retaining workspace-only
      // object properties and array entries.
      doc.set(key, doc.createNode(mergePnpmWorkspaceSetting(workspace[key], value)));
    }
  });
}

/**
 * Rewrite pnpm-workspace.yaml to add vite-plus dependencies
 * @param projectPath - The path to the project
 */
export function rewritePnpmWorkspaceYaml(
  projectPath: string,
  pnpmMajorVersion: number | undefined,
  shouldAllowBrowserBuilds: boolean,
  usesVitest: boolean,
  vitestEcosystemPackages: ReadonlySet<string>,
  writeWorkspaceSettings = true,
  catalogAdditions: ReadonlySet<string> = new Set(),
): void {
  const pnpmWorkspaceYamlPath = path.join(projectPath, 'pnpm-workspace.yaml');
  if (!fs.existsSync(pnpmWorkspaceYamlPath)) {
    fs.writeFileSync(pnpmWorkspaceYamlPath, '');
  }
  const managed = managedOverridePackages(usesVitest);

  editYamlFile(pnpmWorkspaceYamlPath, (doc) => {
    // catalog
    const preferredCatalogSpec = rewriteCatalog(
      doc,
      usesVitest,
      vitestEcosystemPackages,
      catalogAdditions,
    );
    if (!writeWorkspaceSettings) {
      return;
    }

    ensurePnpmExoticSubdepsSetting(doc);
    if (pnpmMajorVersion !== undefined) {
      applyBuildAllowanceToWorkspaceYaml(doc, pnpmMajorVersion, shouldAllowBrowserBuilds);
    }

    // overrides
    const overrides = doc.getIn(['overrides']);
    pruneYamlMapLegacyWrapperAliases(overrides);
    // Drop overrides for packages removed by migration (e.g. @vitest/browser*)
    // so a stale workspace pin can't force an incompatible version against
    // vite-plus's own direct dependency. Bare/versioned global pins
    // (`pkg`, `pkg@version`), global-glob selectors (`**/pkg`), and
    // `vite-plus`-parented selectors (`vite-plus>pkg`) all reach vite-plus's own
    // provider dep and are removed. A selector scoped under a SPECIFIC
    // non-vite-plus parent (e.g. `some-app>@vitest/browser-playwright`) only
    // constrains that parent's subtree, so it is preserved — see
    // `shouldDropProviderOverrideKey`.
    if (overrides instanceof YAMLMap) {
      const keysSnapshot = overrides.items.map((item) => item.key);
      for (const keyNode of keysSnapshot) {
        const rawKey =
          keyNode instanceof Scalar ? String(keyNode.value ?? '') : String(keyNode ?? '');
        if (shouldDropProviderOverrideKey(rawKey)) {
          overrides.delete(keyNode);
        }
      }
    }
    // Common case (no direct vitest): actively strip any lingering managed
    // `vitest` override so it arrives transitively through vite-plus.
    if (!usesVitest) {
      removeYamlMapVitestEntry(doc.getIn(['overrides']));
    }
    for (const key of Object.keys(managed)) {
      const currentVersion = getYamlMapScalarStringValue(overrides, key);
      const version = getCatalogDependencySpec(currentVersion, managed[key], true, {
        preferredCatalogSpec,
      });
      doc.setIn(['overrides', scalarString(key)], scalarString(version));
    }
    // remove dependency selector from vite, e.g. "vite-plugin-svgr>vite": "npm:vite@7.0.12"
    // Snapshot the keys before deleting (mirrors the `keysSnapshot` loop above):
    // YAMLMap.delete splices `.items`, so iterating the live array would shift
    // the next entry into the current slot and skip it (two ADJACENT `...>vite`
    // selectors would leave the second behind).
    const updatedOverrides = doc.getIn(['overrides']) as YAMLMap<Scalar<string>, Scalar<string>>;
    const updatedOverrideKeys = updatedOverrides.items.map((item) => item.key);
    for (const key of updatedOverrideKeys) {
      if (key.value.includes('>')) {
        const splits = key.value.split('>');
        if (splits[splits.length - 1].trim() === 'vite') {
          updatedOverrides.delete(key);
        }
      }
    }

    // peerDependencyRules.allowAny
    let allowAny = doc.getIn(['peerDependencyRules', 'allowAny']) as YAMLSeq<Scalar<string>>;
    if (!allowAny) {
      allowAny = new YAMLSeq<Scalar<string>>();
    }
    // Common case: drop any lingering managed `vitest` allowAny entry.
    if (!usesVitest && VITEST_IS_MANAGED_OVERRIDE) {
      allowAny.items = allowAny.items.filter((n) => n.value !== 'vitest');
    }
    const existing = new Set(allowAny.items.map((n) => n.value));
    for (const key of Object.keys(managed)) {
      if (!existing.has(key)) {
        allowAny.add(scalarString(key));
      }
    }
    doc.setIn(['peerDependencyRules', 'allowAny'], allowAny);

    // peerDependencyRules.allowedVersions
    let allowedVersions = doc.getIn(['peerDependencyRules', 'allowedVersions']) as YAMLMap<
      Scalar<string>,
      Scalar<string>
    >;
    if (!allowedVersions) {
      allowedVersions = new YAMLMap<Scalar<string>, Scalar<string>>();
    }
    // Common case: drop any lingering managed `vitest` allowedVersions entry.
    if (!usesVitest) {
      removeYamlMapVitestEntry(allowedVersions);
    }
    for (const key of Object.keys(managed)) {
      // - vite: '*'
      allowedVersions.set(scalarString(key), scalarString('*'));
    }
    doc.setIn(['peerDependencyRules', 'allowedVersions'], allowedVersions);

    // minimumReleaseAgeExclude: exempt the Vite+-managed packages (vite-plus,
    // @voidzero-dev/*, the ox* family, and the vitest family) from the age gate.
    if (doc.has('minimumReleaseAge')) {
      let minimumReleaseAgeExclude = doc.getIn(['minimumReleaseAgeExclude']) as YAMLSeq<
        Scalar<string>
      >;
      if (!minimumReleaseAgeExclude) {
        minimumReleaseAgeExclude = new YAMLSeq();
      }
      const existing = new Set(minimumReleaseAgeExclude.items.map((n) => n.value));
      for (const exclude of PNPM_MINIMUM_RELEASE_AGE_EXCLUDES) {
        if (!existing.has(exclude)) {
          minimumReleaseAgeExclude.add(scalarString(exclude));
        }
      }
      doc.setIn(['minimumReleaseAgeExclude'], minimumReleaseAgeExclude);
    }
  });
}

/**
 * Move remaining non-Vite pnpm.overrides from package.json to pnpm-workspace.yaml.
 * pnpm ignores workspace-level overrides when pnpm.overrides exists in package.json,
 * so all overrides must live in pnpm-workspace.yaml.
 */
export function migratePnpmOverridesToWorkspaceYaml(
  projectPath: string,
  overrides: Record<string, string>,
): void {
  const pnpmWorkspaceYamlPath = path.join(projectPath, 'pnpm-workspace.yaml');
  editYamlFile(pnpmWorkspaceYamlPath, (doc) => {
    for (const [key, value] of Object.entries(overrides)) {
      // Always overwrite: package.json value was the effective one before migration
      // (pnpm ignores workspace overrides when pnpm.overrides exists in package.json)
      doc.setIn(['overrides', scalarString(key)], scalarString(value));
    }
  });
}

export function applyBuildAllowanceToPackageJsonPnpm(
  pnpm: {
    allowBuilds?: Record<string, boolean>;
    onlyBuiltDependencies?: string[];
  },
  major: number,
  shouldAllow: boolean,
): void {
  if (major >= 10) {
    if (shouldAllow) {
      // WebdriverIO present -> the edgedriver/geckodriver postinstall MUST run. Write
      // `true`, OVERWRITING any stale `false` a prior WebdriverIO-less migration left
      // behind (a re-run after adding WebdriverIO would otherwise keep the driver build
      // blocked).
      for (const name of BROWSER_PROVIDER_POSTINSTALL_PACKAGES) {
        (pnpm.allowBuilds ??= {})[name] = true;
      }
    }
    // No WebdriverIO -> vite-plus does NOT manage these postinstalls. edgedriver and
    // geckodriver reach the tree only via the opt-in webdriverio provider (an OPTIONAL
    // peer of both vite-plus and vitest, so pnpm never auto-installs it); a project that
    // does not use it never installs them, so there is nothing to allow or deny. We
    // write nothing and leave any user-authored allowBuilds entry (their own trust
    // decision) untouched.
  } else if (shouldAllow) {
    // v9 onlyBuiltDependencies is an allow-list — omission is denial, so we
    // only mutate when the user actually needs these packages built.
    const list = pnpm.onlyBuiltDependencies ?? [];
    const existing = new Set(list);
    for (const name of BROWSER_PROVIDER_POSTINSTALL_PACKAGES) {
      if (!existing.has(name)) {
        list.push(name);
        existing.add(name);
      }
    }
    pnpm.onlyBuiltDependencies = list;
  }
}

function applyBuildAllowanceToWorkspaceYaml(
  doc: YamlDocument,
  major: number,
  shouldAllow: boolean,
): void {
  if (major >= 10) {
    if (shouldAllow) {
      // WebdriverIO present -> the edgedriver/geckodriver postinstall MUST run. Set
      // `true`, OVERWRITING any stale `false` a prior WebdriverIO-less migration left
      // behind (a re-run after adding WebdriverIO would otherwise keep the driver build
      // blocked). Mutate an existing map in place (preserving its document position);
      // only attach a freshly created one.
      const existing = doc.getIn(['allowBuilds']);
      const isNew = !(existing instanceof YAMLMap);
      const allowBuilds = isNew
        ? new YAMLMap<Scalar<string>, Scalar<boolean>>()
        : (existing as YAMLMap<Scalar<string>, Scalar<boolean>>);
      for (const name of BROWSER_PROVIDER_POSTINSTALL_PACKAGES) {
        allowBuilds.set(scalarString(name), new Scalar(true));
      }
      if (isNew) {
        doc.setIn(['allowBuilds'], allowBuilds);
      }
    }
    // No WebdriverIO -> vite-plus does NOT manage these postinstalls and leaves any
    // user-authored allowBuilds entry untouched (see the package.json sink rationale).
    // The drivers reach the tree only via the opt-in webdriverio provider, so a project
    // that does not use it never installs them and there is nothing to allow or deny.
  } else if (shouldAllow) {
    let onlyBuiltDependencies = doc.getIn(['onlyBuiltDependencies']) as YAMLSeq<Scalar<string>>;
    if (!(onlyBuiltDependencies instanceof YAMLSeq)) {
      onlyBuiltDependencies = new YAMLSeq<Scalar<string>>();
    }
    const existing = new Set(onlyBuiltDependencies.items.map((n) => n.value));
    for (const name of BROWSER_PROVIDER_POSTINSTALL_PACKAGES) {
      if (!existing.has(name)) {
        onlyBuiltDependencies.add(scalarString(name));
      }
    }
    doc.setIn(['onlyBuiltDependencies'], onlyBuiltDependencies);
  }
}

/**
 * Rewrite .yarnrc.yml to add vite-plus dependencies
 * @param projectPath - The path to the project
 */
// Under Yarn's `node-modules` linker, `nmHoistingLimits: workspaces` STOPS a
// dependency from being hoisted past the workspace that declares it — so every
// workspace that gets a direct `vite-plus` dep receives its OWN physical
// `vitest`/`@vitest/runner` copy instead of sharing one hoisted copy at the
// monorepo root. `vp test` resolves the Vitest runner bin ONCE from the workspace
// root (the root copy) but spawns it with the package as cwd; Vitest's per-package
// Vite server then serves the test graph's `@vitest/runner` from the PACKAGE's own
// copy. The runner process initialises its (root) `@vitest/runner` module instance
// while the test file imports `describe` from the package's DIFFERENT instance
// whose module-level runner is undefined -> `describe(...)` -> `initSuite()` ->
// `validateTags(runner.config, …)` -> `TypeError: Cannot read properties of
// undefined (reading 'config')`. Yarn has no per-package "force-hoist this dep to
// root" lever, so the only reliable dedupe is to let the affected workspaces hoist
// normally (a per-workspace `installConfig.hoistingLimits: none`). See
// `setYarnWorkspaceHoistingOptOut`.
//
// Only `workspaces` is auto-fixable. The stricter `dependencies` limit keeps a
// dependency BELOW each dependent package even when the workspace opts out to
// `none`, so the opt-out does NOT dedupe there — verified with Yarn 4.17: two
// workspaces sharing a dep under root `nmHoistingLimits: dependencies` + per-
// workspace `hoistingLimits: none` still produced two physical copies, whereas
// the same setup under `workspaces` deduped to one root copy. For `dependencies`
// (and for a `workspaces` root where the affected workspace already pins its own
// isolating limit) the migration cannot fix the split from package.json, so it
// WARNS instead of silently leaving a known-broken layout. See
// `applyYarnWorkspaceHoistingFix`.

/**
 * Rewrite catalog in pnpm-workspace.yaml or .yarnrc.yml
 * @param doc - The document to rewrite
 */
export function getCatalogDependencySpec(
  currentValue: string | undefined,
  version: string,
  supportCatalog: boolean,
  options?: {
    dependencyField?: PackageJsonDependencyField;
    dependencyName?: string;
    packageManager?: PackageManager;
    catalogDependencyResolver?: CatalogDependencyResolver;
    preferredCatalogSpec?: string;
  },
): string {
  if (options?.dependencyField === 'peerDependencies') {
    if (currentValue?.startsWith('catalog:') && options.dependencyName) {
      const resolved = options.catalogDependencyResolver?.(currentValue, options.dependencyName);
      if (resolved && !isVitePlusOverrideSpec(resolved)) {
        return resolved;
      }
      return PUBLIC_PEER_DEPENDENCY_FALLBACKS[options.dependencyName] ?? currentValue;
    }
    return currentValue ?? version;
  }
  if (
    options?.dependencyField === 'optionalDependencies' &&
    options?.packageManager === PackageManager.yarn
  ) {
    return version;
  }
  if (!supportCatalog || version.startsWith('file:')) {
    return version;
  }
  return currentValue?.startsWith('catalog:')
    ? currentValue
    : (options?.preferredCatalogSpec ?? 'catalog:');
}

/**
 * #1932: under pnpm, an importer that depends on `vite-plus` (which bundles
 * `vitest`) needs a DIRECT `vite` devDep so the `vite` override binds vitest's
 * required `vite` peer to @voidzero-dev/vite-plus-core. Without a direct edge,
 * pnpm's `autoInstallPeers` fabricates a separate upstream `vite` to satisfy the
 * peer, splitting vite-plus / vite / vitest into duplicate instances (the extra
 * vite also lacks vite's `@voidzero-dev/vite-task-client` integration, breaking
 * the `vp test` cache). npm/yarn/bun redirect transitive/peer vite via root
 * overrides/resolutions (and drop the aliased vite), so this is pnpm-only,
 * mirroring the bun root-package branch in `rewriteRootWorkspacePackageJson`.
 *
 * A package that already declares `vite` in ANY dependency field, including
 * `peerDependencies` (e.g. a vite plugin pinning `vite ^6`), is left untouched
 * so its existing version contract is preserved. Call this AFTER `vite-plus`
 * has been ensured in the package, so the dependency check sees it.
 */
export function ensureDirectViteForPnpm(
  pkg: {
    dependencies?: Record<string, string>;
    devDependencies?: Record<string, string>;
    optionalDependencies?: Record<string, string>;
    peerDependencies?: Record<string, string>;
  },
  packageManager: PackageManager,
  supportCatalog: boolean,
  catalogDependencyResolver?: CatalogDependencyResolver,
): boolean {
  const viteOverride = VITE_PLUS_OVERRIDE_PACKAGES.vite;
  if (packageManager !== PackageManager.pnpm || !viteOverride) {
    return false;
  }
  const dependsOnVitePlus =
    pkg.dependencies?.[VITE_PLUS_NAME] !== undefined ||
    pkg.devDependencies?.[VITE_PLUS_NAME] !== undefined;
  const viteAlreadyDirect =
    pkg.dependencies?.vite !== undefined ||
    pkg.devDependencies?.vite !== undefined ||
    pkg.optionalDependencies?.vite !== undefined ||
    pkg.peerDependencies?.vite !== undefined;
  if (!dependsOnVitePlus || viteAlreadyDirect) {
    return false;
  }
  // The catalog-vs-alias choice is driven entirely by supportCatalog and the
  // (file:/npm:) override spec; the extra getCatalogDependencySpec options only
  // matter for an existing value or a peerDependencies field, neither of which
  // applies here (we only reach this for a fresh devDependencies entry).
  setDirectViteEdge(pkg, supportCatalog, catalogDependencyResolver);
  return true;
}

/**
 * Insert (or overwrite) a DIRECT `vite` devDependency edge in SORTED position.
 *
 * Several migration paths need a direct `vite` devDep for different reasons
 * (pnpm peer binding #1932; bun peer pre-resolution oven-sh/bun#8406; npm
 * `@vitest/mocker` hoisting for opt-in providers), but they all want the SAME
 * spec and the SAME placement, so both are centralized here. Each caller keeps
 * its OWN gate for WHEN a direct edge is needed; this owns only the spec +
 * placement.
 *
 * - The spec is computed once from the `vite` override: under a catalog
 *   (`supportCatalog`) it resolves to the preferred `catalog:` reference,
 *   otherwise the concrete `npm:@voidzero-dev/vite-plus-core@<v>` alias (or the
 *   `file:` tgz under force-override). A `catalog:` reference satisfies bun's
 *   #8406 peer pre-resolution just as well as a concrete alias because catalog
 *   refs resolve during the dependency-graph build (unlike overrides).
 * - `vite` is inserted in SORTED position rather than appended: oxfmt sorts
 *   package.json dependencies and `vp migrate` has no later format pass, so an
 *   out-of-order key (e.g. `vite` after `vite-plus`) would fail a follow-up
 *   `vp check`.
 */
export function setDirectViteEdge(
  pkg: { devDependencies?: Record<string, string> },
  supportCatalog: boolean,
  catalogDependencyResolver?: CatalogDependencyResolver,
): void {
  const viteSpec = getCatalogDependencySpec(
    undefined,
    VITE_PLUS_OVERRIDE_PACKAGES.vite,
    supportCatalog,
    { preferredCatalogSpec: catalogDependencyResolver?.preferredCatalogSpec },
  );
  // Drop any existing `vite` entry first so an out-of-order one (appended after
  // `vite-plus`) is re-placed, then splice the new entry in sorted position.
  const entries: [string, string][] = Object.entries(pkg.devDependencies ?? {}).filter(
    ([name]) => name !== 'vite',
  );
  const insertAt = entries.findIndex(([name]) => name > 'vite');
  entries.splice(insertAt === -1 ? entries.length : insertAt, 0, ['vite', viteSpec]);
  // Rewrite the keys IN PLACE (same object reference) rather than reassigning
  // `pkg.devDependencies`: some callers capture the devDependencies reference
  // before this runs (e.g. the bootstrap path's `installGroups`) and keep
  // mutating it afterwards, so a fresh object would silently drop those edits.
  const target = (pkg.devDependencies ??= {});
  for (const key of Object.keys(target)) {
    delete target[key];
  }
  Object.assign(target, Object.fromEntries(entries));
}

// A peer declaration does not install Vitest and therefore must not keep a
// workspace-wide managed Vitest catalog alive. Resolve its catalog reference to
// the public peer range before that catalog is pruned, so the surviving peer
// never points at a missing default/named catalog entry.
export function normalizeVitestPeerCatalogSpec(
  peerDependencies: Record<string, string> | undefined,
  catalogDependencyResolver?: CatalogDependencyResolver,
): boolean {
  if (!peerDependencies) {
    return false;
  }
  const current = peerDependencies.vitest;
  if (!current?.startsWith('catalog:')) {
    return false;
  }
  const normalized = getCatalogDependencySpec(current, VITEST_VERSION, true, {
    dependencyField: 'peerDependencies',
    dependencyName: 'vitest',
    catalogDependencyResolver,
  });
  if (normalized === current) {
    return false;
  }
  peerDependencies.vitest = normalized;
  return true;
}

function isVitePlusOverrideSpec(value: string): boolean {
  return (
    Object.values(VITE_PLUS_OVERRIDE_PACKAGES).includes(value) ||
    value.startsWith('npm:@voidzero-dev/vite-plus-')
  );
}

export function createCatalogDependencyResolver(
  projectPath: string,
  packageManager: PackageManager,
): CatalogDependencyResolver | undefined {
  if (packageManager === PackageManager.pnpm) {
    const pnpmWorkspaceYamlPath = path.join(projectPath, 'pnpm-workspace.yaml');
    if (!fs.existsSync(pnpmWorkspaceYamlPath)) {
      return undefined;
    }
    const doc = readYamlFile(pnpmWorkspaceYamlPath) as {
      catalog?: Record<string, string>;
      catalogs?: Record<string, Record<string, string>>;
    } | null;
    return createCatalogDependencyResolverFromCatalogs(doc?.catalog, doc?.catalogs);
  }
  if (packageManager === PackageManager.yarn) {
    const yarnrcYmlPath = path.join(projectPath, '.yarnrc.yml');
    if (!fs.existsSync(yarnrcYmlPath)) {
      return undefined;
    }
    const doc = readYamlFile(yarnrcYmlPath) as {
      catalog?: Record<string, string>;
      catalogs?: Record<string, Record<string, string>>;
    } | null;
    return createCatalogDependencyResolverFromCatalogs(doc?.catalog, doc?.catalogs);
  }
  if (packageManager === PackageManager.bun) {
    const packageJsonPath = path.join(projectPath, 'package.json');
    if (!fs.existsSync(packageJsonPath)) {
      return undefined;
    }
    const pkg = readJsonFile(packageJsonPath) as {
      workspaces?: NpmWorkspaces;
      catalog?: Record<string, string>;
      catalogs?: Record<string, Record<string, string>>;
    };
    // A missing/absent `workspaces.catalog` resolves identically whether the
    // fallback is `undefined` (optional chaining) or `{}`, so this shares the
    // exact bun catalog resolution used by the in-memory callers.
    return readBunCatalogDependencyResolver(pkg);
  }
  return undefined;
}

export function createCatalogDependencyResolverFromCatalogs(
  catalog: Record<string, string> | undefined,
  catalogs: Record<string, Record<string, string>> | undefined,
): CatalogDependencyResolver {
  const preferredCatalogSpec = selectPreferredCatalogSpec(catalog, catalogs);
  const resolver = (catalogSpec: string, dependencyName: string) => {
    const catalogName = catalogSpec.slice('catalog:'.length);
    // pnpm accepts the default catalog in either `catalog` or
    // `catalogs.default`, but rejects a workspace that defines both. Both
    // `catalog:` and `catalog:default` resolve through that one logical
    // default catalog.
    if (catalogName && catalogName !== 'default') {
      return catalogs?.[catalogName]?.[dependencyName];
    }
    return (catalog ?? catalogs?.default)?.[dependencyName];
  };
  return Object.assign(resolver, { preferredCatalogSpec });
}

function selectPreferredCatalogSpec(
  catalog: Record<string, string> | undefined,
  catalogs: Record<string, Record<string, string>> | undefined,
): string {
  const candidates: Array<{ spec: string; values: Record<string, string> }> = [];
  if (catalog) {
    candidates.push({ spec: 'catalog:', values: catalog });
  }
  for (const [name, values] of Object.entries(catalogs ?? {})) {
    candidates.push({
      spec: name === 'default' ? 'catalog:' : `catalog:${name}`,
      values,
    });
  }

  // Keep the managed toolchain together when a project already has a catalog
  // for it (for example Vize's `catalogs.vite-stack` and Rari's
  // `catalogs.build`). Prefer vite-plus as the strongest signal, followed by
  // vite and vitest. Existing dependency references keep their exact catalog
  // spec; this choice is for newly injected dependencies and overrides.
  for (const dependencyName of [VITE_PLUS_NAME, 'vite', 'vitest']) {
    const matching = candidates.find(({ values }) => Object.hasOwn(values, dependencyName));
    if (matching) {
      return matching.spec;
    }
  }

  // Reuse either valid spelling of the default catalog. Do not repurpose an
  // unrelated named catalog; when no managed/default catalog exists, create
  // the conventional top-level `catalog` instead.
  if (catalog || catalogs?.default) {
    return 'catalog:';
  }
  return 'catalog:';
}

function getYamlMapScalarStringValue(map: unknown, key: string): string | undefined {
  if (!(map instanceof YAMLMap)) {
    return undefined;
  }
  for (const item of map.items) {
    if (
      item.key instanceof Scalar &&
      item.key.value === key &&
      item.value instanceof Scalar &&
      typeof item.value.value === 'string'
    ) {
      return item.value.value;
    }
  }
  return undefined;
}

function pruneYamlMapLegacyWrapperAliases(map: unknown): void {
  if (!(map instanceof YAMLMap)) {
    return;
  }
  const stale: Array<{ key: Scalar<string>; fallback: string | undefined }> = [];
  for (const item of map.items) {
    const value = item.value instanceof Scalar ? item.value.value : undefined;
    if (typeof value === 'string' && isLegacyWrapperSpec(value) && item.key instanceof Scalar) {
      stale.push({
        key: item.key,
        fallback: LEGACY_WRAPPER_FALLBACK_VERSIONS[item.key.value],
      });
    }
  }
  for (const { key, fallback } of stale) {
    if (fallback !== undefined) {
      map.set(key, scalarString(fallback));
    } else {
      map.delete(key);
    }
  }
}

export function rewriteCatalog(
  doc: YamlDocument,
  usesVitest: boolean,
  vitestEcosystemPackages: ReadonlySet<string>,
  catalogAdditions: ReadonlySet<string>,
): string {
  const parsed = doc.toJS() as {
    catalog?: Record<string, string>;
    catalogs?: Record<string, Record<string, string>>;
  } | null;
  const preferredCatalogSpec = selectPreferredCatalogSpec(parsed?.catalog, parsed?.catalogs);
  const preferredCatalogName = preferredCatalogSpec.slice('catalog:'.length);
  const targetPath: readonly string[] =
    preferredCatalogName && preferredCatalogName !== 'default'
      ? ['catalogs', preferredCatalogName]
      : doc.has('catalog') || !doc.hasIn(['catalogs', 'default'])
        ? ['catalog']
        : ['catalogs', 'default'];

  rewriteYamlCatalogAtPath(
    doc,
    targetPath,
    true,
    usesVitest,
    vitestEcosystemPackages,
    catalogAdditions,
  );

  if (targetPath[0] !== 'catalog') {
    rewriteYamlCatalogAtPath(
      doc,
      ['catalog'],
      false,
      usesVitest,
      vitestEcosystemPackages,
      catalogAdditions,
    );
  }

  const catalogs = doc.getIn(['catalogs']);
  if (catalogs instanceof YAMLMap) {
    for (const item of catalogs.items) {
      const catalogName = item.key instanceof Scalar ? item.key.value : undefined;
      if (
        typeof catalogName !== 'string' ||
        !(item.value instanceof YAMLMap) ||
        (targetPath[0] === 'catalogs' && targetPath[1] === catalogName)
      ) {
        continue;
      }
      rewriteYamlCatalogAtPath(
        doc,
        ['catalogs', catalogName],
        false,
        usesVitest,
        vitestEcosystemPackages,
        catalogAdditions,
      );
    }
  }

  return preferredCatalogSpec;
}

function rewriteYamlCatalogAtPath(
  doc: YamlDocument,
  catalogPath: readonly string[],
  addMissing: boolean,
  usesVitest: boolean,
  vitestEcosystemPackages: ReadonlySet<string>,
  catalogAdditions: ReadonlySet<string>,
): void {
  const managed = managedOverridePackages(usesVitest);
  let catalogNode = doc.getIn(catalogPath);
  if (!(catalogNode instanceof YAMLMap)) {
    if (!addMissing) {
      return;
    }
    catalogNode = new YAMLMap();
    doc.setIn(catalogPath, catalogNode);
  }
  const catalog = catalogNode as YAMLMap;

  // Common case (no direct vitest): remove any lingering managed `vitest`
  // catalog entry so it resolves transitively through vite-plus.
  if (!usesVitest) {
    removeYamlMapVitestEntry(catalog);
  }
  for (const [key, value] of Object.entries(managed)) {
    // ERR_PNPM_CATALOG_IN_OVERRIDES  Could not resolve a catalog in the overrides: The entry for 'vite' in catalog 'default' declares a dependency using the 'file' protocol
    // ignore setting catalog if value starts with 'file:'
    if (value.startsWith('file:') || (!addMissing && !catalog.has(key))) {
      continue;
    }
    catalog.set(scalarString(key), scalarString(value));
  }
  if (!VITE_PLUS_VERSION.startsWith('file:') && (addMissing || catalog.has(VITE_PLUS_NAME))) {
    catalog.set(scalarString(VITE_PLUS_NAME), scalarString(VITE_PLUS_VERSION));
  }
  if (addMissing && VITEST_IS_MANAGED_OVERRIDE) {
    // Injected providers, plus — when the toolchain is catalog-managed (the
    // catalog now owns `vitest`) — every declared alignable @vitest/* so its
    // `catalog:` reference (written by getAlignedVitestEcosystemDependencySpec)
    // resolves instead of dangling. #2005
    const additions = catalog.has('vitest')
      ? new Set<string>([...catalogAdditions, ...vitestEcosystemPackages])
      : catalogAdditions;
    for (const name of additions) {
      if (isAlignableVitestEcosystemPackage(name)) {
        catalog.set(scalarString(name), scalarString(VITEST_VERSION));
      }
    }
  }
  for (const name of REMOVE_PACKAGES) {
    catalog.delete(name);
  }
  // Drop any entry still pointing at the deleted `vite-plus-test` wrapper.
  pruneYamlMapLegacyWrapperAliases(catalog);
  rewriteVitestEcosystemYamlCatalog(catalog, vitestEcosystemPackages);
}

function rewriteVitestEcosystemYamlCatalog(
  catalog: unknown,
  vitestEcosystemPackages: ReadonlySet<string>,
): void {
  if (!VITEST_IS_MANAGED_OVERRIDE || !(catalog instanceof YAMLMap)) {
    return;
  }
  for (const item of catalog.items) {
    const name = item.key instanceof Scalar ? item.key.value : undefined;
    if (
      typeof name === 'string' &&
      vitestEcosystemPackages.has(name) &&
      isAlignableVitestEcosystemPackage(name)
    ) {
      catalog.set(item.key, scalarString(VITEST_VERSION));
    }
  }
}

function rewriteCatalogObject(
  catalog: Record<string, string>,
  addMissing: boolean,
  usesVitest: boolean,
  vitestEcosystemPackages: ReadonlySet<string>,
): void {
  const managed = managedOverridePackages(usesVitest);
  // Common case (no direct vitest): strip a lingering managed `vitest` catalog
  // entry so it resolves transitively through vite-plus.
  if (!usesVitest) {
    removeManagedVitestEntry(catalog);
  }
  for (const [key, value] of Object.entries(managed)) {
    if (value.startsWith('file:') || (!addMissing && !(key in catalog))) {
      continue;
    }
    catalog[key] = value;
  }
  if (!VITE_PLUS_VERSION.startsWith('file:') && (addMissing || VITE_PLUS_NAME in catalog)) {
    catalog[VITE_PLUS_NAME] = VITE_PLUS_VERSION;
  }
  for (const name of REMOVE_PACKAGES) {
    delete catalog[name];
  }
  if (VITEST_IS_MANAGED_OVERRIDE) {
    for (const name of Object.keys(catalog)) {
      if (vitestEcosystemPackages.has(name) && isAlignableVitestEcosystemPackage(name)) {
        catalog[name] = VITEST_VERSION;
      }
    }
  }
}

function rewriteCatalogsObject(
  catalogs: Record<string, Record<string, string>>,
  usesVitest: boolean,
  vitestEcosystemPackages: ReadonlySet<string>,
): void {
  for (const catalog of Object.values(catalogs)) {
    rewriteCatalogObject(catalog, false, usesVitest, vitestEcosystemPackages);
  }
}

/**
 * Write catalog entries to root package.json for bun.
 * Bun stores catalogs in package.json under the `catalog` key,
 * unlike pnpm which uses pnpm-workspace.yaml.
 * @see https://bun.sh/docs/pm/catalogs
 */
export function rewriteBunCatalog(
  projectPath: string,
  usesVitest: boolean,
  vitestEcosystemPackages: ReadonlySet<string>,
): void {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }
  const managed = managedOverridePackages(usesVitest);

  editJsonFile<{
    workspaces?: NpmWorkspaces;
    catalog?: Record<string, string>;
    catalogs?: Record<string, Record<string, string>>;
    overrides?: Record<string, string>;
  }>(packageJsonPath, (pkg) => {
    // Bun supports catalogs in both workspaces.catalog and top-level catalog;
    // prefer the location the user already chose to avoid moving their config.
    const workspacesObj =
      pkg.workspaces && !Array.isArray(pkg.workspaces) ? pkg.workspaces : undefined;
    const useWorkspacesCatalog =
      workspacesObj?.catalog != null || (pkg.catalog == null && workspacesObj?.catalogs != null);
    const catalog: Record<string, string> = {
      ...(useWorkspacesCatalog ? workspacesObj?.catalog : pkg.catalog),
    };

    rewriteCatalogObject(catalog, true, usesVitest, vitestEcosystemPackages);
    pruneLegacyWrapperAliases(catalog);

    if (useWorkspacesCatalog) {
      workspacesObj.catalog = catalog;
      if (pkg.catalog) {
        rewriteCatalogObject(pkg.catalog, false, usesVitest, vitestEcosystemPackages);
        pruneLegacyWrapperAliases(pkg.catalog);
      }
    } else {
      pkg.catalog = catalog;
      if (workspacesObj?.catalog) {
        rewriteCatalogObject(workspacesObj.catalog, false, usesVitest, vitestEcosystemPackages);
        pruneLegacyWrapperAliases(workspacesObj.catalog);
      }
    }
    if (workspacesObj?.catalogs) {
      rewriteCatalogsObject(workspacesObj.catalogs, usesVitest, vitestEcosystemPackages);
      for (const named of Object.values(workspacesObj.catalogs)) {
        pruneLegacyWrapperAliases(named);
      }
    }
    if (pkg.catalogs) {
      rewriteCatalogsObject(pkg.catalogs, usesVitest, vitestEcosystemPackages);
      for (const named of Object.values(pkg.catalogs)) {
        pruneLegacyWrapperAliases(named);
      }
    }

    // bun overrides support catalog: references
    const overrides: Record<string, string> = { ...pkg.overrides };
    pruneLegacyWrapperAliases(overrides);
    // Common case (no direct vitest): strip a lingering managed `vitest`
    // override (string-valued only — a nested user override is left intact;
    // removeManagedVitestEntry also no-ops when vitest is not a managed key).
    if (!usesVitest && typeof overrides.vitest === 'string') {
      removeManagedVitestEntry(overrides);
    }
    for (const [key, value] of Object.entries(managed)) {
      const current = overrides[key] as unknown;
      // A nested object value is a user override scoped under this managed key,
      // not a version pin — leave it intact (getCatalogDependencySpec expects a
      // string and would otherwise clobber it / throw on `.startsWith`).
      if (current !== undefined && typeof current !== 'string') {
        continue;
      }
      overrides[key] = getCatalogDependencySpec(current, value, true);
    }
    pkg.overrides = overrides;

    return pkg;
  });
}

/**
 * Rewrite root workspace package.json to add vite-plus dependencies
 * @param projectPath - The path to the project
 */
export function rewriteRootWorkspacePackageJson(
  projectPath: string,
  packageManager: PackageManager,
  skipStagedMigration?: boolean,
  catalogDependencyResolver?: CatalogDependencyResolver,
  // Forwarded to `rewriteMonorepoProject` so the per-root lint-config
  // sanitizer can see hoisted deps in sibling workspace packages, not
  // just the root's own `package.json`.
  packages?: WorkspacePackage[],
  pnpmMajorVersion?: number,
  pnpmVersion?: string,
  shouldAllowBrowserBuilds = false,
  // Workspace-wide direct-vitest signal: the root resolution/override sinks are
  // shared by every package, so `vitest` stays managed here iff ANY package uses
  // vitest directly.
  workspaceUsesVitest = true,
  // Whether the workspace uses catalog references for its toolchain edges. Yarn
  // catalogs require Yarn >= 4.10.0; pnpm/bun monorepos always manage a catalog.
  // When false, the root `vite-plus` edge and the direct `vite` edge are concrete.
  supportCatalog = true,
): void {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }
  const managed = managedOverridePackages(workspaceUsesVitest);

  let movedPnpmSettings: Record<string, unknown> | undefined;
  editJsonFile<{
    resolutions?: Record<string, string>;
    overrides?: Record<string, string>;
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
    peerDependencies?: Record<string, string>;
    optionalDependencies?: Record<string, string>;
    pnpm?: PnpmPackageJsonSettings;
  }>(packageJsonPath, (pkg) => {
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
    // Common case (no workspace-wide direct vitest): strip a lingering managed
    // `vitest` from the shared root sinks so it isn't re-pinned.
    if (!workspaceUsesVitest) {
      removeManagedVitestEntry(pkg.resolutions);
      removeManagedVitestEntry(pkg.overrides);
    }
    if (packageManager === PackageManager.yarn) {
      pkg.resolutions = {
        ...pkg.resolutions,
        // FIXME: yarn don't support catalog on resolutions
        // https://github.com/yarnpkg/berry/issues/6979
        ...managed,
      };
    } else if (packageManager === PackageManager.npm) {
      pkg.overrides = {
        ...pkg.overrides,
        ...managed,
      };
    } else if (packageManager === PackageManager.bun) {
      // bun overrides are handled in rewriteBunCatalog() with catalog: references
      // Bun walks transitive peer-deps before resolving overrides; vitest 4.1.9
      // declares peer `vite ^6 || ^7 || ^8` and aborts unless `vite` is a direct
      // dep at the workspace root. Mirror the override as a devDep; the override
      // configured in rewriteBunCatalog still redirects it to vite-plus-core. A
      // bun workspace root always manages a catalog, so the direct edge resolves
      // to a `catalog:` reference. See https://github.com/oven-sh/bun/issues/8406.
      setDirectViteEdge(pkg, true, catalogDependencyResolver);
    } else if (packageManager === PackageManager.pnpm) {
      const overrideKeys = Object.keys(managed);
      const usePnpmWorkspaceSettings = pnpmSupportsWorkspaceSettings(pnpmVersion ?? '');
      if (!usePnpmWorkspaceSettings) {
        // Strip selector-shaped overrides (e.g. `parent>@vitest/browser-playwright`)
        // whose target is a removed package, before re-merging the user's
        // overrides into the new pnpm config.
        dropRemovePackageOverrideKeys(pkg.pnpm?.overrides);
        // Common case: drop a lingering managed `vitest` override before merging.
        if (!workspaceUsesVitest) {
          removeManagedVitestEntry(pkg.pnpm?.overrides);
        }
        if (!workspaceUsesVitest && pkg.pnpm?.peerDependencyRules) {
          removeVitestPeerDependencyRule(pkg.pnpm.peerDependencyRules);
        }
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
        for (const key of [...overrideKeys, ...PROVIDER_OVERRIDE_DROP_NAMES]) {
          if (pkg.resolutions?.[key]) {
            delete pkg.resolutions[key];
          }
        }
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
      if (pnpmMajorVersion !== undefined && pkg.pnpm) {
        applyBuildAllowanceToPackageJsonPnpm(pkg.pnpm, pnpmMajorVersion, shouldAllowBrowserBuilds);
      }
    }

    // ensure vite-plus is in devDependencies — skip when it already lives in
    // `dependencies` or `devDependencies` so it isn't duplicated across groups.
    if (!hasDirectVitePlusInstallEntry(pkg)) {
      pkg.devDependencies = {
        ...pkg.devDependencies,
        [VITE_PLUS_NAME]:
          packageManager === PackageManager.npm ||
          !supportCatalog ||
          VITE_PLUS_VERSION.startsWith('file:')
            ? VITE_PLUS_VERSION
            : (catalogDependencyResolver?.preferredCatalogSpec ?? 'catalog:'),
      };
    }
    ensureDirectViteForPnpm(pkg, packageManager, supportCatalog, catalogDependencyResolver);
    return pkg;
  });

  migratePnpmSettingsToWorkspaceYaml(projectPath, movedPnpmSettings);

  // rewrite package.json — `projectPath` IS the workspace root here, so
  // `workspaceContext.rootDir` matches it; sanitizer resolves
  // sibling-package paths against `projectPath`.
  rewriteMonorepoProject(
    projectPath,
    packageManager,
    skipStagedMigration,
    undefined,
    undefined,
    catalogDependencyResolver,
    packages ? { rootDir: projectPath, packages } : undefined,
    true,
    supportCatalog,
  );
}

export function readPnpmWorkspaceCatalogDependencyResolver(
  projectPath: string,
): CatalogDependencyResolver | undefined {
  const pnpmWorkspaceYamlPath = path.join(projectPath, 'pnpm-workspace.yaml');
  if (!fs.existsSync(pnpmWorkspaceYamlPath)) {
    return undefined;
  }
  const doc = readYamlFile(pnpmWorkspaceYamlPath) as {
    catalog?: Record<string, string>;
    catalogs?: Record<string, Record<string, string>>;
  } | null;
  return createCatalogDependencyResolverFromCatalogs(doc?.catalog, doc?.catalogs);
}

export function readPnpmWorkspaceOverrides(
  projectPath: string,
): Record<string, string> | undefined {
  const pnpmWorkspaceYamlPath = path.join(projectPath, 'pnpm-workspace.yaml');
  if (!fs.existsSync(pnpmWorkspaceYamlPath)) {
    return undefined;
  }
  const doc = readYamlFile(pnpmWorkspaceYamlPath) as { overrides?: Record<string, string> } | null;
  return doc?.overrides;
}

// True when pnpm-workspace.yaml configures `minimumReleaseAge` but its
// `minimumReleaseAgeExclude` is missing any Vite+-managed exemption that
// `rewritePnpmWorkspaceYaml` would add. Used by the bootstrap pending check so an
// otherwise-current workspace is not reported "already using Vite+" while the
// freshly pinned versions would still be quarantined by the age gate. Gated on
// `minimumReleaseAge` being present, mirroring the writer (no gate -> nothing to
// exempt).
export function pnpmWorkspaceMinimumReleaseAgeExemptionsPending(projectPath: string): boolean {
  const pnpmWorkspaceYamlPath = path.join(projectPath, 'pnpm-workspace.yaml');
  if (!fs.existsSync(pnpmWorkspaceYamlPath)) {
    return false;
  }
  const doc = readYamlFile(pnpmWorkspaceYamlPath) as {
    minimumReleaseAge?: unknown;
    minimumReleaseAgeExclude?: string[];
  } | null;
  if (!doc || doc.minimumReleaseAge === undefined) {
    return false;
  }
  const existing = new Set(
    Array.isArray(doc.minimumReleaseAgeExclude) ? doc.minimumReleaseAgeExclude : [],
  );
  return PNPM_MINIMUM_RELEASE_AGE_EXCLUDES.some((exclude) => !existing.has(exclude));
}

export function readPnpmWorkspacePeerDependencyRules(
  projectPath: string,
): { allowAny?: string[]; allowedVersions?: Record<string, string> } | undefined {
  const pnpmWorkspaceYamlPath = path.join(projectPath, 'pnpm-workspace.yaml');
  if (!fs.existsSync(pnpmWorkspaceYamlPath)) {
    return undefined;
  }
  const doc = readYamlFile(pnpmWorkspaceYamlPath) as {
    peerDependencyRules?: { allowAny?: string[]; allowedVersions?: Record<string, string> };
  } | null;
  return doc?.peerDependencyRules;
}

export function ensurePnpmWorkspacePackages(
  projectPath: string,
  workspacePatterns: string[],
): boolean {
  if (workspacePatterns.length === 0) {
    return false;
  }
  const pnpmWorkspaceYamlPath = path.join(projectPath, 'pnpm-workspace.yaml');
  let changed = false;
  editYamlFile(pnpmWorkspaceYamlPath, (doc) => {
    if (doc.has('packages')) {
      return;
    }
    const packages = new YAMLSeq<Scalar<string>>();
    for (const pattern of workspacePatterns) {
      packages.add(scalarString(pattern));
    }
    doc.set('packages', packages);
    changed = true;
  });
  return changed;
}

export function readBunCatalogDependencyResolver(pkg: {
  workspaces?: NpmWorkspaces;
  catalog?: Record<string, string>;
  catalogs?: Record<string, Record<string, string>>;
}): CatalogDependencyResolver {
  const workspacesObj = pkg.workspaces && !Array.isArray(pkg.workspaces) ? pkg.workspaces : {};
  const fromWorkspaces = createCatalogDependencyResolverFromCatalogs(
    workspacesObj.catalog,
    workspacesObj.catalogs,
  );
  const fromPkg = createCatalogDependencyResolverFromCatalogs(pkg.catalog, pkg.catalogs);
  const resolver = (catalogSpec: string, dependencyName: string) =>
    fromWorkspaces(catalogSpec, dependencyName) ?? fromPkg(catalogSpec, dependencyName);
  return Object.assign(resolver, {
    preferredCatalogSpec:
      workspacesObj.catalog || workspacesObj.catalogs
        ? fromWorkspaces.preferredCatalogSpec
        : fromPkg.preferredCatalogSpec,
  });
}
