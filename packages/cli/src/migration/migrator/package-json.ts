import { rewriteScripts } from '../../../binding/index.js';
import { PackageManager } from '../../types/index.ts';
import {
  VITEST_VERSION,
  VITE_PLUS_NAME,
  VITE_PLUS_OVERRIDE_PACKAGES,
  VITE_PLUS_VERSION,
  isForceOverrideMode,
} from '../../utils/constants.ts';
import {
  VITEST_DIRECT_USAGE_EXCLUDED,
  alignVitestEcosystemPackages,
  ensureDirectViteForPnpm,
  getAlignedVitestEcosystemDependencySpec,
  getCatalogDependencySpec,
  getScriptRulesYaml,
  hasNuxtTestUtilsDependency,
  managedOverridePackages,
  normalizeVitestPeerCatalogSpec,
  pruneLegacyWrapperAliases,
  readRulesYaml,
  removeManagedVitestEntry,
  setDirectViteEdge,
} from '../migrator.ts';
import {
  BROWSER_PROVIDER_PEER_DEPS,
  findDeclaredSpec,
  resolveProviderPeerSpec,
  OPT_IN_BROWSER_PROVIDERS,
  REMOVE_PACKAGES,
  VITEST_BROWSER_DEP_NAMES,
  VITEST_IS_MANAGED_OVERRIDE,
  type CatalogDependencyResolver,
  type PackageJsonDependencyField,
} from './shared.ts';

export function rewritePackageJson(
  pkg: {
    scripts?: Record<string, string>;
    'lint-staged'?: Record<string, string | string[]>;
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
    peerDependencies?: Record<string, string>;
    optionalDependencies?: Record<string, string>;
  },
  packageManager: PackageManager,
  isMonorepo?: boolean,
  skipStagedMigration?: boolean,
  catalogDependencyResolver?: CatalogDependencyResolver,
  vitestBrowserMode?: boolean,
  // Source-scan signal per opt-in browser provider name (e.g.
  // `@vitest/browser-webdriverio` → true). A provider with no dep declared but
  // imported in source still gets kept/injected.
  providerSourceModes?: Partial<Record<string, boolean>>,
  // Whether the project uses vitest DIRECTLY (a required-peer consumer, an
  // upstream module reference, or browser mode). `vitest` is managed only
  // when true; in the common case (`false`) a lingering managed `vitest` entry
  // is REMOVED so it arrives transitively through vite-plus. Defaults to true to
  // preserve legacy behavior for callers that don't compute the signal.
  usesVitestDirectly = true,
  // Module augmentations, compilerOptions.types, and `vitest/package.json`
  // intentionally retain the upstream package identity after import rewriting
  // and therefore require a package-local provider under strict layouts.
  retainedVitestModule = false,
  // Installed dependency metadata can reveal required Vitest peers whose
  // package names do not include "vitest".
  requiredVitestPeer = false,
  // Opt-in browser providers the workspace catalog is gaining (some package uses
  // one only through source/a shim). An already-installed copy of such a provider
  // must REFERENCE that catalog entry, not pin a concrete version. See #2005.
  providerCatalogAdditions: ReadonlySet<string> = new Set(),
): Record<string, string | string[]> | null {
  if (pkg.scripts) {
    const updated = rewriteScripts(
      JSON.stringify(pkg.scripts),
      getScriptRulesYaml(skipStagedMigration),
    );
    if (updated) {
      pkg.scripts = JSON.parse(updated);
    }
  }
  // Extract staged config from package.json (lint-staged) → will be merged into vite.config.ts.
  // The lint-staged key is NOT deleted here — it's removed by the caller only after
  // the merge into vite.config.ts succeeds, to avoid losing config on merge failure.
  let extractedStagedConfig: Record<string, string | string[]> | null = null;
  if (!skipStagedMigration && pkg['lint-staged']) {
    const config = pkg['lint-staged'];
    const updated = rewriteScripts(JSON.stringify(config), readRulesYaml());
    extractedStagedConfig = updated ? JSON.parse(updated) : config;
  }
  const supportCatalog = !!isMonorepo && packageManager !== PackageManager.npm;
  let needVitePlus = false;
  const dependencyGroups: {
    dependencyField: PackageJsonDependencyField;
    dependencies: Record<string, string> | undefined;
  }[] = [
    { dependencyField: 'devDependencies', dependencies: pkg.devDependencies },
    { dependencyField: 'dependencies', dependencies: pkg.dependencies },
    { dependencyField: 'peerDependencies', dependencies: pkg.peerDependencies },
    { dependencyField: 'optionalDependencies', dependencies: pkg.optionalDependencies },
  ];
  // Scrub stale `npm:@voidzero-dev/vite-plus-test@...` aliases left over from
  // earlier vite-plus migrations — the wrapper package no longer exists, so
  // these entries would break `pnpm install`. Real user ranges are preserved.
  for (const { dependencies } of dependencyGroups) {
    if (pruneLegacyWrapperAliases(dependencies)) {
      needVitePlus = true;
    }
  }
  const managed = managedOverridePackages(usesVitestDirectly);
  // Common case (no direct vitest): vite-plus consumes upstream vitest itself,
  // so ACTIVELY REMOVE any lingering managed `vitest` dependency (a managed pin,
  // a `catalog:` reference, or a stale wrapper alias already normalized above) —
  // it arrives transitively through vite-plus and a future `vp update vite-plus`
  // keeps it correct with no pin to drift. The `@vitest/*` family and unrelated
  // keys are untouched. (Browser-mode / vitest-adjacent projects re-add a direct
  // `vitest` below; those are direct-usage signals, so this never strips one a
  // surviving consumer needs.)
  if (!usesVitestDirectly) {
    // Only the INSTALL groups — a `peerDependencies` `vitest` is a declaration
    // about consumers, not an install pin, so it is not removed here. Catalog
    // peer specs are resolved to their public range/fallback below.
    for (const { dependencyField, dependencies } of dependencyGroups) {
      if (dependencyField === 'peerDependencies') {
        continue;
      }
      if (removeManagedVitestEntry(dependencies)) {
        needVitePlus = true;
      }
    }
  }
  for (const [key, version] of Object.entries(managed)) {
    for (const { dependencyField, dependencies } of dependencyGroups) {
      if (dependencies?.[key]) {
        dependencies[key] = getCatalogDependencySpec(dependencies[key], version, supportCatalog, {
          dependencyField,
          dependencyName: key,
          packageManager,
          catalogDependencyResolver,
          preferredCatalogSpec: catalogDependencyResolver?.preferredCatalogSpec,
        });
        needVitePlus = true;
      }
    }
  }
  if (normalizeVitestPeerCatalogSpec(pkg.peerDependencies, catalogDependencyResolver)) {
    needVitePlus = true;
  }
  // Optional Vitest packages are published in lockstep with the runner. Keep
  // every declared official @vitest/* package on the bundled version during a
  // fresh migration too; existing-Vite+ upgrades use the same rule in the
  // bootstrap path.
  alignVitestEcosystemPackages(pkg, packageManager, supportCatalog, catalogDependencyResolver);
  // Force-override mode (ecosystem CI / `vp create` E2E) must re-pin any
  // pre-existing `vite-plus` range to the local tgz. Otherwise pnpm reads the
  // published vite-plus metadata for transitive dep resolution (e.g.
  // `@voidzero-dev/vite-plus-test`) even though the override replaces the
  // vite-plus package itself, dragging the stale wrapper into node_modules.
  if (isForceOverrideMode()) {
    for (const { dependencies } of dependencyGroups) {
      if (dependencies?.[VITE_PLUS_NAME]) {
        // The referenced catalog entry is rewritten to the preview target
        // separately. Preserve named/default catalog references so projects
        // such as Vize do not gain an unnecessary default catalog.
        if (
          !supportCatalog ||
          VITE_PLUS_VERSION.startsWith('file:') ||
          !dependencies[VITE_PLUS_NAME].startsWith('catalog:')
        ) {
          dependencies[VITE_PLUS_NAME] = VITE_PLUS_VERSION;
        }
        needVitePlus = true;
      }
    }
  }
  // Capture browser-mode signal from the original deps BEFORE the removal loop
  // strips them. A package can drive vitest browser mode purely through config
  // (`test.browser.provider: 'playwright'` in `vite.config.ts`) without ever
  // importing `@vitest/browser*` in source — the provider package is listed in
  // devDependencies but vitest loads it by name. The source-scan signal
  // (`usesVitestBrowserMode`) misses this case; the dep declaration is the
  // authoritative intent signal.
  const hasBrowserDepSignal = VITEST_BROWSER_DEP_NAMES.some((name) =>
    dependencyGroups.some(({ dependencies }) => dependencies?.[name] !== undefined),
  );
  // remove packages that are replaced with vite-plus
  for (const name of REMOVE_PACKAGES) {
    let wasRemoved = false;
    for (const { dependencies } of dependencyGroups) {
      if (dependencies?.[name]) {
        delete dependencies[name];
        wasRemoved = true;
      }
    }
    if (wasRemoved) {
      needVitePlus = true;
    }
  }
  // The browser providers (webdriverio, playwright) are opt-in: vite-plus no
  // longer bundles them at runtime (each drags a heavy non-optional framework
  // peer), so a user targeting a provider must own it themselves for the
  // rewritten `vite-plus/test/browser-<provider>` import to resolve. Unlike the
  // rest of the `@vitest/*` family they are deliberately NOT in
  // VITE_PLUS_OVERRIDE_PACKAGES (so projects not using a provider stay
  // untouched), which means the normalization loop above does not add them. We
  // align each installed provider here using its existing catalog when present,
  // or the concrete bundled version otherwise, and ensure its runtime framework
  // peer (`webdriverio` / `playwright`). (`@vitest/browser`/preview stay bundled
  // + stripped, handled in the REMOVE_PACKAGES loop above.)
  let usesAnyOptInProvider = false;
  for (const provider of OPT_IN_BROWSER_PROVIDERS) {
    const usesProvider =
      providerSourceModes?.[provider] ||
      dependencyGroups.some(({ dependencies }) => dependencies?.[provider] !== undefined);
    if (!usesProvider) {
      continue;
    }
    usesAnyOptInProvider = true;
    // The provider must be INSTALLED (in deps/devDeps/optionalDeps, not merely a
    // peer) for the rewritten `vite-plus/test/browser-<provider>` import to
    // resolve. Normalize an existing install-group declaration to the bundled
    // vitest version in place (the override loop above no longer pins it);
    // otherwise — a source-only or peer-only user — inject it into devDeps.
    const installGroupEntry = dependencyGroups.find(
      ({ dependencyField, dependencies }) =>
        dependencyField !== 'peerDependencies' && dependencies?.[provider] !== undefined,
    );
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
    const peer = BROWSER_PROVIDER_PEER_DEPS[provider]; // 'webdriverio' / 'playwright'
    const peerPresent = findDeclaredSpec(pkg, peer);
    if (peer && !peerPresent) {
      pkg.devDependencies ??= {};
      pkg.devDependencies[peer] = resolveProviderPeerSpec(
        pkg,
        peer,
        supportCatalog,
        catalogDependencyResolver,
      );
    }
    needVitePlus = true;
  }
  // An opt-in browser provider drags in its OWN `@vitest/browser → @vitest/mocker`
  // subtree that is distinct from the one vite-plus bundles, so npm's flat
  // node_modules cannot dedupe the two and leaves several nested `@vitest/mocker`
  // copies. `@vitest/mocker/dist/node.js` statically `import`s `vite` (its `vite`
  // peer is optional, so install never errors), and the `vite` override only lands
  // deep inside the `vitest` subtree — unreachable from the nested provider chain.
  // The result is `ERR_MODULE_NOT_FOUND: Cannot find package 'vite'` when loading
  // the browser config. Mirror the override as a direct `vite` devDep (as the bun
  // branch already does for its own resolver) so npm hoists a single top-level
  // `node_modules/vite` that every nested `@vitest/mocker` resolves. Gated on
  // provider usage so non-browser (node-mode) projects — which dedupe cleanly and
  // need no direct `vite` — stay untouched. pnpm/yarn use symlink/PnP layouts that
  // already expose the override to the provider subtree, so this is npm-only.
  if (usesAnyOptInProvider && packageManager === PackageManager.npm) {
    const viteOverride = VITE_PLUS_OVERRIDE_PACKAGES.vite;
    const viteAlreadyDirect =
      pkg.dependencies?.vite ?? pkg.devDependencies?.vite ?? pkg.optionalDependencies?.vite;
    if (viteOverride && !viteAlreadyDirect) {
      // npm has no catalog (supportCatalog=false), so the shared helper resolves
      // the direct edge to the concrete core alias, just placed in sorted order.
      setDirectViteEdge(pkg, supportCatalog, catalogDependencyResolver);
      needVitePlus = true;
    }
  }
  // Promote dep-derived signal to the same flag the source-scan feeds, so the
  // downstream "add direct `vitest`" branch fires for config-only browser-mode
  // setups too.
  const effectiveBrowserMode = vitestBrowserMode || hasBrowserDepSignal;
  // Trigger vite-plus install when a project has a vitest-adjacent package
  // (e.g. `vitest-browser-svelte`) that declares vitest as a peer dep — even
  // if the project has no vite/oxlint/tsdown dep to migrate. Only installed
  // dependency groups count; a peer declaration alone installs nothing here.
  const installableNames = [
    ...Object.keys(pkg.dependencies ?? {}),
    ...Object.keys(pkg.devDependencies ?? {}),
    ...Object.keys(pkg.optionalDependencies ?? {}),
  ];
  const isVitestAdjacent =
    !installableNames.includes('vitest') &&
    installableNames.some(
      (name) =>
        name !== 'vitest' && name.includes('vitest') && !VITEST_DIRECT_USAGE_EXCLUDED.has(name),
    );
  // A @nuxt/test-utils package keeps its `from 'vitest'` imports (the import
  // rewriter preserves them for Nuxt packages), so it needs a package-local
  // `vitest` under strict pnpm/Yarn layouts even though its name doesn't
  // contain "vitest".
  const hasNuxtTestUtils = hasNuxtTestUtilsDependency(pkg);
  // Normalize a pre-existing pinned vite-plus so sub-packages don't drift
  // from siblings: in catalog-supporting monorepos that's `catalog:`, under
  // force-override (file:) it's the tgz path. Preserve protocol-prefixed
  // specs (catalog:named, workspace:*, link:, file:, npm:, github:, git+/git:,
  // http(s)://) so deliberate user pins survive; only vanilla version ranges
  // (e.g. `^0.1.20`, `latest`) are rewritten.
  const canonicalVitePlusSpec =
    supportCatalog && !VITE_PLUS_VERSION.startsWith('file:')
      ? (catalogDependencyResolver?.preferredCatalogSpec ?? 'catalog:')
      : VITE_PLUS_VERSION;
  // Treat vite-plus as present when it lives in either `devDependencies` or
  // `dependencies` (devDeps wins when both exist). Re-pin/normalize happens in
  // whichever group already owns it so a `dependencies` entry is never
  // duplicated into `devDependencies`.
  const existingVitePlusGroup =
    pkg.devDependencies?.[VITE_PLUS_NAME] !== undefined
      ? pkg.devDependencies
      : pkg.dependencies?.[VITE_PLUS_NAME] !== undefined
        ? pkg.dependencies
        : undefined;
  const existingVitePlus = existingVitePlusGroup?.[VITE_PLUS_NAME];
  const shouldNormalizeExistingVitePlus =
    !!existingVitePlus &&
    supportCatalog &&
    existingVitePlus !== canonicalVitePlusSpec &&
    !isProtocolPinnedSpec(existingVitePlus);
  // vitest-adjacent / browser-mode signals only trigger a vite-plus INSTALL when the
  // project doesn't already have vite-plus — otherwise vite-plus is already present and
  // re-adding it would be churn. (The direct `vitest` pin those signals also require is
  // decided separately below, independent of whether vite-plus is present.)
  if (!existingVitePlus && (isVitestAdjacent || effectiveBrowserMode)) {
    needVitePlus = true;
  }
  // Browser mode AND a vitest-adjacent dep (e.g. `vitest-browser-svelte`, which
  // declares a non-optional `vitest` peer) both need a direct `vitest` pin INDEPENDENT
  // of whether `vite-plus` is already present: that peer must resolve from the package's
  // OWN root under pnpm strict / Yarn PnP, where `vite-plus`'s transitive `vitest` is not
  // visible. Tracked separately from `needVitePlus` so the pin is added without re-adding
  // an already-present `vite-plus` — e.g. a monorepo root, where
  // `rewriteRootWorkspacePackageJson` injects `vite-plus` BEFORE this runs (so
  // `existingVitePlus` is already truthy here), or a re-migration of a project that
  // already owns it. The guard below still no-ops when a direct `vitest` already exists,
  // so a genuine normalize pass of an already-correct project mutates nothing.
  const needDirectVitest =
    needVitePlus ||
    effectiveBrowserMode ||
    isVitestAdjacent ||
    retainedVitestModule ||
    requiredVitestPeer ||
    hasNuxtTestUtils;
  if (existingVitePlusGroup) {
    // Already present in `dependencies` or `devDependencies`: re-pin in place
    // (only vanilla ranges are normalized; protocol pins are preserved) and
    // never add a cross-group duplicate.
    if (shouldNormalizeExistingVitePlus) {
      existingVitePlusGroup[VITE_PLUS_NAME] = canonicalVitePlusSpec;
    }
  } else if (needVitePlus) {
    // Absent from both groups: add it to `devDependencies` as before.
    pkg.devDependencies = {
      ...pkg.devDependencies,
      [VITE_PLUS_NAME]: canonicalVitePlusSpec,
    };
  }
  ensureDirectViteForPnpm(pkg, packageManager, supportCatalog, catalogDependencyResolver);
  // Add `vitest` as a direct devDependency when:
  //  - a remaining dependency likely peer-depends on vitest (e.g.
  //    vitest-browser-svelte), OR
  //  - the package runs vitest browser mode (`@vitest/browser` needs
  //    `vitest` resolvable from the package root — see usesVitestBrowserMode).
  // Vite-plus already bundles upstream vitest as a direct dep, but a strict
  // pnpm / yarn Plug'n'Play layout will not expose that transitive `vitest`
  // to the package. Pinning it here points the dep at the same upstream
  // version vite-plus ships with. Gated by needDirectVitest (browser-mode /
  // vitest-adjacent, or some other change) — a pure normalize pass must not
  // mutate the project beyond the vite-plus spec.
  if (needDirectVitest) {
    const installableDeps = {
      ...pkg.dependencies,
      ...pkg.devDependencies,
      ...pkg.optionalDependencies,
    };
    if (
      !installableDeps.vitest &&
      (effectiveBrowserMode ||
        retainedVitestModule ||
        requiredVitestPeer ||
        // Only a genuinely vitest-adjacent dep (excludes `@vitest/eslint-plugin`
        // etc. via VITEST_DIRECT_USAGE_EXCLUDED) or a Nuxt package with preserved
        // vitest imports warrants a direct pin — otherwise the catalog omits
        // `vitest` and this would leave a dangling `catalog:` spec.
        isVitestAdjacent ||
        hasNuxtTestUtils)
    ) {
      pkg.devDependencies ??= {};
      pkg.devDependencies.vitest = getCatalogDependencySpec(
        undefined,
        VITEST_VERSION,
        supportCatalog,
        { preferredCatalogSpec: catalogDependencyResolver?.preferredCatalogSpec },
      );
    }
  }
  return extractedStagedConfig;
}

// Returns true if the spec uses a known protocol prefix (catalog:, workspace:,
// link:, file:, npm:, github:, git+/git:, http(s)://) and so represents a
// deliberate user choice that should not be silently rewritten.
export function isProtocolPinnedSpec(spec: string): boolean {
  return /^(catalog:|workspace:|link:|file:|npm:|github:|git[+:]|https?:\/\/)/.test(spec);
}
