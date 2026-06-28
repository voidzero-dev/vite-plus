import fs from 'node:fs';
import path from 'node:path';

import { Scalar, YAMLMap } from 'yaml';

import { PackageManager, type WorkspacePackage } from '../../types/index.ts';
import {
  VITEST_VERSION,
  VITE_PLUS_NAME,
  VITE_PLUS_OVERRIDE_PACKAGES,
} from '../../utils/constants.ts';
import { readJsonFile } from '../../utils/json.ts';
import { detectPackageMetadata } from '../../utils/package.ts';
import {
  bootstrapProjectPaths,
  getCatalogDependencySpec,
  hasNuxtTestUtilsDependency,
  sourceTreeReferencesRetainedVitestModule,
  usesVitestBrowserMode,
  type BootstrapPackageJson,
} from '../migrator.ts';
import {
  LEGACY_WRAPPER_FALLBACK_VERSIONS,
  PROVIDER_OVERRIDE_DROP_NAMES,
  REMOVE_PACKAGES,
  VITEST_BROWSER_DEP_NAMES,
  VITEST_IS_MANAGED_OVERRIDE,
  type CatalogDependencyResolver,
  type PackageJsonDependencyField,
} from './shared.ts';

// Official `@vitest/*` packages are versioned in lockstep with vitest and carry
// an EXACT `vitest` peer (verified against the registry: `@vitest/coverage-v8`,
// `@vitest/coverage-istanbul`, `@vitest/ui`, `@vitest/web-worker`, the browser
// family, and the runtime internals all pin `vitest: <version>`), so any the
// project lists must match the bundled vitest or Vitest runs mixed copies (the
// `define-config.ts` coverage guard fail-fasts on exactly this skew).
// `@vitest/eslint-plugin` versions on its own line, and deprecated
// `@vitest/coverage-c8` never published on the Vitest 4 line, so neither may be
// pinned to the bundled Vitest version.
const VITEST_ALIGN_EXCLUDED = new Set([
  '@vitest/eslint-plugin',
  // Deprecated at 0.33.0 and replaced by @vitest/coverage-v8. It does not
  // publish versions on Vitest's current release line, so pinning it to the
  // bundled Vitest version creates a dependency spec that does not exist.
  '@vitest/coverage-c8',
]);

// Official packages that do not declare a required `vitest` peer. Keep them
// aligned when a project lists them directly, but do not add a direct vitest
// merely because they are present.
export const VITEST_DIRECT_USAGE_EXCLUDED = new Set([
  '@vitest/eslint-plugin',
  '@vitest/expect',
  '@vitest/mocker',
  '@vitest/pretty-format',
  '@vitest/runner',
  '@vitest/snapshot',
  '@vitest/spy',
  '@vitest/utils',
  '@vitest/ws-client',
]);

export function isAlignableVitestEcosystemPackage(name: string): boolean {
  return name.startsWith('@vitest/') && !VITEST_ALIGN_EXCLUDED.has(name);
}

// Extract the package name an override/resolution key *targets* — i.e. the
// package whose version would be forced. This mirrors the grammar of the real
// package-manager parsers (verified against `@yarnpkg/parsers` parseResolution):
//   - bare (`pkg`, `@scope/pkg`)
//   - versioned (`pkg@1`, `@scope/pkg@1`)
//   - pnpm parent selectors (`parent>pkg`, chained `a@1>b>@scope/pkg`)
//   - yarn `from/target` selectors (`parent/pkg`, `parent/@scope/pkg`,
//     `parent@1/pkg`, glob `**/pkg`)
// For a yarn `from/target` selector the forced package is the TRAILING
// descriptor, not the parent: `@scope/pkg@4/child` targets `child`, and an
// npm-alias key like `@scope/pkg@npm:@other/fork@1` is parsed by yarn as
// `from=@scope/pkg@npm:@other`, `descriptor=fork@1` — so the target is `fork`,
// NOT `@scope/pkg`. Taking the trailing descriptor is exactly that. (Yarn
// *rejects* keys whose range embeds a slash, e.g. `pkg@patch:…/…` or git/URL
// ranges, so those never reach us as valid keys and need no special handling.)
// Scoped names keep their leading `@` and internal `/`.
function extractOverrideTargetName(key: string): string {
  // pnpm parent selector `parent>child` (incl. chains `a>b>child`): the forced
  // package is the deepest child. pnpm splits at a `>` whose preceding char is
  // NOT space, `|`, or `@` — this is pnpm's own delimiter rule (DELIMITER_REGEX
  // = /[^ |@]>/ in @pnpm/parse-overrides) — so a semver comparator range such as
  // `pkg@>=4`, `pkg@>4`, or `>1 || >2` is NOT mistaken for a parent selector.
  // Peel parent levels until none remain, keeping the trailing child.
  let target = key.trim();
  for (let delim = target.search(/[^ |@]>/); delim !== -1; delim = target.search(/[^ |@]>/)) {
    target = target.slice(delim + 2).trim();
  }
  if (!target) {
    return target;
  }
  // yarn `from/target` selector: drop leading parent/glob segments, keeping the
  // trailing package descriptor (and a scoped name's own `/`).
  if (target.includes('/')) {
    const segments = target.split('/');
    const last = segments[segments.length - 1];
    const scope = segments[segments.length - 2];
    target = scope?.startsWith('@') ? `${scope}/${last}` : last;
  }
  // Strip a trailing version/range suffix. The version `@` follows the name
  // (after the `/` for a scoped name); the leading scope `@` is never a version
  // separator.
  const nameStart = target.startsWith('@') ? target.indexOf('/') + 1 : 0;
  const versionAt = target.indexOf('@', nameStart);
  if (versionAt > 0) {
    target = target.slice(0, versionAt);
  }
  return target;
}

// True iff a pnpm.overrides key's target (after stripping selector and
// version suffixes) is a provider whose stale pin must be dropped (see
// PROVIDER_OVERRIDE_DROP_NAMES). Shared by the JSON-object and YAMLMap
// variants below.
function isRemovePackageOverrideKey(key: string): boolean {
  return (PROVIDER_OVERRIDE_DROP_NAMES as readonly string[]).includes(
    extractOverrideTargetName(key),
  );
}

// Strip a trailing `@version`/range from a selector segment and keep its scope.
// Mirrors the version-suffix peeling in `extractOverrideTargetName`: the version
// `@` follows the name (after the `/` of a scoped name); the leading scope `@`
// is never a version separator.
function stripSegmentVersion(segment: string): string {
  const nameStart = segment.startsWith('@') ? segment.indexOf('/') + 1 : 0;
  const versionAt = segment.indexOf('@', nameStart);
  return versionAt > 0 ? segment.slice(0, versionAt) : segment;
}

// True iff a single parent-NAME glob segment matches the given literal package
// name. `*` matches any run of characters; all other glob/regex metacharacters
// are escaped. Used for the concrete ancestor segments of a selector.
function parentGlobMatchesName(glob: string, name: string): boolean {
  const pattern = glob
    .split('*')
    .map((part) => part.replace(/[.+?^${}()|[\]\\]/g, '\\$&'))
    .join('.*');
  return new RegExp(`^${pattern}$`).test(name);
}

// True iff an ancestor segment (literal or glob) matches the given package name.
function ancestorSegmentMatches(segment: string, name: string): boolean {
  return segment.includes('*') ? parentGlobMatchesName(segment, name) : segment === name;
}

// Provider names that sit on vite-plus's OWN dependency path and can therefore
// appear as ANCESTORS of a pin that still constrains vite-plus's provider
// subtree: pnpm/yarn parent selectors are not root-anchored, so a chain like
// `@vitest/browser-preview>@vitest/browser` forces the provider's child
// everywhere that provider appears — including under vite-plus's own direct
// provider dep. Only the vite-plus-supplied `@vitest/browser*` members of
// REMOVE_PACKAGES qualify; the user-owned opt-in providers (webdriverio,
// playwright) are deliberately NOT included — vite-plus no longer ships them, so
// a `@vitest/browser-playwright>…` chain constrains the user's own provider
// subtree, not vite-plus's (see the ACCEPTED EDGE note below).
const OWNED_PROVIDER_ANCESTOR_NAMES = (REMOVE_PACKAGES as readonly string[]).filter((name) =>
  name.startsWith('@vitest/'),
);

// True iff a selector's PARENT chain reaches vite-plus's OWN direct provider dep.
// The subtree migration protects is `<root> → vite-plus → @vitest/provider → …`;
// since vite-plus is a direct dependency of the project, a parent chain reaches
// that subtree iff it glob-matches a path along it:
//   - `**` segments match zero-or-more ancestors, so they are ignored here;
//   - the FIRST remaining concrete ancestor may glob-match `vite-plus`
//     (`vite-plus`, `vite-*`, `*`);
//   - every OTHER concrete ancestor must glob-match a vite-plus-owned provider
//     (`@vitest/browser*`), because un-anchored selectors such as
//     `@vitest/browser-playwright>@vitest/browser` still constrain the
//     provider's children under vite-plus.
// Any chain carrying a SPECIFIC unrelated ancestor (`some-parent>vite-plus`,
// `some-parent/**`, `some-parent/vite-*`, `some-app>@vitest/browser-playwright`)
// constrains a different subtree and does NOT touch the root vite-plus provider,
// so it is preserved. A chain of only `**` (`**`, `**/**`) is global and matches.
function parentChainReachesVitePlus(segments: string[]): boolean {
  const concrete = segments.filter((segment) => segment !== '**');
  let index = 0;
  if (concrete.length > 0 && ancestorSegmentMatches(concrete[0], VITE_PLUS_NAME)) {
    index = 1;
  }
  for (; index < concrete.length; index += 1) {
    const segment = concrete[index];
    if (!OWNED_PROVIDER_ANCESTOR_NAMES.some((name) => ancestorSegmentMatches(segment, name))) {
      return false;
    }
  }
  return true;
}

// Extract the ordered PARENT chain of an override/resolution key — the ancestor
// segments above the forced TARGET — or `null` when the key has no parent
// selector (a bare/versioned global pin). Each segment's own `@version`/range is
// stripped and scoped names (`@scope/name`) are kept whole; glob segments (`**`,
// `vite-*`) are preserved verbatim for `parentChainReachesVitePlus`.
//
// Mirrors `extractOverrideTargetName`'s grammar so target and parent stay
// consistent (see that function for the full delimiter rationale):
//   - pnpm `a>b>child`: every `>`-separated prefix is a parent level (`a`, `b`);
//     pnpm has no globs, so a chain of length > 1 always carries a specific
//     ancestor.
//   - yarn `from/descriptor`: the descriptor is the trailing 1 (unscoped) or 2
//     (scoped) segments; the remaining leading `/`-segments are the `from` chain,
//     with scoped ancestors (`@scope/name`) rejoined.
//   - bare/versioned names (`pkg`, `@scope/pkg`, `pkg@4`) have NO parent → `null`.
function extractOverrideParentSegments(key: string): string[] | null {
  let rest = key.trim();
  // Peel every pnpm `>` parent level. pnpm splits at a `>` whose preceding char
  // is NOT space, `|`, or `@` (its DELIMITER_REGEX), so semver comparators like
  // `pkg@>=4` are not mistaken for a parent selector.
  const pnpmParents: string[] = [];
  for (let delim = rest.search(/[^ |@]>/); delim !== -1; delim = rest.search(/[^ |@]>/)) {
    pnpmParents.push(stripSegmentVersion(rest.slice(0, delim + 1).trim()));
    rest = rest.slice(delim + 2).trim();
  }
  if (pnpmParents.length > 0) {
    return pnpmParents;
  }
  // No pnpm parent — check for a yarn `from/descriptor` selector. `rest` is the
  // child (target) descriptor; only a `/` beyond a single scoped name leaves a
  // leading `from` (parent) chain.
  if (!rest.includes('/')) {
    return null;
  }
  const segments = rest.split('/');
  // The trailing descriptor occupies the last 2 segments when it is a scoped
  // name (second-to-last segment starts with `@`), else the last 1.
  const descriptorIsScoped = segments[segments.length - 2]?.startsWith('@') ?? false;
  const descriptorSegmentCount = descriptorIsScoped ? 2 : 1;
  const rawParents = segments.slice(0, segments.length - descriptorSegmentCount);
  if (rawParents.length === 0) {
    // The whole key was a bare scoped name (`@scope/pkg`) — no parent selector.
    return null;
  }
  // Rejoin scoped ancestors (`@scope` + `name`) and strip each segment's version.
  const parents: string[] = [];
  for (let i = 0; i < rawParents.length; i += 1) {
    const segment = rawParents[i];
    if (segment.startsWith('@') && i + 1 < rawParents.length) {
      parents.push(stripSegmentVersion(`${segment}/${rawParents[i + 1]}`));
      i += 1;
    } else {
      parents.push(stripSegmentVersion(segment));
    }
  }
  return parents;
}

// True iff a provider override/resolution key (target ∈
// PROVIDER_OVERRIDE_DROP_NAMES) should be dropped because the pin would affect
// vite-plus's OWN direct provider dep. The pin reaches that dep iff its parent
// selector is:
//   1. ABSENT — bare/versioned global pin (`@vitest/browser-playwright`,
//      `@vitest/browser-playwright@4`).
//   2. a chain that glob-matches a path along the vite-plus provider subtree: a
//      pure glob (`**/...`, `*/...`), a name glob matching vite-plus
//      (`vite-*/...`), the literal `vite-plus` (`vite-plus>...`, `vite-plus/...`),
//      `**`-padded variants (`**/vite-plus/...`), or a chain whose remaining
//      ancestors are vite-plus-owned providers — un-anchored selectors such as
//      `@vitest/browser-preview>@vitest/browser` or nested npm
//      `{ "@vitest/browser-preview": { "@vitest/browser": … } }` still force
//      the provider's children under vite-plus. See
//      `parentChainReachesVitePlus`.
// A selector carrying a SPECIFIC unrelated ancestor anywhere in its chain
// (`some-app>@vitest/...`, `some-parent/@vitest/...`, `a>vite-plus>@vitest/...`,
// `some-parent/**/@vitest/...`, `some-parent/vite-*/@vitest/...`) or a mere
// wildcard RANGE on a specific parent (`parent@*/...`) only constrains that
// parent's subtree and is preserved. The parent chain comes from the KEY STRING
// for flat pnpm/yarn selectors; for npm/bun NESTED objects it is accumulated from
// the enclosing keys by `dropRemovePackageOverrideKeys` and passed in via
// `ancestorChain`, so a nested `{ a: { vite-plus: { provider } } }` is treated
// exactly like the flat `a>vite-plus>provider` (both preserved).
//
// ACCEPTED EDGE: reachability is judged from `vite-plus` only. A pnpm selector
// whose parent is the project's OWN (root/workspace) package name — which keeps
// an opt-in provider as a direct dep after migration, e.g.
// `my-app>@vitest/browser-webdriverio` or `my-app>@vitest/browser-playwright` —
// is therefore preserved even though it could re-pin that direct dep. Likewise a
// chain parented by an opt-in provider itself (`@vitest/browser-playwright>…`)
// constrains the USER's provider subtree, not vite-plus's, so it is preserved
// (the opt-in providers are excluded from OWNED_PROVIDER_ANCESTOR_NAMES).
// Dropping these would require threading importer names through this pass; per
// PR #1588 this is left as a known, visible (the pin stays in the manifest)
// limitation rather than risk over-deleting genuinely unrelated transitive
// selectors (the behavior the posted P2 review asked us to keep).
function providerKeyReachesVitePlus(key: string, ancestorChain: string[]): boolean {
  if (!isRemovePackageOverrideKey(key)) {
    return false;
  }
  const keyParents = extractOverrideParentSegments(key) ?? [];
  return parentChainReachesVitePlus([...ancestorChain, ...keyParents]);
}

// Flat-selector entry point (no enclosing object nesting): used by the
// pnpm-workspace YAML sweep, where each key carries its whole parent chain.
export function shouldDropProviderOverrideKey(key: string): boolean {
  return providerKeyReachesVitePlus(key, []);
}

// The ancestor segments a key contributes when the recursion descends into its
// object value: the key's own embedded selector parents followed by its target
// package name (version-stripped). For a plain npm/bun nested key (`a`) this is
// just `[a]`, so the accumulated chain mirrors a flat pnpm/yarn parent chain.
function childChainContribution(key: string): string[] {
  const parents = extractOverrideParentSegments(key) ?? [];
  return [...parents, extractOverrideTargetName(key)];
}

// Drop override keys whose target is a drop-listed provider AND whose pin would
// reach vite-plus's OWN direct provider dep — the edge `<root> → vite-plus →
// @vitest/provider`. Covers bare, versioned, global-glob and `vite-plus`-parent
// shapes that exact-key matching would miss. A pin scoped under a SPECIFIC
// non-vite-plus parent (pnpm `some-app>@vitest/...`, yarn `some-parent/@vitest/...`,
// or the npm/bun nested `{ "some-pkg": { "@vitest/...": "x" } }`) only constrains
// that parent's subtree and is PRESERVED.
//
// The decision is uniform across sinks: a provider pin is dropped iff its FULL
// ancestor chain reaches the root vite-plus edge (see `parentChainReachesVitePlus`).
// For flat pnpm/yarn selectors the whole chain lives in the KEY STRING; for npm/bun
// nested objects it is accumulated here from the enclosing object keys
// (`ancestorChain`) — so `{ "a": { "vite-plus": { provider } } }` is treated like
// the flat `a>vite-plus>provider` (both PRESERVED: vite-plus sits under `a`, not at
// the root). A long-form provider override (`{ "@vitest/browser-playwright": { ".":
// "x", "other": "y" } }`) has its own version pin (`.`) dropped while unrelated
// children (`other`) are kept. A parent we EMPTY by dropping its last pin is pruned
// so no meaningless `{}` is left; user-authored empties and untouched maps are kept.
// (pnpm/yarn override values are flat strings, so the recursion is inert for those
// sinks.) Returns whether any key/pin was removed.
export function dropRemovePackageOverrideKeys(
  overrides: Record<string, unknown> | undefined,
  ancestorChain: string[] = [],
): boolean {
  if (!overrides) {
    return false;
  }
  let removed = false;
  for (const key of Object.keys(overrides)) {
    const value = overrides[key];
    const child =
      value !== null && typeof value === 'object' && !Array.isArray(value)
        ? (value as Record<string, unknown>)
        : undefined;
    if (providerKeyReachesVitePlus(key, ancestorChain)) {
      if (child) {
        // Long-form provider override: drop the provider's own version pin (`.`)
        // but keep any unrelated child overrides scoped under it; still descend
        // (with the provider appended to the chain) for any deeper root pin.
        let changed = false;
        if ('.' in child) {
          delete child['.'];
          changed = true;
        }
        if (
          dropRemovePackageOverrideKeys(child, [...ancestorChain, ...childChainContribution(key)])
        ) {
          changed = true;
        }
        if (Object.keys(child).length === 0) {
          delete overrides[key];
          changed = true;
        }
        if (changed) {
          removed = true;
        }
      } else {
        delete overrides[key];
        removed = true;
      }
      continue;
    }
    if (child) {
      // Not a root-vite-plus provider pin here: descend with the chain extended by
      // this key so a deeper pin sees its full ancestor path; prune the parent only
      // if the descent emptied it.
      if (
        dropRemovePackageOverrideKeys(child, [...ancestorChain, ...childChainContribution(key)])
      ) {
        removed = true;
        if (Object.keys(child).length === 0) {
          delete overrides[key];
        }
      }
    }
  }
  return removed;
}

// The managed override/catalog packages vite-plus writes and the detector
// requires. `vite` is ALWAYS managed (aliased to vite-plus-core). `vitest` is
// managed ONLY when the project uses vitest DIRECTLY — vite-plus consumes
// upstream vitest itself, so a non-vitest project gets it transitively through
// vite-plus and must NOT carry a managed `vitest` pin (which would drift on a
// future `vp update vite-plus`). When `usesVitest` is false the common-case
// removal logic ACTIVELY strips any lingering `vitest` entry.
export function managedOverridePackages(usesVitest: boolean): Record<string, string> {
  if (usesVitest) {
    return VITE_PLUS_OVERRIDE_PACKAGES;
  }
  // Drop only `vitest`; every other managed key (e.g. `vite`, and in
  // force-override/CI mode the `@voidzero-dev/vite-plus-core` file: alias) stays.
  return Object.fromEntries(
    Object.entries(VITE_PLUS_OVERRIDE_PACKAGES).filter(([key]) => key !== 'vitest'),
  );
}

// True iff a dependency field lists a vitest ecosystem package — any name that
// contains `vitest` other than bare `vitest` itself (e.g. `@vitest/coverage-v8`,
// `@vitest/browser-playwright`, `vitest-browser-svelte`). A bare `vitest`
// dependency alone is deliberately NOT a signal — a prior migration may have
// injected it transitively-redundantly, so it must not keep the project pinned
// to a managed `vitest`. This mirrors the `isVitestAdjacent` signal used later
// when deciding to inject a direct `vitest`, so the two stay consistent.
function projectListsVitestEcosystemDep(pkg: {
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
  optionalDependencies?: Record<string, string>;
  peerDependencies?: Record<string, string>;
}): boolean {
  // Peer declarations do not install the package in this project; its consumer
  // is responsible for satisfying that package's peers.
  const dependencyGroups = [pkg.dependencies, pkg.devDependencies, pkg.optionalDependencies];
  return dependencyGroups.some((deps) =>
    deps
      ? Object.keys(deps).some(
          (name) =>
            name !== 'vitest' &&
            name.includes('vitest') &&
            // Excluded official packages either have no vitest peer or (for the
            // ESLint plugin) only an optional `vitest: *` peer. Neither needs a
            // direct install or workspace-wide override.
            !VITEST_DIRECT_USAGE_EXCLUDED.has(name),
        )
      : false,
  );
}

// Detect installed dependencies whose package metadata declares a required
// Vitest peer. Package names are not authoritative: integrations such as
// `vite-plugin-gherkin` require Vitest without containing "vitest" in their
// own name. Optional peers do not require package-local provisioning.
export function projectListsRequiredVitestPeer(
  projectPath: string,
  pkg: {
    dependencies?: Record<string, string>;
    devDependencies?: Record<string, string>;
    optionalDependencies?: Record<string, string>;
  },
): boolean {
  const installGroups = [pkg.dependencies, pkg.devDependencies, pkg.optionalDependencies];
  const hasExistingVitest = installGroups.some(
    (dependencies) => dependencies?.vitest !== undefined,
  );
  const dependencyNames = new Set([
    ...Object.keys(pkg.dependencies ?? {}),
    ...Object.keys(pkg.devDependencies ?? {}),
    ...Object.keys(pkg.optionalDependencies ?? {}),
  ]);
  dependencyNames.delete('vitest');
  dependencyNames.delete('vite');
  dependencyNames.delete(VITE_PLUS_NAME);
  for (const name of VITEST_DIRECT_USAGE_EXCLUDED) {
    dependencyNames.delete(name);
  }
  let metadataUnavailable = false;

  for (const name of dependencyNames) {
    const metadata = detectPackageMetadata(projectPath, name);
    if (!metadata) {
      metadataUnavailable = true;
      continue;
    }
    try {
      const installedPkg = readJsonFile(path.join(metadata.path, 'package.json')) as {
        peerDependencies?: Record<string, string>;
        peerDependenciesMeta?: Record<string, { optional?: boolean }>;
      };
      if (
        typeof installedPkg.peerDependencies?.vitest === 'string' &&
        installedPkg.peerDependenciesMeta?.vitest?.optional !== true
      ) {
        return true;
      }
    } catch {
      metadataUnavailable = true;
    }
  }
  // A clean checkout may not have node_modules/.pnp metadata yet. If the user
  // already carries a direct Vitest while any dependency's peer contract is
  // unknown, preserve it rather than risk removing the provider for an
  // arbitrary integration such as vite-plugin-gherkin. A later migration with
  // complete metadata can safely remove a genuinely redundant pin.
  return metadataUnavailable && hasExistingVitest;
}

// True iff the project uses vitest DIRECTLY — via a dependency that is expected
// to have a required vitest peer (see `projectListsVitestEcosystemDep`), an
// upstream `vitest` module specifier, a package-level @nuxt/test-utils
// compatibility boundary, or vitest browser mode. Drives
// whether the migration keeps `vitest` managed or removes it entirely; the
// browser-mode arm keeps it aligned with the direct-`vitest` injection below so
// an injected `catalog:` spec never dangles against a vitest-less catalog.
export function projectUsesVitestDirectly(
  projectPath: string,
  pkg: {
    dependencies?: Record<string, string>;
    optionalDependencies?: Record<string, string>;
    devDependencies?: Record<string, string>;
    peerDependencies?: Record<string, string>;
  },
  // Lazily computed when omitted, after the cheap ecosystem-dep check below
  // short-circuits, mirroring the precomputedScans pattern. Avoids the
  // dependency scan when the project already lists a vitest-ecosystem dep.
  requiredVitestPeer?: boolean,
  preserveNuxtVitestImports = true,
  // Optional precomputed source-tree scan results. Callers that already computed
  // these for the same `projectPath` at the same point (no source mutation in
  // between) thread them here to avoid re-traversing the source tree. When
  // omitted, the scans run lazily as before, preserving short-circuit behavior.
  precomputedScans?: { browserMode: boolean; retainedModule: boolean },
): boolean {
  return (
    projectListsVitestEcosystemDep(pkg) ||
    (requiredVitestPeer ?? projectListsRequiredVitestPeer(projectPath, pkg)) ||
    // Browser packages declared only as peers still become direct installs:
    // rewritePackageJson/reconcileVitePlusBootstrapPackage promote opt-in
    // providers into devDependencies and treat the bundled browser packages as
    // browser-mode intent. Account for that promotion before shared
    // catalog/override ownership is decided, otherwise the promoted provider's
    // exact Vitest peer is left unsatisfied under strict pnpm/Yarn layouts.
    VITEST_BROWSER_DEP_NAMES.some((name) => pkg.peerDependencies?.[name] !== undefined) ||
    (precomputedScans?.retainedModule ?? sourceTreeReferencesRetainedVitestModule(projectPath)) ||
    (preserveNuxtVitestImports && hasNuxtTestUtilsDependency(pkg)) ||
    (precomputedScans?.browserMode ?? usesVitestBrowserMode(projectPath))
  );
}

// Remove a managed `vitest` key from a flat string-valued record (dependency
// field, npm/bun overrides, yarn resolutions, pnpm.overrides, a catalog object).
// Only a STRING value is removed: a managed pin, `catalog:` reference, or wrapper
// alias is always a string, whereas a nested object value (npm/bun `overrides`)
// is a user override scoped under `vitest` and must be left intact. Returns true
// iff an entry was removed.
export function removeManagedVitestEntry(record: Record<string, string> | undefined): boolean {
  if (VITEST_IS_MANAGED_OVERRIDE && typeof record?.vitest === 'string') {
    delete record.vitest;
    return true;
  }
  return false;
}

// Remove a managed `vitest` scalar key from a YAMLMap (pnpm-workspace.yaml
// `overrides`, `catalog`, and each named `catalogs` entry).
export function removeYamlMapVitestEntry(map: unknown): void {
  if (!VITEST_IS_MANAGED_OVERRIDE || !(map instanceof YAMLMap)) {
    return;
  }
  const target = map.items.find(
    (item) => item.key instanceof Scalar && item.key.value === 'vitest',
  )?.key;
  if (target) {
    map.delete(target);
  }
}

// Remove the managed `vitest` entry from pnpm peerDependencyRules (its
// `allowAny` array entry and `allowedVersions.vitest`), in place. Works on both
// the package.json `pnpm.peerDependencyRules` JSON shape and the same shape read
// back from pnpm-workspace.yaml.
export function removeVitestPeerDependencyRule(peerDependencyRules: {
  allowAny?: string[];
  allowedVersions?: Record<string, string>;
}): void {
  if (!VITEST_IS_MANAGED_OVERRIDE) {
    return;
  }
  if (Array.isArray(peerDependencyRules.allowAny)) {
    peerDependencyRules.allowAny = peerDependencyRules.allowAny.filter((key) => key !== 'vitest');
  }
  if (peerDependencyRules.allowedVersions) {
    delete peerDependencyRules.allowedVersions.vitest;
  }
}

// Legacy wrapper package names that may appear as the target of override
// aliases left over from earlier vite-plus migrations. `@voidzero-dev/vite-plus-test`
// was deleted; any catalog/override entry still pointing at it is stale.
const LEGACY_WRAPPER_PACKAGE_NAMES = ['@voidzero-dev/vite-plus-test'] as const;

export function isLegacyWrapperSpec(value: unknown): boolean {
  // A wrapper spec is always a flat string range; npm/bun `overrides` may hold
  // nested object values, which can never themselves be a wrapper alias (the
  // recursion in `pruneLegacyWrapperAliases` descends into those).
  if (typeof value !== 'string' || !value) {
    return false;
  }
  for (const name of LEGACY_WRAPPER_PACKAGE_NAMES) {
    if (value === `npm:${name}` || value.startsWith(`npm:${name}@`)) {
      return true;
    }
  }
  return false;
}

/**
 * Rewrite or remove keys whose value points at a deleted vite-plus wrapper.
 * When a fallback exists for the key (e.g. `vitest`), the value is replaced
 * so existing `catalog:` references continue to resolve. Otherwise the key
 * is dropped entirely. Returns true iff any entry was changed.
 *
 * npm/bun `overrides` may nest an object of scoped overrides under a parent
 * key (e.g. `{ "some-parent": { "vitest": "npm:@voidzero-dev/vite-plus-test@latest" } }`),
 * so object values are recursed into; a parent emptied by pruning is dropped so
 * no `{}` is left behind. Flat maps (pnpm `overrides`, yarn `resolutions`,
 * catalogs) hold only string values, where the recursion is inert.
 */
export function pruneLegacyWrapperAliases(record: Record<string, unknown> | undefined): boolean {
  if (!record) {
    return false;
  }
  let mutated = false;
  for (const key of Object.keys(record)) {
    const value = record[key];
    if (value !== null && typeof value === 'object' && !Array.isArray(value)) {
      if (pruneLegacyWrapperAliases(value as Record<string, unknown>)) {
        mutated = true;
        if (Object.keys(value as Record<string, unknown>).length === 0) {
          delete record[key];
        }
      }
      continue;
    }
    if (isLegacyWrapperSpec(value)) {
      const fallback = LEGACY_WRAPPER_FALLBACK_VERSIONS[key];
      if (fallback !== undefined) {
        record[key] = fallback;
      } else {
        delete record[key];
      }
      mutated = true;
    }
  }
  return mutated;
}

export function getAlignedVitestEcosystemDependencySpec(
  current: string,
  dependencyName: string,
  dependencyField: PackageJsonDependencyField,
  packageManager: PackageManager,
  supportCatalog: boolean,
  catalogDependencyResolver?: CatalogDependencyResolver,
): string {
  const catalogSpec = current.startsWith('catalog:') ? current : 'catalog:';
  const catalogSupported =
    supportCatalog && catalogDependencyResolver?.(catalogSpec, dependencyName) !== undefined;
  return getCatalogDependencySpec(current, VITEST_VERSION, catalogSupported, {
    dependencyField,
    dependencyName,
    packageManager,
    catalogDependencyResolver,
    preferredCatalogSpec: catalogDependencyResolver?.preferredCatalogSpec,
  });
}

// Align every declared official `@vitest/*` package with the bundled Vitest.
// Prefer an existing default or named catalog entry when the package manager
// supports catalogs; otherwise use the concrete bundled version. Returns true
// if any package.json spec changed. Catalog values are reconciled separately by
// the package-manager config writers above.
export function alignVitestEcosystemPackages(
  pkg: BootstrapPackageJson,
  packageManager: PackageManager,
  supportCatalog: boolean,
  catalogDependencyResolver?: CatalogDependencyResolver,
): boolean {
  if (!VITEST_IS_MANAGED_OVERRIDE) {
    return false;
  }
  const dependencyGroups: Array<{
    dependencyField: PackageJsonDependencyField;
    dependencies: Record<string, string> | undefined;
  }> = [
    { dependencyField: 'devDependencies', dependencies: pkg.devDependencies },
    { dependencyField: 'dependencies', dependencies: pkg.dependencies },
    { dependencyField: 'optionalDependencies', dependencies: pkg.optionalDependencies },
  ];
  let changed = false;
  for (const { dependencyField, dependencies } of dependencyGroups) {
    if (!dependencies) {
      continue;
    }
    for (const name of Object.keys(dependencies)) {
      if (!isAlignableVitestEcosystemPackage(name)) {
        continue;
      }
      const aligned = getAlignedVitestEcosystemDependencySpec(
        dependencies[name],
        name,
        dependencyField,
        packageManager,
        supportCatalog,
        catalogDependencyResolver,
      );
      if (dependencies[name] !== aligned) {
        dependencies[name] = aligned;
        changed = true;
      }
    }
  }
  return changed;
}

export function vitestEcosystemCatalogReferencesPending(
  pkg: BootstrapPackageJson,
  catalogDependencyResolver?: CatalogDependencyResolver,
): boolean {
  if (!VITEST_IS_MANAGED_OVERRIDE || !catalogDependencyResolver) {
    return false;
  }
  for (const dependencies of [pkg.devDependencies, pkg.dependencies, pkg.optionalDependencies]) {
    if (!dependencies) {
      continue;
    }
    for (const [name, spec] of Object.entries(dependencies)) {
      if (
        isAlignableVitestEcosystemPackage(name) &&
        spec.startsWith('catalog:') &&
        catalogDependencyResolver(spec, name) !== VITEST_VERSION
      ) {
        return true;
      }
    }
  }
  return false;
}

export function collectVitestEcosystemInstallDependencyNames(
  rootDir: string,
  packages?: WorkspacePackage[],
): Set<string> {
  const names = new Set<string>();
  for (const packagePath of bootstrapProjectPaths(rootDir, packages)) {
    const packageJsonPath = path.join(packagePath, 'package.json');
    if (!fs.existsSync(packageJsonPath)) {
      continue;
    }
    const pkg = readJsonFile(packageJsonPath) as BootstrapPackageJson;
    for (const dependencies of [pkg.devDependencies, pkg.dependencies, pkg.optionalDependencies]) {
      for (const name of Object.keys(dependencies ?? {})) {
        if (isAlignableVitestEcosystemPackage(name)) {
          names.add(name);
        }
      }
    }
  }
  return names;
}
