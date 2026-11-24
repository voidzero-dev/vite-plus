import fs from 'node:fs';
import path from 'node:path';

import tsrsPkg from 'tsrs/package.json' with { type: 'json' };
import { coerce, isLessThan } from 'verkit';
import { isMap, isScalar, isSeq, Scalar, YAMLMap, YAMLSeq } from 'yaml';

import type { WorkspacePackage } from '../../types/index.ts';
import { PackageManager } from '../../types/index.ts';
import type { CatalogDependencyResolver } from '../../utils/catalog.ts';
import {
  bunSupportsMergedDefaultCatalogs,
  createCatalogDependencyResolverFromCatalogs,
  getBunCatalogConfig,
  getYamlCatalogPath,
  readBunCatalogDependencyResolver,
  selectPreferredCatalogSpec,
  versionAtLeast,
} from '../../utils/catalog.ts';
import {
  VITEST_AGE_GATE_EXEMPT_PACKAGES,
  VITEST_VERSION,
  VITE_PLUS_NAME,
  VITE_PLUS_OVERRIDE_PACKAGES,
  VITE_PLUS_VERSION,
  isForceOverrideMode,
} from '../../utils/constants.ts';
import { isObjectValue } from '../../utils/json-value.ts';
import type { JsonObject, JsonValue } from '../../utils/json.ts';
import { isJsonObject } from '../../utils/json.ts';
import { editJsonFile, readJsonFile } from '../../utils/json.ts';
import type { NpmWorkspaces } from '../../utils/workspace.ts';
import type { YamlMapping } from '../../utils/yaml-value.ts';
import { isYamlMapping } from '../../utils/yaml-value.ts';
import type { YamlDocument } from '../../utils/yaml.ts';
import { editYamlFile, readYamlFile, scalarString } from '../../utils/yaml.ts';
import {
  dropRemovePackageOverrideKeys,
  ensurePackageJsonOverrides,
  ensurePnpmExoticSubdepsSetting,
  hasDirectVitePlusInstallEntry,
  isAlignableVitestEcosystemPackage,
  isLegacyWrapperSpec,
  managedOverridePackages,
  mergeManagedPnpmPeerDependencyRules,
  isManagedAllowedVersion,
  peerDependencyRulesHaveStaleEntry,
  pruneLegacyWrapperAliases,
  removeManagedVitestEntry,
  removeYamlMapVitestEntry,
  rewriteMonorepoProject,
  shouldDropProviderOverrideKey,
  pnpmPeerDependencyRuleKeys,
  workspaceUsesWebdriverioProvider,
} from '../migrator.ts';
import type { MigrationReport } from '../report.ts';
import type {
  PreparedOxlintConfigs,
  RetainedImportDependencies,
  PackageJsonDependencyField,
  PnpmPackageJsonSettings,
} from './shared.ts';
import { lookupSetting } from './shared.ts';
import {
  LEGACY_WRAPPER_FALLBACK_VERSIONS,
  PROVIDER_OVERRIDE_DROP_NAMES,
  OPT_IN_BROWSER_PROVIDERS,
  bundledPackagesToRemove,
  VITEST_IS_MANAGED_OVERRIDE,
  isPlainRecord,
  pinnedPackageManagerVersion,
  pnpmOverrideKey,
  readPackageJsonIfExists,
  vitePeerDependencyRuleRequired,
} from './shared.ts';

// Transitive packages with postinstall scripts that vite-plus's deps drag in
// via `@vitest/browser-webdriverio` → `webdriverio` → `@wdio/utils`. pnpm v10
// refuses to run these without explicit approval, so `vp migrate` records the
// allow/deny decision up front: deny by default (the user isn't using
// webdriverio), allow when the user actually depends on webdriverio.
const BROWSER_PROVIDER_POSTINSTALL_PACKAGES = ['edgedriver', 'geckodriver'] as const;

const PUBLIC_PEER_DEPENDENCY_FALLBACKS = {
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
  'tsrs',
  '@tsrs/*',
  'oxfmt',
  '@oxfmt/*',
  ...VITEST_AGE_GATE_EXEMPT_PACKAGES,
] as const;

// The bootstrap placeholder used staged publishing; the real release uses
// GitHub trusted publishing with provenance. pnpm ranks the former higher.
// Use the same dependency metadata as vp toolchain. Exempt only the bundled
// version, not other releases or the @tsrs/* platform packages.
// https://github.com/oxc-project/tsrs/issues/945
const PNPM_TSRS_TRUST_POLICY_EXCLUDE = `tsrs@${tsrsPkg.version}`;

function hasTsrsTrustPolicyException(excludes: ReadonlyArray<unknown>): boolean {
  return excludes.includes('tsrs') || excludes.includes(PNPM_TSRS_TRUST_POLICY_EXCLUDE);
}

// The packages field became optional in pnpm 10.5.0.
// https://github.com/pnpm/pnpm/releases/tag/v10.5.0
export function pnpmRequiresWorkspacePackages(version: string): boolean {
  const coerced = coerce(version);
  return coerced ? isLessThan(coerced, '10.5.0') : false;
}

const PNPM_WORKSPACE_SETTINGS_MIN_VERSION = '10.6.2';

// pnpm 10.5 started reading package.json#pnpm settings from
// pnpm-workspace.yaml, but overrides and peerDependencyRules needed fixes in
// 10.5.1 and 10.6.2 respectively. Use the latter as the atomic migration
// boundary so the complete object can move without splitting its ownership.
export function pnpmSupportsWorkspaceSettings(version: string): boolean {
  return versionAtLeast(version, PNPM_WORKSPACE_SETTINGS_MIN_VERSION, ['latest', 'next']);
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
  return versionAtLeast(version, PNPM_CATALOG_MIN_VERSION, ['latest', 'next']);
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
  return versionAtLeast(version, YARN_CATALOG_MIN_VERSION, ['latest', 'next', 'stable']);
}

const YARN_PREAPPROVED_PACKAGES_MIN_VERSION = '4.10.0';

// `npmPreapprovedPackages` shipped in Yarn 4.10.0, and an older Yarn refuses to
// run at all with an rc key it does not know. Like `yarnSupportsCatalog`, this
// receives the recorded (pre-upgrade) version.
export function yarnSupportsPreapprovedPackages(version: string): boolean {
  return versionAtLeast(version, YARN_PREAPPROVED_PACKAGES_MIN_VERSION, [
    'latest',
    'next',
    'stable',
  ]);
}

const BUN_TOP_LEVEL_CATALOG_MIN_VERSION = '1.2.21';

// Bun resolves `catalog:` references ONLY inside a workspace: a root
// package.json with a `workspaces` field. An empty one (`[]` or
// `{ packages: [] }`) is enough (#3375). Bun before 1.2.21 also ignores a
// top-level `catalog` or `catalogs` (verified on 1.2.14 to 1.2.20), so it
// needs the nested location that `rewriteBunCatalog` keeps in place. Otherwise
// `catalog:` edges make `bun install` abort with "vite@catalog: failed to
// resolve", so the toolchain edges must stay concrete.
export function bunSupportsCatalog(pkg: { workspaces?: NpmWorkspaces }, version: string): boolean {
  const { workspaces } = pkg;
  if (workspaces === undefined || workspaces === null) {
    return false;
  }
  const writesNestedCatalog = getBunCatalogConfig(pkg) !== pkg;
  return (
    writesNestedCatalog ||
    versionAtLeast(version, BUN_TOP_LEVEL_CATALOG_MIN_VERSION, ['latest', 'canary'])
  );
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

// `supportsCatalog` for a workspace root, reading Bun's workspace state from its package.json.
export function workspaceSupportsCatalog(
  rootDir: string,
  packageManager: PackageManager,
  version: string,
): boolean {
  return supportsCatalog(
    packageManager,
    version,
    packageManager === PackageManager.bun &&
      bunSupportsCatalog(readJsonFile(path.join(rootDir, 'package.json')), version),
  );
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
}): JsonObject | undefined {
  if (!pkg.pnpm) {
    return undefined;
  }
  const settings: JsonObject = {};
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
function mergePnpmWorkspaceSetting(existing: unknown, incoming: JsonValue | undefined) {
  if (Array.isArray(existing) && Array.isArray(incoming)) {
    const seen = new Set<string | undefined>();
    const current: Array<unknown> = existing;
    return [...current, ...incoming].filter((value) => {
      const key = JSON.stringify(value);
      if (seen.has(key)) {
        return false;
      }
      seen.add(key);
      return true;
    });
  }
  if (isYamlMapping(existing) && isPlainRecord(incoming)) {
    const merged: YamlMapping = { ...existing };
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
  settings: JsonObject | undefined,
): void {
  if (!settings || Object.keys(settings).length === 0) {
    return;
  }
  const pnpmWorkspaceYamlPath = path.join(projectPath, 'pnpm-workspace.yaml');
  if (!fs.existsSync(pnpmWorkspaceYamlPath)) {
    fs.writeFileSync(pnpmWorkspaceYamlPath, '');
  }
  editYamlFile(pnpmWorkspaceYamlPath, (doc) => {
    const output: unknown = doc.toJS();
    const workspace = isYamlMapping(output) ? output : {};
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
  usesWebdriverioProvider = false,
  pnpmVersion: string | undefined,
): void {
  const pnpmWorkspaceYamlPath = path.join(projectPath, 'pnpm-workspace.yaml');
  if (!fs.existsSync(pnpmWorkspaceYamlPath)) {
    fs.writeFileSync(pnpmWorkspaceYamlPath, '');
  }
  const managed = managedOverridePackages(usesVitest, usesWebdriverioProvider);

  editYamlFile(pnpmWorkspaceYamlPath, (doc) => {
    // catalog
    const preferredCatalogSpec = rewriteCatalog(
      doc,
      usesVitest,
      vitestEcosystemPackages,
      catalogAdditions,
      PackageManager.pnpm,
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
    if (isMap(overrides)) {
      const keysSnapshot = overrides.items.map((item) => item.key);
      for (const keyNode of keysSnapshot) {
        const rawKey = isScalar(keyNode) ? keyNode.value : keyNode;
        if (typeof rawKey === 'string' && shouldDropProviderOverrideKey(rawKey)) {
          overrides.delete(keyNode);
        }
      }
    }
    // Common case (no direct vitest): actively strip any lingering managed
    // `vitest` override so it arrives transitively through vite-plus.
    if (!usesVitest) {
      removeYamlMapVitestEntry(doc.getIn(['overrides']), 'pnpm-ranged');
    }
    for (const key of Object.keys(managed)) {
      // Managed keys are range-qualified (`vite@*`), so the override never
      // rewrites an importer's `catalog:` spec. See `pnpmOverrideKey`.
      // Move a pre-#2309 bare key's value to the new key, so a user's own
      // `catalog:<name>` choice survives. Then delete the bare key. Two keys
      // for one package would restore the match that clobbers `catalog:`.
      const overrideKey = pnpmOverrideKey(key);
      const currentVersion =
        getYamlMapScalarStringValue(overrides, overrideKey) ??
        getYamlMapScalarStringValue(overrides, key);
      // Keep the WebdriverIO browser override concrete: it also works in a
      // workspace where no package directly imports the browser entry point.
      const version =
        key === '@vitest/browser'
          ? managed[key]
          : getCatalogDependencySpec(currentVersion, managed[key], true, {
              preferredCatalogSpec,
            });
      if (isMap(overrides)) {
        overrides.delete(key);
      }
      doc.setIn(['overrides', scalarString(overrideKey)], scalarString(version));
    }
    // remove dependency selector from vite, e.g. "vite-plugin-svgr>vite": "npm:vite@7.0.12"
    // Snapshot the keys before deleting (mirrors the `keysSnapshot` loop above):
    // YAMLMap.delete splices `.items`, so iterating the live array would shift
    // the next entry into the current slot and skip it (two ADJACENT `...>vite`
    // selectors would leave the second behind).
    const updatedOverrides = doc.getIn(['overrides']);
    if (isMap(updatedOverrides)) {
      const updatedOverrideKeys = updatedOverrides.items.map((item) => item.key);
      for (const key of updatedOverrideKeys) {
        if (isScalar(key) && typeof key.value === 'string' && key.value.includes('>')) {
          const splits = key.value.split('>');
          if (splits.at(-1)?.trim() === 'vite') {
            updatedOverrides.delete(key);
          }
        }
      }
    }
    // peerDependencyRules.allowAny
    // `getIn` unwraps scalar values, so a malformed scalar-valued `allowAny:`
    // (`allowAny: react`) arrives as a string here; coerce it into a sequence
    // entry instead of crashing on `.items` below.
    const allowAnyNode = doc.getIn(['peerDependencyRules', 'allowAny']);
    let allowAny: YAMLSeq;
    if (isSeq(allowAnyNode)) {
      allowAny = allowAnyNode;
    } else {
      allowAny = new YAMLSeq<Scalar<string>>();
      if (typeof allowAnyNode === 'string' && allowAnyNode.length > 0) {
        allowAny.add(scalarString(allowAnyNode));
      }
    }
    // Drop lingering managed `vitest` and stale `vite` entries.
    const peerRuleKeys = pnpmPeerDependencyRuleKeys(usesVitest, pnpmVersion);
    const stalePeerRuleKeys = peerRuleKeys.stale;
    allowAny.items = allowAny.items.filter(
      (n) => !(isScalar(n) && typeof n.value === 'string' && stalePeerRuleKeys.includes(n.value)),
    );
    const existing = yamlSequenceValues(allowAny);
    for (const key of peerRuleKeys.managed) {
      if (!existing.has(key)) {
        allowAny.add(scalarString(key));
      }
    }
    setPeerDependencyRulesField(doc, 'allowAny', allowAny);

    // peerDependencyRules.allowedVersions
    // Same guard as above: a malformed non-map `allowedVersions:` value (pnpm
    // expects a map, so it carries no usable data) is replaced outright.
    const allowedVersionsNode = doc.getIn(['peerDependencyRules', 'allowedVersions']);
    let allowedVersions: YAMLMap;
    if (isMap(allowedVersionsNode)) {
      allowedVersions = allowedVersionsNode;
    } else {
      allowedVersions = new YAMLMap<Scalar<string>, Scalar<string>>();
    }
    for (const key of stalePeerRuleKeys) {
      if (isManagedAllowedVersion(key, allowedVersions.get(key))) {
        allowedVersions.delete(key);
      }
    }
    for (const key of peerRuleKeys.managed) {
      // - vite: '*'
      allowedVersions.set(scalarString(key), scalarString('*'));
    }
    setPeerDependencyRulesField(doc, 'allowedVersions', allowedVersions);

    // minimumReleaseAgeExclude: exempt the Vite+-managed packages (vite-plus,
    // @voidzero-dev/*, the ox* family, and the vitest family) from the age gate.
    if (doc.has('minimumReleaseAge')) {
      const existingExclude = doc.getIn(['minimumReleaseAgeExclude']);
      if (existingExclude && !isSeq(existingExclude)) {
        throw new Error('pnpm-workspace.yaml: minimumReleaseAgeExclude must be a sequence');
      }
      const minimumReleaseAgeExclude = isSeq(existingExclude) ? existingExclude : new YAMLSeq();
      const existing = yamlSequenceValues(minimumReleaseAgeExclude);
      for (const exclude of PNPM_MINIMUM_RELEASE_AGE_EXCLUDES) {
        if (!existing.has(exclude)) {
          minimumReleaseAgeExclude.add(scalarString(exclude));
        }
      }
      doc.setIn(['minimumReleaseAgeExclude'], minimumReleaseAgeExclude);
    }

    if (doc.get('trustPolicy') === 'no-downgrade') {
      const node = doc.get('trustPolicyExclude');
      if (node !== undefined && node !== null && !isSeq(node)) {
        throw new Error('pnpm-workspace.yaml: trustPolicyExclude must be a sequence');
      }
      const excludes = node ?? new YAMLSeq<Scalar<string>>();
      if (!hasTsrsTrustPolicyException(excludes.toJSON())) {
        excludes.add(scalarString(PNPM_TSRS_TRUST_POLICY_EXCLUDE));
        doc.set('trustPolicyExclude', excludes);
      }
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
    onlyBuiltDependencies?: Array<string>;
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
      const isNew = !isMap(existing);
      const allowBuilds = isNew ? new YAMLMap() : existing;
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
    const node = doc.getIn(['onlyBuiltDependencies']);
    const onlyBuiltDependencies = isSeq(node) ? node : new YAMLSeq();
    const existing = yamlSequenceValues(onlyBuiltDependencies);
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
      return (
        lookupSetting(PUBLIC_PEER_DEPENDENCY_FALLBACKS, options.dependencyName) ?? currentValue
      );
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
 * Insert (or overwrite) a DIRECT `vite` devDependency edge in SORTED position.
 *
 * Several migration paths need a direct `vite` devDep for different reasons
 * (bun peer pre-resolution oven-sh/bun#8406; npm
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
  const entries: Array<[string, string]> = Object.entries(pkg.devDependencies ?? {}).filter(
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
  packageManagerVersion?: string,
): CatalogDependencyResolver | undefined {
  if (packageManager === PackageManager.pnpm) {
    return readPnpmWorkspaceCatalogDependencyResolver(projectPath);
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
    return createCatalogDependencyResolverFromCatalogs(doc?.catalog, doc?.catalogs, packageManager);
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
      packageManager?: string;
      devEngines?: { packageManager?: unknown };
    };
    return readBunCatalogDependencyResolver(
      pkg,
      packageManagerVersion ?? pinnedPackageManagerVersion(pkg),
    );
  }
  return undefined;
}

function yamlSequenceValues(sequence: YAMLSeq): Set<unknown> {
  return new Set(sequence.items.map((item) => (isScalar(item) ? item.value : item)));
}

function getYamlMapScalarStringValue(map: unknown, key: string): string | undefined {
  if (!isMap(map)) {
    return undefined;
  }
  for (const item of map.items) {
    if (
      isScalar(item.key) &&
      item.key.value === key &&
      isScalar(item.value) &&
      typeof item.value.value === 'string'
    ) {
      return item.value.value;
    }
  }
  return undefined;
}

/**
 * Merge the managed override entries into a pnpm `overrides` record, under the
 * range-qualified keys (see `pnpmOverrideKey`).
 *
 * Both writers of the package.json `pnpm.overrides` sink go through this
 * helper: the standalone writer and the monorepo-root writer. That sink serves
 * pnpm 9.5 to 10.6.1. The helper deletes a bare managed key that a pre-#2309
 * migration left. Two keys for one package would restore the match that
 * clobbers `catalog:` importer specs.
 *
 * A bare key's `catalog:` value moves to the ranged key, so a user's own
 * `catalog:<name>` choice survives the re-keying. This mirrors
 * `getCatalogDependencySpec`, which keeps a `catalog:` reference in the
 * pnpm-workspace.yaml sink. A `file:` managed spec (force-override mode) always
 * wins, because there is no catalog to resolve against in that mode.
 *
 * Force-override mode also appends the `vite-plus` pin, under a BARE key. The
 * pin only exists in `file:` tgz mode, where migration writes the tgz spec
 * straight into every manifest instead of a `catalog:` reference. In that mode
 * there is no catalog provenance for a bare key to strip. This matches the
 * workspace-yaml force-override path in `rewriteStandaloneProject`.
 */
export function mergeManagedPnpmOverrides(
  overrides: Record<string, string> | undefined,
  managed: Record<string, string>,
) {
  const next = { ...overrides };
  for (const [dependencyName, managedSpec] of Object.entries(managed)) {
    const overrideKey = pnpmOverrideKey(dependencyName);
    const existing = next[overrideKey] ?? next[dependencyName];
    delete next[dependencyName];
    next[overrideKey] =
      existing?.startsWith('catalog:') && !managedSpec.startsWith('file:') ? existing : managedSpec;
  }
  if (isForceOverrideMode()) {
    next[VITE_PLUS_NAME] = VITE_PLUS_VERSION;
  }
  return next;
}

function pruneYamlMapLegacyWrapperAliases(map: unknown): void {
  if (!isMap(map)) {
    return;
  }
  const stale: Array<{ key: Scalar; fallback: string | undefined }> = [];
  for (const item of map.items) {
    const value = isScalar(item.value) ? item.value.value : undefined;
    if (
      typeof value === 'string' &&
      isLegacyWrapperSpec(value) &&
      isScalar(item.key) &&
      typeof item.key.value === 'string'
    ) {
      stale.push({
        key: item.key,
        fallback: lookupSetting(LEGACY_WRAPPER_FALLBACK_VERSIONS, item.key.value),
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
  packageManager: PackageManager,
): string {
  const parsed: unknown = doc.toJS();
  const settings = isYamlMapping(parsed) ? parsed : {};
  const namedCatalogs: Record<string, YamlMapping> = {};
  if (isYamlMapping(settings.catalogs)) {
    for (const [name, entries] of Object.entries(settings.catalogs)) {
      if (isYamlMapping(entries)) {
        namedCatalogs[name] = entries;
      }
    }
  }
  const preferredCatalogSpec = selectPreferredCatalogSpec(
    isYamlMapping(settings.catalog) ? settings.catalog : undefined,
    namedCatalogs,
    packageManager,
  );
  const targetPath = getYamlCatalogPath(doc, preferredCatalogSpec, packageManager);

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
  if (isMap(catalogs)) {
    for (const item of catalogs.items) {
      const catalogName = isScalar(item.key) ? item.key.value : undefined;
      if (
        typeof catalogName !== 'string' ||
        !isMap(item.value) ||
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
  catalogPath: ReadonlyArray<string>,
  addMissing: boolean,
  usesVitest: boolean,
  vitestEcosystemPackages: ReadonlySet<string>,
  catalogAdditions: ReadonlySet<string>,
): void {
  const managed = managedOverridePackages(usesVitest);
  const catalogNode = doc.getIn(catalogPath);
  const catalog = isMap(catalogNode) ? catalogNode : new YAMLMap();
  if (catalog !== catalogNode) {
    if (!addMissing) {
      return;
    }
    doc.setIn(catalogPath, catalog);
  }

  // Common case (no direct vitest): remove any lingering managed `vitest`
  // catalog entry so it resolves transitively through vite-plus.
  if (!usesVitest) {
    removeYamlMapVitestEntry(catalog);
  }
  for (const [key, value] of Object.entries(managed)) {
    // ERR_PNPM_CATALOG_IN_OVERRIDES  Could not resolve a catalog in the overrides: The entry for 'vite' in catalog 'default' declares a dependency using the 'file' protocol
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
    // These packages need catalog entries even when the catalog omits vitest (#2005).
    // Retained browser peers keep their declared catalog ranges.
    const additions = new Set([
      ...vitestEcosystemPackages,
      ...OPT_IN_BROWSER_PROVIDERS.filter((name) => catalogAdditions.has(name)),
    ]);
    for (const name of additions) {
      if (isAlignableVitestEcosystemPackage(name)) {
        catalog.set(scalarString(name), scalarString(VITEST_VERSION));
      }
    }
  }
  for (const name of bundledPackagesToRemove(catalogAdditions)) {
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
  if (!VITEST_IS_MANAGED_OVERRIDE || !isMap(catalog)) {
    return;
  }
  for (const item of catalog.items) {
    const name = isScalar(item.key) ? item.key.value : undefined;
    if (
      typeof name === 'string' &&
      vitestEcosystemPackages.has(name) &&
      isAlignableVitestEcosystemPackage(name)
    ) {
      catalog.set(item.key, scalarString(VITEST_VERSION));
    }
  }
}

function shouldWriteCatalogEntry(
  name: string,
  catalog: Record<string, string>,
  addMissing: boolean,
  otherDefaultCatalog?: Record<string, string>,
): boolean {
  // Merged Bun default catalogs reject names defined in both.
  return name in catalog || (addMissing && !Object.hasOwn(otherDefaultCatalog ?? {}, name));
}

function rewriteCatalogObject(
  catalog: Record<string, string>,
  addMissing: boolean,
  usesVitest: boolean,
  vitestEcosystemPackages: ReadonlySet<string>,
  catalogAdditions: ReadonlySet<string>,
  otherDefaultCatalog?: Record<string, string>,
): void {
  const managed = managedOverridePackages(usesVitest);
  // Common case (no direct vitest): strip a lingering managed `vitest` catalog
  // entry so it resolves transitively through vite-plus.
  if (!usesVitest) {
    removeManagedVitestEntry(catalog);
  }
  for (const [key, value] of Object.entries(managed)) {
    if (
      value.startsWith('file:') ||
      !shouldWriteCatalogEntry(key, catalog, addMissing, otherDefaultCatalog)
    ) {
      continue;
    }
    catalog[key] = value;
  }
  if (
    !VITE_PLUS_VERSION.startsWith('file:') &&
    shouldWriteCatalogEntry(VITE_PLUS_NAME, catalog, addMissing, otherDefaultCatalog)
  ) {
    catalog[VITE_PLUS_NAME] = VITE_PLUS_VERSION;
  }
  for (const name of bundledPackagesToRemove(catalogAdditions)) {
    delete catalog[name];
  }
  if (VITEST_IS_MANAGED_OVERRIDE) {
    // Like the pnpm catalog, add every declared alignable @vitest/* so its new
    // `catalog:` reference resolves. #2005
    const names = addMissing
      ? new Set([...Object.keys(catalog), ...vitestEcosystemPackages])
      : Object.keys(catalog);
    for (const name of names) {
      if (
        vitestEcosystemPackages.has(name) &&
        isAlignableVitestEcosystemPackage(name) &&
        shouldWriteCatalogEntry(name, catalog, addMissing, otherDefaultCatalog)
      ) {
        catalog[name] = VITEST_VERSION;
      }
    }
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
  usesWebdriverioProvider = false,
  packageManagerVersion = 'latest',
  catalogAdditions: ReadonlySet<string> = new Set(),
): void {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }
  const managed = managedOverridePackages(usesVitest, usesWebdriverioProvider);

  function rewriteCatalog(
    target: Record<string, string>,
    addMissing: boolean,
    otherDefaultCatalog?: Record<string, string>,
  ): void {
    rewriteCatalogObject(
      target,
      addMissing,
      usesVitest,
      vitestEcosystemPackages,
      catalogAdditions,
      otherDefaultCatalog,
    );
    pruneLegacyWrapperAliases(target);
  }

  editJsonFile<{
    workspaces?: NpmWorkspaces;
    catalog?: Record<string, string>;
    catalogs?: Record<string, Record<string, string>>;
    overrides?: JsonObject;
  }>(packageJsonPath, (pkg) => {
    // Bun supports catalogs in both workspaces.catalog and top-level catalog;
    // prefer the location the user already chose to avoid moving their config.
    const workspacesObj =
      pkg.workspaces && !Array.isArray(pkg.workspaces) ? pkg.workspaces : undefined;
    const activeCatalogs = getBunCatalogConfig(pkg);
    // Like the pnpm catalog, add missing entries to the catalog the new
    // `catalog:` references point at. A named one such as `catalogs.build`
    // takes them instead of a newly created default catalog.
    const { preferredCatalogSpec } = readBunCatalogDependencyResolver(pkg, packageManagerVersion);
    const mergeDefaults = bunSupportsMergedDefaultCatalogs(packageManagerVersion);
    const preferredName = preferredCatalogSpec.slice('catalog:'.length);
    const namedTarget =
      preferredName || mergeDefaults
        ? activeCatalogs.catalogs?.[preferredName || 'default']
        : undefined;
    if (!namedTarget || activeCatalogs.catalog) {
      const catalog = { ...activeCatalogs.catalog };
      rewriteCatalog(catalog, !namedTarget);
      activeCatalogs.catalog = catalog;

      const otherCatalog = activeCatalogs === pkg ? workspacesObj?.catalog : pkg.catalog;
      if (otherCatalog) {
        rewriteCatalog(otherCatalog, false);
      }
    }
    for (const location of [workspacesObj, pkg]) {
      for (const [name, named] of Object.entries(location?.catalogs ?? {})) {
        const otherDefault = mergeDefaults && name === 'default' ? location?.catalog : undefined;
        rewriteCatalog(named, named === namedTarget, otherDefault);
      }
    }

    pruneLegacyWrapperAliases(pkg.overrides);
    // Keep a named reference if the updated catalog supplies the target.
    const resolveCatalog = readBunCatalogDependencyResolver(pkg, packageManagerVersion);
    pkg.overrides = ensurePackageJsonOverrides(pkg.overrides, managed, (name, current, target) => {
      if (name === '@vitest/browser') {
        return target;
      }
      const catalogSpec =
        current?.startsWith('catalog:') && resolveCatalog(current, name) === target
          ? current
          : preferredCatalogSpec;
      return getCatalogDependencySpec(catalogSpec, target, true);
    }).overrides;

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
  packages?: Array<WorkspacePackage>,
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
  preparedOxlintConfigs?: PreparedOxlintConfigs,
  retainedImportDependencies?: RetainedImportDependencies,
): void {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }
  const managed = managedOverridePackages(
    workspaceUsesVitest,
    workspaceUsesWebdriverioProvider(projectPath, packages),
  );

  let movedPnpmSettings: JsonObject | undefined;
  editJsonFile<{
    resolutions?: Record<string, string>;
    overrides?: JsonObject;
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
    // Remove stale overrides for bundled and official opt-in providers before
    // aligning them with Vitest. Preserve community-provider overrides.
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
      pkg.overrides = ensurePackageJsonOverrides(pkg.overrides, managed).overrides;
    } else if (packageManager === PackageManager.bun) {
      if (!supportCatalog) {
        pkg.overrides = ensurePackageJsonOverrides(pkg.overrides, managed).overrides;
      }
      // Bun walks transitive peer-deps before resolving overrides; vitest 4.1.9
      // declares peer `vite ^6 || ^7 || ^8` and aborts unless `vite` is a direct
      // dep at the workspace root. Mirror the override as a devDep; the override
      // configured in rewriteBunCatalog still redirects it to vite-plus-core.
      // See https://github.com/oven-sh/bun/issues/8406.
      setDirectViteEdge(pkg, supportCatalog, catalogDependencyResolver);
    } else if (packageManager === PackageManager.pnpm) {
      const usePnpmWorkspaceSettings = pnpmSupportsWorkspaceSettings(pnpmVersion ?? '');
      if (!usePnpmWorkspaceSettings) {
        // Strip selector-shaped overrides (e.g. `parent>@vitest/browser-playwright`)
        // whose target is a removed package, before re-merging the user's
        // overrides into the new pnpm config.
        dropRemovePackageOverrideKeys(pkg.pnpm?.overrides);
        // Common case: drop a lingering managed `vitest` override before merging.
        // `mergeManagedPnpmPeerDependencyRules` drops its peer rule.
        if (!workspaceUsesVitest) {
          removeManagedVitestEntry(pkg.pnpm?.overrides, 'pnpm-ranged');
        }
        pkg.pnpm = {
          ...pkg.pnpm,
          overrides: mergeManagedPnpmOverrides(pkg.pnpm?.overrides, managed),
          peerDependencyRules: mergeManagedPnpmPeerDependencyRules(
            pkg.pnpm?.peerDependencyRules,
            pnpmPeerDependencyRuleKeys(workspaceUsesVitest, pnpmVersion),
          ),
        };
      } else {
        const overrideKeys = Object.keys(managedOverridePackages(workspaceUsesVitest));
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
          if (splits.at(-1)?.trim() === 'vite') {
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
    packages
      ? {
          rootDir: projectPath,
          packages,
          oxlintConfigs: preparedOxlintConfigs,
          retainedImportDependencies,
        }
      : undefined,
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
  return createCatalogDependencyResolverFromCatalogs(
    doc?.catalog,
    doc?.catalogs,
    PackageManager.pnpm,
  );
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
    minimumReleaseAgeExclude?: Array<string>;
  } | null;
  if (!doc || doc.minimumReleaseAge === undefined) {
    return false;
  }
  const existing = new Set(
    Array.isArray(doc.minimumReleaseAgeExclude) ? doc.minimumReleaseAgeExclude : [],
  );
  return PNPM_MINIMUM_RELEASE_AGE_EXCLUDES.some((exclude) => !existing.has(exclude));
}

// Reconcile the exception even when the project already uses Vite+ and its
// other migration settings are current. Mirror the writer's policy gate.
export function pnpmWorkspaceTsrsTrustPolicyExceptionPending(projectPath: string): boolean {
  const workspacePath = path.join(projectPath, 'pnpm-workspace.yaml');
  if (!fs.existsSync(workspacePath)) {
    return false;
  }
  const doc = readYamlFile(workspacePath) as {
    trustPolicy?: unknown;
    trustPolicyExclude?: unknown;
  } | null;
  if (doc?.trustPolicy !== 'no-downgrade') {
    return false;
  }
  const excludes = Array.isArray(doc.trustPolicyExclude) ? doc.trustPolicyExclude : [];
  return !hasTsrsTrustPolicyException(excludes);
}

// Write one pnpm-workspace.yaml `peerDependencyRules` field, or delete it when
// no entry remains. An emptied `peerDependencyRules` map is deleted too.
function setPeerDependencyRulesField(
  doc: YamlDocument,
  field: 'allowAny' | 'allowedVersions',
  node: YAMLSeq | YAMLMap,
): void {
  if (node.items.length > 0) {
    doc.setIn(['peerDependencyRules', field], node);
    return;
  }
  if (doc.hasIn(['peerDependencyRules', field])) {
    doc.deleteIn(['peerDependencyRules', field]);
  }
  const rules = doc.get('peerDependencyRules');
  if (isMap(rules) && rules.items.length === 0) {
    doc.delete('peerDependencyRules');
  }
}

export function readPnpmWorkspacePeerDependencyRules(
  projectPath: string,
): { allowAny?: Array<string>; allowedVersions?: Record<string, string> } | undefined {
  const pnpmWorkspaceYamlPath = path.join(projectPath, 'pnpm-workspace.yaml');
  if (!fs.existsSync(pnpmWorkspaceYamlPath)) {
    return undefined;
  }
  const doc = readYamlFile(pnpmWorkspaceYamlPath) as {
    peerDependencyRules?: { allowAny?: Array<string>; allowedVersions?: Record<string, string> };
  } | null;
  return doc?.peerDependencyRules;
}

// Whether the root pnpm config (the package.json `pnpm` field or
// pnpm-workspace.yaml) holds a `vite` peer rule.
function hasVitePeerDependencyRule(projectPath: string): boolean {
  if (
    peerDependencyRulesHaveStaleEntry(readPnpmWorkspacePeerDependencyRules(projectPath), ['vite'])
  ) {
    return true;
  }
  const pkg = readPackageJsonIfExists(path.join(projectPath, 'package.json'));
  const pnpm = pkg && 'pnpm' in pkg && isJsonObject(pkg.pnpm) ? pkg.pnpm : undefined;
  const rules = pnpm?.peerDependencyRules;
  return peerDependencyRulesHaveStaleEntry(isObjectValue(rules) ? rules : undefined, ['vite']);
}

// Capture the `vite` peer rule before a migration step. Call the returned
// function after the step: it records the rule's removal in the report, so the
// migrate summary names it (see `vitePeerDependencyRuleRequired`).
export function trackVitePeerDependencyRuleRemoval(
  projectPath: string,
  packageManager: PackageManager,
  report: MigrationReport | undefined,
  pnpmVersion: string | undefined,
): () => void {
  if (
    !report ||
    packageManager !== PackageManager.pnpm ||
    vitePeerDependencyRuleRequired(VITE_PLUS_OVERRIDE_PACKAGES.vite, pnpmVersion) ||
    !hasVitePeerDependencyRule(projectPath)
  ) {
    return () => {};
  }
  return () => {
    if (!hasVitePeerDependencyRule(projectPath)) {
      report.vitePeerDependencyRuleRemoved = true;
    }
  };
}

export function ensurePnpmWorkspacePackages(
  projectPath: string,
  workspacePatterns: Array<string>,
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
