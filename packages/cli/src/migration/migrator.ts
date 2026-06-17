import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { styleText } from 'node:util';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import spawn from 'cross-spawn';
import type { OxlintConfig } from 'oxlint';
import semver from 'semver';
import { Scalar, YAMLMap, YAMLSeq } from 'yaml';

import {
  hasConfigKey,
  mergeJsonConfig,
  mergeTsdownConfig,
  rewriteEslint,
  rewritePrettier,
  rewriteScripts,
  rewriteImportsInDirectory,
  wrapLazyPlugins,
  type DownloadPackageManagerResult,
} from '../../binding/index.js';
import {
  createDefaultVitePlusLintConfig,
  ensureVitePlusImportRuleDefaults,
} from '../oxlint-plugin-config.ts';
import { PackageManager, type WorkspaceInfo, type WorkspacePackage } from '../types/index.ts';
import { runCommandSilently } from '../utils/command.ts';
import {
  BASEURL_TSCONFIG_WARNING,
  VITE_PLUS_NAME,
  VITE_PLUS_OVERRIDE_PACKAGES,
  VITE_PLUS_VERSION,
  VITEST_AGE_GATE_EXEMPT_PACKAGES,
  VITEST_VERSION,
  isForceOverrideMode,
} from '../utils/constants.ts';
import { editJsonFile, isJsonFile, readJsonFile } from '../utils/json.ts';
import { detectPackageMetadata } from '../utils/package.ts';
import { displayRelative, rulesDir } from '../utils/path.ts';
import { cancelAndExit } from '../utils/prompts.ts';
import { getSpinner } from '../utils/spinner.ts';
import {
  findTsconfigFiles,
  hasBaseUrlInTsconfig,
  hasTypesToRewriteInTsconfig,
  removeDeprecatedTsconfigFalseOption,
  rewriteTypesInTsconfig,
} from '../utils/tsconfig.ts';
import type { NpmWorkspaces } from '../utils/workspace.ts';
import { editYamlFile, readYamlFile, scalarString, type YamlDocument } from '../utils/yaml.ts';
import {
  PRETTIER_CONFIG_FILES,
  PRETTIER_PACKAGE_JSON_CONFIG,
  detectConfigs,
  type ConfigFiles,
} from './detector.ts';
import { addManualStep, addMigrationWarning, type MigrationReport } from './report.ts';

// All known lint-staged config file names.
// JSON-parseable ones come first so rewriteLintStagedConfigFile can rewrite them.
const LINT_STAGED_JSON_CONFIG_FILES = ['.lintstagedrc.json', '.lintstagedrc'] as const;
const LINT_STAGED_OTHER_CONFIG_FILES = [
  '.lintstagedrc.yaml',
  '.lintstagedrc.yml',
  '.lintstagedrc.mjs',
  'lint-staged.config.mjs',
  '.lintstagedrc.cjs',
  'lint-staged.config.cjs',
  '.lintstagedrc.js',
  'lint-staged.config.js',
  '.lintstagedrc.ts',
  'lint-staged.config.ts',
  '.lintstagedrc.mts',
  'lint-staged.config.mts',
  '.lintstagedrc.cts',
  'lint-staged.config.cts',
] as const;
const LINT_STAGED_ALL_CONFIG_FILES = [
  ...LINT_STAGED_JSON_CONFIG_FILES,
  ...LINT_STAGED_OTHER_CONFIG_FILES,
] as const;

// packages that are replaced with vite-plus
const REMOVE_PACKAGES = [
  'oxlint',
  'oxlint-tsgolint',
  'oxfmt',
  'tsdown',
  '@vitest/browser',
  '@vitest/browser-preview',
] as const;

// The opt-in browser providers. Unlike `@vitest/browser`/preview these are NOT
// bundled by vite-plus or stripped from users (so they stay out of
// REMOVE_PACKAGES); each drags a heavy non-optional framework peer
// (`playwright` / `webdriverio`) that non-browser consumers must not be forced
// to install. The migration keeps a provider the user actually targets in their
// own deps, pinned to the bundled vitest version.
const WEBDRIVERIO_PROVIDER = '@vitest/browser-webdriverio';
const PLAYWRIGHT_PROVIDER = '@vitest/browser-playwright';

// All opt-in browser providers handled identically by the migration: kept in
// the user's deps (pinned to the bundled vitest), framework peer ensured, stale
// forcing pins dropped, while their catalog entries are PRESERVED.
const OPT_IN_BROWSER_PROVIDERS = [WEBDRIVERIO_PROVIDER, PLAYWRIGHT_PROVIDER] as const;

// Provider names whose stale pnpm overrides / resolutions are dropped during
// migration: everything vite-plus owns (REMOVE_PACKAGES) plus the user-owned
// opt-in providers. The provider DEP is preserved, but a leftover
// override/resolution pin to another version would WIN over the direct dep and
// misalign the provider against the bundled vitest — so the stale forcing pin is
// dropped while the dependency itself stays installed. NOTE: catalog deletion
// uses REMOVE_PACKAGES (not this set) on purpose — a catalog entry is only a
// version *definition*, and deleting it could dangle a surviving `catalog:`
// reference (e.g. in peerDependencies) and break install.
const PROVIDER_OVERRIDE_DROP_NAMES = [...REMOVE_PACKAGES, ...OPT_IN_BROWSER_PROVIDERS] as const;

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
function shouldDropProviderOverrideKey(key: string): boolean {
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
function dropRemovePackageOverrideKeys(
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

// When a browser provider package is removed, its runtime peer dependency
// must be preserved in devDependencies so browser tests continue to work.
const BROWSER_PROVIDER_PEER_DEPS: Record<string, string> = {
  '@vitest/browser-playwright': 'playwright',
  '@vitest/browser-webdriverio': 'webdriverio',
};

// Transitive packages with postinstall scripts that vite-plus's deps drag in
// via `@vitest/browser-webdriverio` → `webdriverio` → `@wdio/utils`. pnpm v10
// refuses to run these without explicit approval, so `vp migrate` records the
// allow/deny decision up front: deny by default (the user isn't using
// webdriverio), allow when the user actually depends on webdriverio.
const BROWSER_PROVIDER_POSTINSTALL_PACKAGES = ['edgedriver', 'geckodriver'] as const;

// Webdriverio is the runtime peer that drags `edgedriver` / `geckodriver` in.
const WEBDRIVERIO_PEER_DEP = 'webdriverio';

// Dependencies whose presence before migration signals the user will end up
// with webdriverio after migration. `@vitest/browser-webdriverio` is the opt-in
// provider vite-plus keeps in the user's deps (pinned to the bundled vitest)
// and `webdriverio` is its runtime peer (added via `BROWSER_PROVIDER_PEER_DEPS`);
// either one means the edgedriver/geckodriver postinstalls must be allowed.
const WEBDRIVERIO_ALLOW_SIGNAL_DEPS = [WEBDRIVERIO_PEER_DEP, WEBDRIVERIO_PROVIDER] as const;

// Browser-provider package names that, when present in the user's deps
// before migration, signal vitest browser mode even if no source file
// imports them. This covers config-only browser-mode setups (e.g.
// `test.browser.provider: 'playwright'` in `vite.config.ts`) where the
// provider package is declared in `devDependencies` but never `import`ed.
const VITEST_BROWSER_DEP_NAMES = [
  '@vitest/browser',
  '@vitest/browser-preview',
  '@vitest/browser-playwright',
  '@vitest/browser-webdriverio',
] as const;

const PUBLIC_PEER_DEPENDENCY_FALLBACKS: Record<string, string> = {
  vite: '*',
  vitest: '*',
};

// Plugins Oxlint resolves natively (no JS import). Source:
// `LintPluginOptionsSchema` in `node_modules/oxlint/dist/index.d.ts`.
// Anything else in the merged `lint.plugins[]` after migration is a
// reference left over from `@oxlint/migrate` that won't resolve at lint
// time.
const OXLINT_NATIVE_PLUGINS = new Set<string>([
  'eslint',
  'react',
  'unicorn',
  'typescript',
  'oxc',
  'import',
  'jsdoc',
  'jest',
  'vitest',
  'jsx-a11y',
  'nextjs',
  'react-perf',
  'promise',
  'node',
  'vue',
]);

// Legacy wrapper package names that may appear as the target of override
// aliases left over from earlier vite-plus migrations. `@voidzero-dev/vite-plus-test`
// was deleted; any catalog/override entry still pointing at it is stale.
const LEGACY_WRAPPER_PACKAGE_NAMES = ['@voidzero-dev/vite-plus-test'] as const;

// Fallback specs used when normalizing a stale wrapper alias. Real user
// ranges (e.g. `vitest: ^3.0.0`) are preserved — only the wrapper alias is
// rewritten. For `vitest`, we substitute the vitest version vite-plus
// bundles so any `catalog:` reference the user still has resolves cleanly.
const LEGACY_WRAPPER_FALLBACK_VERSIONS: Record<string, string> = {
  vitest: VITEST_VERSION,
};

function isLegacyWrapperSpec(value: unknown): boolean {
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
function pruneLegacyWrapperAliases(record: Record<string, unknown> | undefined): boolean {
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

type PackageJsonDependencyField =
  | 'devDependencies'
  | 'dependencies'
  | 'peerDependencies'
  | 'optionalDependencies';

type CatalogDependencyResolver = (
  catalogSpec: string,
  dependencyName: string,
) => string | undefined;

function warnMigration(message: string, report?: MigrationReport) {
  addMigrationWarning(report, message);
  if (!report) {
    prompts.log.warn(message);
  }
}

function infoMigration(message: string, report?: MigrationReport) {
  addManualStep(report, message);
  if (!report) {
    prompts.log.info(message);
  }
}

export function checkViteVersion(projectPath: string): boolean {
  return checkPackageVersion(projectPath, 'vite', '7.0.0');
}

export function checkVitestVersion(projectPath: string): boolean {
  return checkPackageVersion(projectPath, 'vitest', '4.0.0');
}

/**
 * Check the package version is supported by auto migration
 * @param projectPath - The path to the project
 * @param name - The name of the package
 * @param minVersion - The minimum version of the package
 * @returns true if the package version is supported by auto migration
 */
function checkPackageVersion(projectPath: string, name: string, minVersion: string): boolean {
  const metadata = detectPackageMetadata(projectPath, name);
  if (!metadata || metadata.name !== name) {
    return true;
  }
  if (semver.satisfies(metadata.version, `<${minVersion}`)) {
    const packageJsonFilePath = path.join(projectPath, 'package.json');
    prompts.log.error(
      `✘ ${name}@${metadata.version} in ${displayRelative(packageJsonFilePath)} is not supported by auto migration`,
    );
    prompts.log.info(`Please upgrade ${name} to version >=${minVersion} first`);
    return false;
  }
  return true;
}

export function detectEslintProject(
  projectPath: string,
  packages?: WorkspacePackage[],
): {
  hasDependency: boolean;
  configFile?: string;
  legacyConfigFile?: string;
} {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return { hasDependency: false };
  }
  const pkg = readJsonFile(packageJsonPath) as {
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
  };
  let hasDependency = !!(pkg.devDependencies?.eslint || pkg.dependencies?.eslint);
  const configs = detectConfigs(projectPath);
  let configFile = configs.eslintConfig;
  const legacyConfigFile = configs.eslintLegacyConfig;

  // If root doesn't have eslint dependency, check workspace packages
  if (!hasDependency && packages) {
    for (const wp of packages) {
      const pkgJsonPath = path.join(projectPath, wp.path, 'package.json');
      if (!fs.existsSync(pkgJsonPath)) {
        continue;
      }
      const wpPkg = readJsonFile(pkgJsonPath) as {
        devDependencies?: Record<string, string>;
        dependencies?: Record<string, string>;
      };
      if (wpPkg.devDependencies?.eslint || wpPkg.dependencies?.eslint) {
        hasDependency = true;
        break;
      }
    }
  }

  return { hasDependency, configFile, legacyConfigFile };
}

/**
 * Run a `vp dlx @oxlint/migrate` step with graceful error handling.
 * Returns true on success, false on failure (spawn error or non-zero exit).
 */
async function runOxlintMigrateStep(
  vpBin: string,
  cwd: string,
  migratePackage: string,
  args: string[],
  spinner: ReturnType<typeof getSpinner>,
  failMessage: string,
  manualHint: string,
): Promise<boolean> {
  try {
    const result = await runCommandSilently({
      command: vpBin,
      args: ['dlx', migratePackage, ...args],
      cwd,
      envs: process.env,
    });
    if (result.exitCode !== 0) {
      spinner.stop(failMessage);
      const stderr = result.stderr.toString().trim();
      if (stderr) {
        prompts.log.warn(`⚠ ${stderr}`);
      }
      prompts.log.info(manualHint);
      return false;
    }
    return true;
  } catch {
    spinner.stop(failMessage);
    prompts.log.info(manualHint);
    return false;
  }
}

export async function migrateEslintToOxlint(
  projectPath: string,
  interactive: boolean,
  eslintConfigFile?: string,
  packages?: WorkspacePackage[],
  options?: { silent?: boolean; report?: MigrationReport },
): Promise<boolean> {
  const vpBin = process.env.VP_CLI_BIN ?? 'vp';
  const spinner = options?.silent
    ? {
        start: () => {},
        stop: () => {},
        pause: () => {},
        resume: () => {},
        cancel: () => {},
        error: () => {},
        clear: () => {},
        message: () => {},
        isCancelled: false,
      }
    : getSpinner(interactive);

  // Steps 1-2: Only run @oxlint/migrate if there's an eslint config at root
  if (eslintConfigFile) {
    // Pin @oxlint/migrate to the bundled oxlint version.
    // @ts-expect-error — resolved at runtime from dist/ → dist/versions.js
    const { versions } = await import('../versions.js');
    const migratePackage = `@oxlint/migrate@${versions.oxlint}`;
    const migrateArgs = [
      '--merge',
      ...(!hasBaseUrlInTsconfig(projectPath) ? ['--type-aware'] : []),
      '--with-nursery',
      '--details',
    ];

    // Step 1: Generate .oxlintrc.json from ESLint config
    spinner.start('Migrating ESLint config to Oxlint...');
    const migrateOk = await runOxlintMigrateStep(
      vpBin,
      projectPath,
      migratePackage,
      migrateArgs,
      spinner,
      'ESLint migration failed',
      `You can run \`vp dlx ${migratePackage} ${migrateArgs.join(' ')}\` manually later`,
    );
    if (!migrateOk) {
      return false;
    }
    spinner.stop('ESLint config migrated to .oxlintrc.json');

    // Step 2: Replace eslint-disable comments with oxlint-disable
    spinner.start('Replacing ESLint comments with Oxlint equivalents...');
    const replaceOk = await runOxlintMigrateStep(
      vpBin,
      projectPath,
      migratePackage,
      ['--replace-eslint-comments'],
      spinner,
      'ESLint comment replacement failed',
      `You can run \`vp dlx ${migratePackage} --replace-eslint-comments\` manually later`,
    );
    if (replaceOk) {
      spinner.stop('ESLint comments replaced');
    }
    // Continue with cleanup regardless — .oxlintrc.json was generated successfully
  }

  if (options?.report) {
    options.report.eslintMigrated = true;
  }

  // Read the generated `.oxlintrc.json` to find any packages it references
  // in `lint.jsPlugins`. Those packages need to stay in `package.json` so
  // Oxlint can actually `import()` them at lint time — without this carve-out,
  // the next step would strip them via `isEslintEcosystemDep` and we'd
  // immediately invalidate the config we just generated. Local-path
  // specifiers (`./X`, `../X`, `/X`) are skipped — they're paths, not
  // package names, and have no `package.json` entry to preserve.
  const preserveJsPlugins = collectJsPluginPackageNames(projectPath);

  // Step 3-5: Cleanup runs uniformly across the root and every workspace
  // package — delete eslint config files, scrub ESLint-ecosystem deps from
  // package.json, and rewrite eslint references in any local lint-staged
  // config. A monorepo running `vp migrate` is treated as adopted as a
  // whole; there's no per-package opt-out today. If a workspace package
  // publishes a shared ESLint preset that you want to keep intact, exclude
  // it from your `pnpm-workspace.yaml` / `workspaces` before running
  // `vp migrate`, then add it back afterwards.
  const cleanupTargets = [
    projectPath,
    ...(packages ?? []).map((p) => path.join(projectPath, p.path)),
  ];
  for (const target of cleanupTargets) {
    if (!fs.existsSync(path.join(target, 'package.json'))) {
      continue;
    }
    deleteEslintConfigFiles(target, options?.report, options?.silent);
    rewriteEslintPackageJson(path.join(target, 'package.json'), preserveJsPlugins);
    rewriteEslintLintStagedConfigFiles(target, options?.report);
  }

  return true;
}

/**
 * Read `<projectPath>/.oxlintrc.json` (if any) and collect the package
 * names referenced via `lint.jsPlugins[]` string entries. Object-form
 * entries (`{ name, specifier }`) and local-path specifiers (`./X`,
 * `../X`, `/X`) are excluded — neither maps to a `package.json` entry
 * we'd accidentally strip.
 */
function collectJsPluginPackageNames(projectPath: string): Set<string> {
  const out = new Set<string>();
  const oxlintConfigPath = path.join(projectPath, '.oxlintrc.json');
  if (!fs.existsSync(oxlintConfigPath)) {
    return out;
  }
  let config: OxlintConfig;
  try {
    config = readJsonFile(oxlintConfigPath, true) as OxlintConfig;
  } catch {
    return out;
  }
  const collectFrom = (jsPlugins: OxlintConfig['jsPlugins']): void => {
    for (const entry of jsPlugins ?? []) {
      if (typeof entry !== 'string') {
        continue;
      }
      if (entry.startsWith('./') || entry.startsWith('../') || entry.startsWith('/')) {
        continue;
      }
      out.add(entry);
    }
  };
  collectFrom(config.jsPlugins);
  if (Array.isArray(config.overrides)) {
    for (const override of config.overrides) {
      collectFrom(override.jsPlugins);
    }
  }
  return out;
}

function deleteEslintConfigFiles(basePath: string, report?: MigrationReport, silent = false): void {
  const configs = detectConfigs(basePath);
  for (const file of [configs.eslintConfig, configs.eslintLegacyConfig]) {
    if (file) {
      const configPath = path.join(basePath, file);
      if (fs.existsSync(configPath)) {
        fs.unlinkSync(configPath);
        if (report) {
          report.removedConfigCount++;
        }
        if (!silent) {
          prompts.log.success(`✔ Removed ${displayRelative(configPath)}`);
        }
      }
    }
  }
}

// Bare names of packages whose sole purpose is to support ESLint. Removed
// at root cleanup. Reusable AST libraries published under
// `@typescript-eslint/*` (`utils`, `typescript-estree`, `scope-manager`,
// `types`) are deliberately absent so codemods and doc generators that
// import them directly keep working after migration.
const ESLINT_ECOSYSTEM_NAMES = new Set<string>([
  'eslint',
  'typescript-eslint',
  'eslintrc',
  'eslint-utils',
  'eslint-visitor-keys',
  'eslint-scope',
  'eslint-define-config',
  'eslint-doc-generator',
  // ESLint-only typescript-eslint entry points:
  '@typescript-eslint/eslint-plugin',
  '@typescript-eslint/parser',
  '@typescript-eslint/rule-tester',
  // Note: framework-ESLint integration modules (e.g. `@nuxt/eslint`)
  // are NOT listed here. They short-circuit the entire ESLint
  // migration via `INCOMPATIBLE_ESLINT_INTEGRATIONS`, so this list is
  // never consulted for them. Keeping them out avoids duplicating the
  // "what to do about Nuxt" decision in two places.
]);

// Flat name prefixes that mark an ESLint-only package.
const ESLINT_ECOSYSTEM_PREFIXES = ['eslint-plugin-', 'eslint-config-', 'eslint-formatter-'];

// Scopes whose every package is part of the ESLint ecosystem.
//   @eslint/*           — official ESLint scope (e.g. @eslint/js, @eslint/eslintrc)
//   @eslint-community/* — community-maintained ESLint dependencies
//   @angular-eslint/*   — Angular's ESLint integration family
const ESLINT_ECOSYSTEM_SCOPES = ['@eslint/', '@eslint-community/', '@angular-eslint/'];

/**
 * Decide whether a dependency entry should be removed alongside `eslint`
 * itself. The set is intentionally broad: anything whose only purpose is
 * to extend, configure, format, or wire ESLint becomes dead weight after
 * migration. `@types/<X>` packages are checked symmetrically with `<X>`
 * so type-only counterparts of removed runtime packages also go.
 */
function isEslintEcosystemDep(name: string): boolean {
  const stripped = name.startsWith('@types/') ? name.slice('@types/'.length) : name;
  if (ESLINT_ECOSYSTEM_NAMES.has(stripped)) {
    return true;
  }
  if (ESLINT_ECOSYSTEM_PREFIXES.some((p) => stripped.startsWith(p))) {
    return true;
  }
  if (ESLINT_ECOSYSTEM_SCOPES.some((s) => stripped.startsWith(s))) {
    return true;
  }
  // Scoped plugins/configs/formatters, e.g.:
  //   @vue/eslint-config-typescript
  //   @stylistic/eslint-plugin-ts
  //   @vitest/eslint-plugin
  if (/^@[^/]+\/eslint-(plugin|config|formatter)(-.+)?$/.test(stripped)) {
    return true;
  }
  return false;
}

/**
 * Rewrite a project's `package.json` after ESLint has been migrated to
 * Oxlint: drop every ESLint-ecosystem dependency (see
 * `isEslintEcosystemDep`), strip empty containers, and rewrite eslint
 * tokens in scripts / lint-staged. Applied uniformly to the root and to
 * every workspace package — the migration treats the whole workspace as
 * in scope for adoption, so a half-cleanup at the workspace level would
 * be inconsistent with the rest of the flow (which already replaces
 * vite-related overrides and adds vite-plus across all packages).
 *
 * `preserveJsPlugins` names packages that `@oxlint/migrate` referenced
 * via `lint.jsPlugins` and that Oxlint will need to `import()` at lint
 * time. They override `isEslintEcosystemDep` so the generated config
 * isn't immediately invalidated by the cleanup step.
 */
export function rewriteEslintPackageJson(
  packageJsonPath: string,
  preserveJsPlugins: ReadonlySet<string> = new Set(),
): void {
  editJsonFile<{
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
    peerDependencies?: Record<string, string>;
    optionalDependencies?: Record<string, string>;
    scripts?: Record<string, string>;
    'lint-staged'?: Record<string, string | string[]>;
  }>(packageJsonPath, (pkg) => {
    let changed = false;
    for (const field of [
      'devDependencies',
      'dependencies',
      'peerDependencies',
      'optionalDependencies',
    ] as const) {
      const deps = pkg[field];
      if (!deps) {
        continue;
      }
      let removedAny = false;
      for (const name of Object.keys(deps)) {
        if (preserveJsPlugins.has(name)) {
          continue;
        }
        if (isEslintEcosystemDep(name)) {
          delete deps[name];
          changed = true;
          removedAny = true;
        }
      }
      // Drop the field entirely if our cleanup emptied it — avoid
      // leaving `"devDependencies": {}` noise in the output.
      if (removedAny && Object.keys(deps).length === 0) {
        delete pkg[field];
      }
    }
    if (pkg.scripts) {
      const updated = rewriteEslint(JSON.stringify(pkg.scripts));
      if (updated) {
        pkg.scripts = JSON.parse(updated);
        changed = true;
      }
    }
    if (pkg['lint-staged']) {
      const updated = rewriteEslint(JSON.stringify(pkg['lint-staged']));
      if (updated) {
        pkg['lint-staged'] = JSON.parse(updated);
        changed = true;
      }
    }
    return changed ? pkg : undefined;
  });
}

/**
 * Rewrite tool references in lint-staged config files (JSON ones are rewritten,
 * non-JSON ones get a warning).
 */
function rewriteToolLintStagedConfigFiles(
  projectPath: string,
  rewriteFn: (json: string) => string | null,
  toolName: string,
  report?: MigrationReport,
): void {
  for (const filename of LINT_STAGED_JSON_CONFIG_FILES) {
    const configPath = path.join(projectPath, filename);
    if (!fs.existsSync(configPath)) {
      continue;
    }
    if (filename === '.lintstagedrc' && !isJsonFile(configPath)) {
      warnMigration(
        `${displayRelative(configPath)} is not JSON — please update ${toolName} references manually`,
        report,
      );
      continue;
    }
    editJsonFile<Record<string, string | string[]>>(configPath, (config) => {
      const updated = rewriteFn(JSON.stringify(config));
      if (updated) {
        return JSON.parse(updated);
      }
      return undefined;
    });
  }
  for (const filename of LINT_STAGED_OTHER_CONFIG_FILES) {
    const configPath = path.join(projectPath, filename);
    if (!fs.existsSync(configPath)) {
      continue;
    }
    warnMigration(
      `${displayRelative(configPath)} — please update ${toolName} references manually`,
      report,
    );
  }
}

function rewriteEslintLintStagedConfigFiles(projectPath: string, report?: MigrationReport): void {
  rewriteToolLintStagedConfigFiles(projectPath, rewriteEslint, 'eslint', report);
}

export function detectPrettierProject(
  projectPath: string,
  packages?: WorkspacePackage[],
): {
  hasDependency: boolean;
  configFile?: string;
} {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return { hasDependency: false };
  }
  const pkg = readJsonFile(packageJsonPath) as {
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
  };
  let hasDependency = !!(pkg.devDependencies?.prettier || pkg.dependencies?.prettier);
  const configs = detectConfigs(projectPath);
  const configFile = configs.prettierConfig;

  // If root doesn't have prettier dependency, check workspace packages
  if (!hasDependency && packages) {
    for (const wp of packages) {
      const pkgJsonPath = path.join(projectPath, wp.path, 'package.json');
      if (!fs.existsSync(pkgJsonPath)) {
        continue;
      }
      const wpPkg = readJsonFile(pkgJsonPath) as {
        devDependencies?: Record<string, string>;
        dependencies?: Record<string, string>;
      };
      if (wpPkg.devDependencies?.prettier || wpPkg.dependencies?.prettier) {
        hasDependency = true;
        break;
      }
    }
  }

  return { hasDependency, configFile };
}

/**
 * Run `vp fmt --migrate=prettier` step with graceful error handling.
 * Returns true on success, false on failure.
 */
async function runPrettierMigrateStep(
  vpBin: string,
  cwd: string,
  spinner: ReturnType<typeof getSpinner>,
  failMessage: string,
  manualHint: string,
): Promise<boolean> {
  try {
    const result = await runCommandSilently({
      command: vpBin,
      args: ['fmt', '--migrate=prettier'],
      cwd,
      envs: process.env,
    });
    if (result.exitCode !== 0) {
      spinner.stop(failMessage);
      const stderr = result.stderr.toString().trim();
      if (stderr) {
        prompts.log.warn(`⚠ ${stderr}`);
      }
      prompts.log.info(manualHint);
      return false;
    }
    return true;
  } catch {
    spinner.stop(failMessage);
    prompts.log.info(manualHint);
    return false;
  }
}

export async function migratePrettierToOxfmt(
  projectPath: string,
  interactive: boolean,
  prettierConfigFile?: string,
  packages?: WorkspacePackage[],
  options?: { silent?: boolean; report?: MigrationReport },
): Promise<boolean> {
  const vpBin = process.env.VP_CLI_BIN ?? 'vp';
  const spinner = options?.silent
    ? {
        start: () => {},
        stop: () => {},
        pause: () => {},
        resume: () => {},
        cancel: () => {},
        error: () => {},
        clear: () => {},
        message: () => {},
        isCancelled: false,
      }
    : getSpinner(interactive);

  // Step 1: Generate .oxfmtrc.json from Prettier config
  if (prettierConfigFile) {
    let tempPrettierConfig: string | undefined;

    // If config is in package.json, extract it to a temporary .prettierrc.json
    // so that `vp fmt --migrate=prettier` can read it
    if (prettierConfigFile === PRETTIER_PACKAGE_JSON_CONFIG) {
      const packageJsonPath = path.join(projectPath, 'package.json');
      const pkg = readJsonFile(packageJsonPath) as { prettier?: unknown };
      if (pkg.prettier) {
        tempPrettierConfig = path.join(projectPath, '.prettierrc.json');
        fs.writeFileSync(tempPrettierConfig, JSON.stringify(pkg.prettier, null, 2));
      } else {
        // Config disappeared between detection and migration — nothing to migrate
        return true;
      }
    }

    try {
      spinner.start('Migrating Prettier config to Oxfmt...');
      const migrateOk = await runPrettierMigrateStep(
        vpBin,
        projectPath,
        spinner,
        'Prettier migration failed',
        'You can run `vp fmt --migrate=prettier` manually later',
      );
      if (!migrateOk) {
        return false;
      }
      spinner.stop('Prettier config migrated to .oxfmtrc.json');
    } finally {
      if (tempPrettierConfig) {
        try {
          fs.unlinkSync(tempPrettierConfig);
        } catch {}
      }
    }
  }

  if (options?.report) {
    options.report.prettierMigrated = true;
  }

  // Step 2: Delete all prettier config files at root
  deletePrettierConfigFiles(projectPath, options?.report, options?.silent);

  // Step 3: Remove prettier dependency and rewrite prettier scripts (root)
  rewritePrettierPackageJson(path.join(projectPath, 'package.json'));

  // Step 3b: Rewrite prettier scripts in workspace packages
  if (packages) {
    for (const pkg of packages) {
      rewritePrettierPackageJson(path.join(projectPath, pkg.path, 'package.json'));
    }
  }

  // Step 4: Rewrite prettier references in lint-staged config files
  rewritePrettierLintStagedConfigFiles(projectPath, options?.report);

  // Step 5: Warn about .prettierignore if it exists
  const prettierIgnorePath = path.join(projectPath, '.prettierignore');
  if (fs.existsSync(prettierIgnorePath)) {
    warnMigration(
      `${displayRelative(prettierIgnorePath)} found — Oxfmt supports .prettierignore, but using the \`ignorePatterns\` option is recommended.`,
      options?.report,
    );
  }

  return true;
}

function deletePrettierConfigFiles(
  basePath: string,
  report?: MigrationReport,
  silent = false,
): void {
  // Delete detected prettier config file (like deleteEslintConfigFiles uses detectConfigs)
  const configs = detectConfigs(basePath);
  if (configs.prettierConfig && configs.prettierConfig !== PRETTIER_PACKAGE_JSON_CONFIG) {
    const configPath = path.join(basePath, configs.prettierConfig);
    if (fs.existsSync(configPath)) {
      fs.unlinkSync(configPath);
      if (report) {
        report.removedConfigCount++;
      }
      if (!silent) {
        prompts.log.success(`✔ Removed ${displayRelative(configPath)}`);
      }
    }
  }
  // Also clean up any stale prettier config files that detectConfigs didn't pick
  // (prettier only uses one config, but users may have leftover files)
  for (const file of PRETTIER_CONFIG_FILES) {
    if (file === configs.prettierConfig) {
      continue; // already handled above
    }
    const configPath = path.join(basePath, file);
    if (fs.existsSync(configPath)) {
      fs.unlinkSync(configPath);
      if (report) {
        report.removedConfigCount++;
      }
      if (!silent) {
        prompts.log.success(`✔ Removed ${displayRelative(configPath)}`);
      }
    }
  }
  // Remove "prettier" key from package.json if present
  editJsonFile<{ prettier?: unknown }>(path.join(basePath, 'package.json'), (pkg) => {
    if (pkg.prettier) {
      delete pkg.prettier;
      return pkg;
    }
    return undefined;
  });
}

function rewritePrettierPackageJson(packageJsonPath: string): void {
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }
  editJsonFile<{
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
    scripts?: Record<string, string>;
    'lint-staged'?: Record<string, string | string[]>;
  }>(packageJsonPath, (pkg) => {
    let changed = false;
    // Remove prettier and prettier-plugin-* dependencies
    if (pkg.devDependencies) {
      for (const dep of Object.keys(pkg.devDependencies)) {
        if (dep === 'prettier' || dep.startsWith('prettier-plugin-')) {
          delete pkg.devDependencies[dep];
          changed = true;
        }
      }
    }
    if (pkg.dependencies) {
      for (const dep of Object.keys(pkg.dependencies)) {
        if (dep === 'prettier' || dep.startsWith('prettier-plugin-')) {
          delete pkg.dependencies[dep];
          changed = true;
        }
      }
    }
    if (pkg.scripts) {
      const updated = rewritePrettier(JSON.stringify(pkg.scripts));
      if (updated) {
        pkg.scripts = JSON.parse(updated);
        changed = true;
      }
    }
    if (pkg['lint-staged']) {
      const updated = rewritePrettier(JSON.stringify(pkg['lint-staged']));
      if (updated) {
        pkg['lint-staged'] = JSON.parse(updated);
        changed = true;
      }
    }
    return changed ? pkg : undefined;
  });
}

function rewritePrettierLintStagedConfigFiles(projectPath: string, report?: MigrationReport): void {
  rewriteToolLintStagedConfigFiles(projectPath, rewritePrettier, 'prettier', report);
}

function cleanupDeprecatedTsconfigOptions(
  projectPath: string,
  silent = false,
  report?: MigrationReport,
): void {
  const deprecatedOptions = ['esModuleInterop', 'allowSyntheticDefaultImports'];
  const files = findTsconfigFiles(projectPath);
  for (const filePath of files) {
    for (const name of deprecatedOptions) {
      if (removeDeprecatedTsconfigFalseOption(filePath, name)) {
        if (report) {
          report.removedConfigCount++;
        }
        if (!silent) {
          prompts.log.success(`✔ Removed ${name}: false from ${displayRelative(filePath)}`);
        }
        warnMigration(
          `Removed \`"${name}": false\` from ${displayRelative(filePath)} — this option has been deprecated. See https://github.com/oxc-project/tsgolint/issues/351, https://github.com/microsoft/TypeScript/issues/62529`,
          report,
        );
      }
    }
  }
}

function rewriteTsconfigTypes(
  projectPath: string,
  silent = false,
  report?: MigrationReport,
): boolean {
  let changed = false;
  const files = findTsconfigFiles(projectPath);
  for (const filePath of files) {
    if (rewriteTypesInTsconfig(filePath)) {
      changed = true;
      if (report) {
        report.removedConfigCount++;
      }
      if (!silent) {
        prompts.log.success(`✔ Rewrote types in ${displayRelative(filePath)}`);
      }
    }
  }
  return changed;
}

function hasTsconfigTypesToRewrite(projectPath: string): boolean {
  return findTsconfigFiles(projectPath).some((filePath) => hasTypesToRewriteInTsconfig(filePath));
}

// .svelte files are handled by @sveltejs/vite-plugin-svelte (transpilation)
// and svelte-check / Svelte Language Server (type checking).
// Module resolution for `.svelte` imports is typically set up by the
// project template (e.g. src/vite-env.d.ts in Vite svelte-ts, or
// auto-generated tsconfig in SvelteKit) rather than this file.
// https://svelte.dev/docs/svelte/typescript
export type Framework = 'vue' | 'astro';

const FRAMEWORK_SHIMS: Record<Framework, string> = {
  // https://vuejs.org/guide/typescript/overview#volar-takeover-mode
  vue: [
    "declare module '*.vue' {",
    "  import type { DefineComponent } from 'vue';",
    '  const component: DefineComponent<{}, {}, unknown>;',
    '  export default component;',
    '}',
  ].join('\n'),
  // astro/client is the pre-v4.14 form; v4.14+ prefers `/// <reference path="../.astro/types.d.ts" />`
  // but .astro/types.d.ts is generated at build time and may not exist yet after migration.
  // astro/client remains valid and is still used in official Astro integrations.
  // https://docs.astro.build/en/guides/typescript/#extending-global-types
  astro: '/// <reference types="astro/client" />',
};

export function detectFramework(projectPath: string): Framework[] {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return [];
  }
  const pkg = readJsonFile(packageJsonPath) as {
    dependencies?: Record<string, string>;
    devDependencies?: Record<string, string>;
  };
  const allDeps = { ...pkg.dependencies, ...pkg.devDependencies };
  return (['vue', 'astro'] as const).filter((framework) => !!allDeps[framework]);
}

function getEnvDtsPath(projectPath: string): string {
  const srcEnvDts = path.join(projectPath, 'src', 'env.d.ts');
  const rootEnvDts = path.join(projectPath, 'env.d.ts');
  for (const candidate of [srcEnvDts, rootEnvDts]) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }
  return fs.existsSync(path.join(projectPath, 'src')) ? srcEnvDts : rootEnvDts;
}

export function hasFrameworkShim(projectPath: string, framework: Framework): boolean {
  const dirsToScan = [projectPath, path.join(projectPath, 'src')];
  for (const dir of dirsToScan) {
    if (!fs.existsSync(dir)) {
      continue;
    }
    let entries: string[];
    try {
      entries = fs.readdirSync(dir);
    } catch {
      continue;
    }
    for (const entry of entries) {
      if (!entry.endsWith('.d.ts')) {
        continue;
      }
      const content = fs.readFileSync(path.join(dir, entry), 'utf-8');
      if (framework === 'astro') {
        if (content.includes('astro/client')) {
          return true;
        }
      } else if (content.includes(`'*.${framework}'`) || content.includes(`"*.${framework}"`)) {
        return true;
      }
    }
  }
  return false;
}

export function addFrameworkShim(
  projectPath: string,
  framework: Framework,
  report?: MigrationReport,
): void {
  const envDtsPath = getEnvDtsPath(projectPath);
  const shim = FRAMEWORK_SHIMS[framework];
  if (fs.existsSync(envDtsPath)) {
    const existing = fs.readFileSync(envDtsPath, 'utf-8');
    fs.writeFileSync(envDtsPath, `${existing.trimEnd()}\n\n${shim}\n`, 'utf-8');
  } else {
    fs.mkdirSync(path.dirname(envDtsPath), { recursive: true });
    fs.writeFileSync(envDtsPath, `${shim}\n`, 'utf-8');
  }
  if (report) {
    report.frameworkShimAdded = true;
  }
}

/**
 * Rewrite standalone project to add vite-plus dependencies
 * @param projectPath - The path to the project
 */
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
  const pnpmMajorVersion = pnpmMajor(workspaceInfo.downloadPackageManager.version);
  let extractedStagedConfig: Record<string, string | string[]> | null = null;
  let remainingPnpmOverrides: Record<string, string> | undefined;
  let shouldRewritePnpmWorkspaceYaml = false;
  let shouldAddPnpmWorkspaceVitePlusOverride = false;
  let shouldAllowBrowserProviderBuilds = false;
  // Determined inside editJsonFile callback to avoid a redundant file read
  let usePnpmWorkspaceYaml = false;
  editJsonFile<{
    overrides?: Record<string, string>;
    resolutions?: Record<string, string>;
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
    peerDependencies?: Record<string, string>;
    optionalDependencies?: Record<string, string>;
    scripts?: Record<string, string>;
    pnpm?: {
      overrides?: Record<string, string>;
      peerDependencyRules?: {
        allowAny?: string[];
        allowedVersions?: Record<string, string>;
      };
      allowBuilds?: Record<string, boolean>;
      onlyBuiltDependencies?: string[];
    };
  }>(packageJsonPath, (pkg) => {
    shouldAllowBrowserProviderBuilds =
      hasOwnWebdriverioDependency(pkg) || usesWebdriverioProvider(projectPath);
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
    if (packageManager === PackageManager.yarn) {
      pkg.resolutions = {
        ...pkg.resolutions,
        ...VITE_PLUS_OVERRIDE_PACKAGES,
      };
    } else if (packageManager === PackageManager.npm || packageManager === PackageManager.bun) {
      pkg.overrides = {
        ...pkg.overrides,
        ...VITE_PLUS_OVERRIDE_PACKAGES,
      };
      if (packageManager === PackageManager.bun) {
        // Bun walks transitive peer-deps before resolving overrides; vitest
        // 4.1.9 declares peer `vite ^6 || ^7 || ^8` and aborts with
        // "vite@... failed to resolve" if `vite` isn't a direct dep somewhere
        // in the tree, even when the override would redirect it. Mirror the
        // override as a devDep so bun's resolver sees `vite` immediately;
        // the override above still points it at vite-plus-core.
        // See https://github.com/oven-sh/bun/issues/8406.
        pkg.devDependencies = {
          ...pkg.devDependencies,
          vite: VITE_PLUS_OVERRIDE_PACKAGES.vite,
        };
      }
    } else if (packageManager === PackageManager.pnpm) {
      // If package.json already has a "pnpm" field, keep using it;
      // otherwise use pnpm-workspace.yaml.
      usePnpmWorkspaceYaml = !pkg.pnpm;
      if (usePnpmWorkspaceYaml) {
        shouldRewritePnpmWorkspaceYaml = true;
        shouldAddPnpmWorkspaceVitePlusOverride = isForceOverrideMode();
      }
      const overrideKeys = Object.keys(VITE_PLUS_OVERRIDE_PACKAGES);
      if (!usePnpmWorkspaceYaml) {
        // Strip selector-shaped overrides (e.g. `parent>@vitest/browser-playwright`)
        // whose target is a removed package, before re-merging the user's
        // overrides into the new pnpm config.
        dropRemovePackageOverrideKeys(pkg.pnpm?.overrides);
        // Project already has pnpm config in package.json -- keep using it.
        pkg.pnpm = {
          ...pkg.pnpm,
          overrides: {
            ...pkg.pnpm?.overrides,
            ...VITE_PLUS_OVERRIDE_PACKAGES,
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
        remainingPnpmOverrides = cleanupPnpmOverridesForWorkspaceYaml(pkg, overrideKeys);
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
      usePnpmWorkspaceYaml,
      skipStagedMigration,
      catalogDependencyResolver,
      usesVitestBrowserMode(projectPath),
      collectProviderSourceModes(projectPath),
    );

    // ensure vite-plus is in devDependencies
    if (!pkg.devDependencies?.[VITE_PLUS_NAME] || isForceOverrideMode()) {
      const version =
        usePnpmWorkspaceYaml && !VITE_PLUS_VERSION.startsWith('file:')
          ? 'catalog:'
          : VITE_PLUS_VERSION;
      pkg.devDependencies = {
        ...pkg.devDependencies,
        [VITE_PLUS_NAME]: version,
      };
    }
    return pkg;
  });

  if (shouldRewritePnpmWorkspaceYaml) {
    rewritePnpmWorkspaceYaml(projectPath, pnpmMajorVersion, shouldAllowBrowserProviderBuilds);
  }

  // Move remaining non-Vite pnpm.overrides to pnpm-workspace.yaml
  if (remainingPnpmOverrides) {
    migratePnpmOverridesToWorkspaceYaml(projectPath, remainingPnpmOverrides);
  }

  if (shouldAddPnpmWorkspaceVitePlusOverride) {
    migratePnpmOverridesToWorkspaceYaml(projectPath, {
      [VITE_PLUS_NAME]: VITE_PLUS_VERSION,
    });
  }

  if (packageManager === PackageManager.yarn) {
    rewriteYarnrcYml(projectPath);
  } else if (packageManager === PackageManager.bun) {
    ensureBunfigPeerSuppression(projectPath);
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
  rewriteAllImports(projectPath, silent, report);
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
  const workspaceShouldAllowBrowserBuilds = workspaceUsesWebdriverio(
    workspaceInfo.rootDir,
    workspaceInfo.packages,
  );
  // rewrite root workspace
  if (workspaceInfo.packageManager === PackageManager.pnpm) {
    rewritePnpmWorkspaceYaml(
      workspaceInfo.rootDir,
      pnpmMajorVersion,
      workspaceShouldAllowBrowserBuilds,
    );
  } else if (workspaceInfo.packageManager === PackageManager.yarn) {
    rewriteYarnrcYml(workspaceInfo.rootDir);
  } else if (workspaceInfo.packageManager === PackageManager.bun) {
    rewriteBunCatalog(workspaceInfo.rootDir);
  }
  rewriteRootWorkspacePackageJson(
    workspaceInfo.rootDir,
    workspaceInfo.packageManager,
    skipStagedMigration,
    catalogDependencyResolver,
    workspaceInfo.packages,
    pnpmMajorVersion,
    workspaceShouldAllowBrowserBuilds,
  );
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
  rewriteAllImports(workspaceInfo.rootDir, silent, report);
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
    // rewrite scripts in package.json
    extractedStagedConfig = rewritePackageJson(
      pkg,
      packageManager,
      true,
      skipStagedMigration,
      catalogDependencyResolver,
      usesVitestBrowserMode(projectPath),
      collectProviderSourceModes(projectPath),
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
      pkg.devDependencies?.[VITE_PLUS_NAME]
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

/**
 * Rewrite pnpm-workspace.yaml to add vite-plus dependencies
 * @param projectPath - The path to the project
 */
function rewritePnpmWorkspaceYaml(
  projectPath: string,
  pnpmMajorVersion: number | undefined,
  shouldAllowBrowserBuilds: boolean,
): void {
  const pnpmWorkspaceYamlPath = path.join(projectPath, 'pnpm-workspace.yaml');
  if (!fs.existsSync(pnpmWorkspaceYamlPath)) {
    fs.writeFileSync(pnpmWorkspaceYamlPath, '');
  }

  editYamlFile(pnpmWorkspaceYamlPath, (doc) => {
    // catalog
    rewriteCatalog(doc);
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
    for (const key of Object.keys(VITE_PLUS_OVERRIDE_PACKAGES)) {
      const currentVersion = getYamlMapScalarStringValue(overrides, key);
      const version = getCatalogDependencySpec(
        currentVersion,
        VITE_PLUS_OVERRIDE_PACKAGES[key],
        true,
      );
      doc.setIn(['overrides', scalarString(key)], scalarString(version));
    }
    // remove dependency selector from vite, e.g. "vite-plugin-svgr>vite": "npm:vite@7.0.12"
    const updatedOverrides = doc.getIn(['overrides']) as YAMLMap<Scalar<string>, Scalar<string>>;
    for (const item of updatedOverrides.items) {
      if (item.key.value.includes('>')) {
        const splits = item.key.value.split('>');
        if (splits[splits.length - 1].trim() === 'vite') {
          updatedOverrides.delete(item.key);
        }
      }
    }

    // peerDependencyRules.allowAny
    let allowAny = doc.getIn(['peerDependencyRules', 'allowAny']) as YAMLSeq<Scalar<string>>;
    if (!allowAny) {
      allowAny = new YAMLSeq<Scalar<string>>();
    }
    const existing = new Set(allowAny.items.map((n) => n.value));
    for (const key of Object.keys(VITE_PLUS_OVERRIDE_PACKAGES)) {
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
    for (const key of Object.keys(VITE_PLUS_OVERRIDE_PACKAGES)) {
      // - vite: '*'
      allowedVersions.set(scalarString(key), scalarString('*'));
    }
    doc.setIn(['peerDependencyRules', 'allowedVersions'], allowedVersions);

    // minimumReleaseAgeExclude
    if (doc.has('minimumReleaseAge')) {
      // Exempt the Vite+-managed packages from the age gate: vite-plus,
      // @voidzero-dev/*, the ox* family, and the vitest family. Vite+ pins
      // `vitest` to an exact (sometimes freshly published) version and the
      // in-tree @vitest/* siblings install transitively at that version, so the
      // age gate would otherwise quarantine them and break `vp install`.
      const excludes = [
        'vite-plus',
        '@voidzero-dev/*',
        'oxlint',
        '@oxlint/*',
        'oxlint-tsgolint',
        '@oxlint-tsgolint/*',
        'oxfmt',
        '@oxfmt/*',
        ...VITEST_AGE_GATE_EXEMPT_PACKAGES,
      ];
      let minimumReleaseAgeExclude = doc.getIn(['minimumReleaseAgeExclude']) as YAMLSeq<
        Scalar<string>
      >;
      if (!minimumReleaseAgeExclude) {
        minimumReleaseAgeExclude = new YAMLSeq();
      }
      const existing = new Set(minimumReleaseAgeExclude.items.map((n) => n.value));
      for (const exclude of excludes) {
        if (!existing.has(exclude)) {
          minimumReleaseAgeExclude.add(scalarString(exclude));
        }
      }
      doc.setIn(['minimumReleaseAgeExclude'], minimumReleaseAgeExclude);
    }
  });
}

/**
 * Clean up pnpm.overrides and peerDependencyRules from package.json when migrating
 * to pnpm-workspace.yaml. Returns any remaining non-Vite overrides that need to be
 * moved to pnpm-workspace.yaml.
 */
function cleanupPnpmOverridesForWorkspaceYaml(
  pkg: {
    pnpm?: {
      overrides?: Record<string, string>;
      peerDependencyRules?: { allowAny?: string[]; allowedVersions?: Record<string, string> };
    };
  },
  overrideKeys: string[],
): Record<string, string> | undefined {
  // Strip selector-shaped overrides (e.g. `parent>@vitest/browser-playwright`)
  // whose target is a removed package, before the exact-key sweep below.
  dropRemovePackageOverrideKeys(pkg.pnpm?.overrides);
  // Remove Vite-managed keys from pnpm.overrides
  const catalogOverrides: Record<string, string> = {};
  const overrides = pkg.pnpm?.overrides;
  for (const key of [...overrideKeys, ...PROVIDER_OVERRIDE_DROP_NAMES]) {
    const value = overrides?.[key];
    if (value) {
      if (overrideKeys.includes(key) && value.startsWith('catalog:')) {
        catalogOverrides[key] = value;
      }
      delete overrides[key];
    }
  }
  // Remove dependency selectors targeting vite
  for (const key in pkg.pnpm?.overrides) {
    if (key.includes('>')) {
      const splits = key.split('>');
      if (splits[splits.length - 1].trim() === 'vite') {
        delete pkg.pnpm.overrides[key];
      }
    }
  }
  // Collect remaining overrides to move to pnpm-workspace.yaml then delete all
  // (pnpm ignores workspace-level overrides when pnpm.overrides exists in package.json)
  let remaining: Record<string, string> | undefined;
  if (Object.keys(catalogOverrides).length > 0) {
    remaining = { ...catalogOverrides };
  }
  if (pkg.pnpm?.overrides && Object.keys(pkg.pnpm.overrides).length > 0) {
    remaining = { ...remaining, ...pkg.pnpm.overrides };
  }
  delete pkg.pnpm?.overrides;
  // Only remove Vite-managed peerDependencyRules entries, preserve custom ones
  cleanupPeerDependencyRules(pkg.pnpm?.peerDependencyRules, overrideKeys);
  if (pkg.pnpm?.peerDependencyRules && Object.keys(pkg.pnpm.peerDependencyRules).length === 0) {
    delete pkg.pnpm.peerDependencyRules;
  }
  if (pkg.pnpm && Object.keys(pkg.pnpm).length === 0) {
    delete pkg.pnpm;
  }
  return remaining;
}

/**
 * Move remaining non-Vite pnpm.overrides from package.json to pnpm-workspace.yaml.
 * pnpm ignores workspace-level overrides when pnpm.overrides exists in package.json,
 * so all overrides must live in pnpm-workspace.yaml.
 */
function migratePnpmOverridesToWorkspaceYaml(
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

type DependencyBag = {
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
  optionalDependencies?: Record<string, string>;
  peerDependencies?: Record<string, string>;
};

function hasOwnWebdriverioDependency(pkg: DependencyBag): boolean {
  for (const name of WEBDRIVERIO_ALLOW_SIGNAL_DEPS) {
    if (
      pkg.dependencies?.[name] ??
      pkg.devDependencies?.[name] ??
      pkg.optionalDependencies?.[name] ??
      pkg.peerDependencies?.[name]
    ) {
      return true;
    }
  }
  return false;
}

function workspaceUsesWebdriverio(
  rootDir: string,
  packages: WorkspacePackage[] | undefined,
): boolean {
  const rootPkg = readPackageJsonIfExists(path.join(rootDir, 'package.json'));
  if (rootPkg && hasOwnWebdriverioDependency(rootPkg)) {
    return true;
  }
  // Source-only signal: a package may target the webdriverio provider purely
  // through imports (e.g. `vite-plus/test/browser-webdriverio`) without a
  // declared dep yet. The migration injects the provider for those, so the
  // driver postinstalls must be allowed too.
  if (usesWebdriverioProvider(rootDir)) {
    return true;
  }
  if (!packages) {
    return false;
  }
  for (const pkg of packages) {
    const packageDir = path.join(rootDir, pkg.path);
    const subPkg = readPackageJsonIfExists(path.join(packageDir, 'package.json'));
    if (subPkg && hasOwnWebdriverioDependency(subPkg)) {
      return true;
    }
    if (usesWebdriverioProvider(packageDir)) {
      return true;
    }
  }
  return false;
}

function readPackageJsonIfExists(packageJsonPath: string): DependencyBag | undefined {
  if (!fs.existsSync(packageJsonPath)) {
    return undefined;
  }
  try {
    return readJsonFile(packageJsonPath) as DependencyBag;
  } catch {
    return undefined;
  }
}

// pnpm v10 introduced the map-shaped `allowBuilds` and removed the implicit
// "build everything" default; v9 (>= 9.5) gates builds via the list-shaped
// `onlyBuiltDependencies`. Both live in pnpm-workspace.yaml or in
// `package.json`'s `pnpm` field — vp migrate writes to whichever sink the
// rest of the migration is already touching.
function pnpmMajor(version: string | undefined): number | undefined {
  const coerced = version ? semver.coerce(version)?.version : undefined;
  return coerced ? semver.major(coerced) : undefined;
}

function applyBuildAllowanceToPackageJsonPnpm(
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
 * Remove only Vite-managed entries from peerDependencyRules, preserving custom ones.
 */
function cleanupPeerDependencyRules(
  peerDependencyRules:
    | { allowAny?: string[]; allowedVersions?: Record<string, string> }
    | undefined,
  overrideKeys: string[],
): void {
  if (!peerDependencyRules) {
    return;
  }
  if (Array.isArray(peerDependencyRules.allowAny)) {
    peerDependencyRules.allowAny = peerDependencyRules.allowAny.filter(
      (key) => !overrideKeys.includes(key),
    );
    if (peerDependencyRules.allowAny.length === 0) {
      delete peerDependencyRules.allowAny;
    }
  }
  if (peerDependencyRules.allowedVersions) {
    for (const key of overrideKeys) {
      delete peerDependencyRules.allowedVersions[key];
    }
    if (Object.keys(peerDependencyRules.allowedVersions).length === 0) {
      delete peerDependencyRules.allowedVersions;
    }
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

// Read a SINGLE directory's `.yarnrc.yml` scalar value for `key` (or undefined when
// the file/key is absent or non-string). Malformed YAML throws inside `readYamlFile`,
// so guard with try/catch — a broken ancestor rc must not abort the migration.
//
// Values are taken VERBATIM: Yarn's `${VAR}` / `${VAR:-default}` string interpolation
// is NOT evaluated. An interpolated `nmHoistingLimits`/`nodeLinker` therefore won't
// match the literal `'workspaces'`/`'node-modules'` the caller compares against, so the
// hoisting fix conservatively does NOTHING for it — a no-op (and never a spurious
// mutation), the same outcome as a repo with no hoisting handling at all. Faithfully
// evaluating Yarn interpolation would mean reimplementing Yarn's config loader (or
// shelling out to `yarn config get`, a fragile pre-install process dependency), which
// is out of scope for this best-effort safety net.
//
// The filename is the literal `.yarnrc.yml`, not Yarn's `YARN_RC_FILENAME`-renamed rc.
// `YARN_RC_FILENAME` support is intentionally out of scope: the rest of the Yarn
// migration (catalog/`nodeLinker`/`npmPreapprovedPackages` writes in `rewriteYarnrcYml`
// et al.) only ever writes `.yarnrc.yml`, so reading a renamed rc here would be a
// partial, inconsistent treatment — and a repo with `YARN_RC_FILENAME` set cannot be
// migrated at all until the write path also honours it (a separate, larger change).
// Keeping reads and writes on the same `.yarnrc.yml` is the consistent behaviour.
function readYarnrcValue(dir: string, key: string): string | undefined {
  const yarnrcYmlPath = path.join(dir, '.yarnrc.yml');
  if (!fs.existsSync(yarnrcYmlPath)) {
    return undefined;
  }
  try {
    const doc = readYamlFile(yarnrcYmlPath) as Record<string, unknown> | null;
    const value = doc?.[key];
    return typeof value === 'string' ? value : undefined;
  } catch {
    return undefined;
  }
}

// Resolve the EFFECTIVE value Yarn would apply for a config `key` (and its
// `YARN_<KEY>` env override) for a project rooted at `workspaceRootDir`, matching
// Yarn 4.17 precedence (all verified with `yarn config get`):
//   1. the `YARN_*` environment variable wins over every `.yarnrc.yml` (e.g.
//      `YARN_NM_HOISTING_LIMITS`, `YARN_NODE_LINKER`);
//   2. otherwise Yarn merges `.yarnrc.yml` across the project root AND its ancestor
//      directories, the CLOSEST file that defines the key winning — so a key set only
//      in an ancestor rc is in effect, while a workspace-root value overrides it.
// So check the env var, then walk UP from the workspace root, then finally the home
// `~/.yarnrc.yml`, returning the first DEFINED value; undefined when none set it (the
// caller applies Yarn's default). The ancestor walk starts AT the workspace root,
// never below it — a sub-workspace's own `.yarnrc.yml` is not part of Yarn's
// install-time config resolution and must not shadow the root.
//
// The home rc is consulted LAST (lowest precedence, below the project/ancestor chain
// — verified with Yarn 4.17: a project-root value beats the home value). For a project
// UNDER $HOME the ancestor walk already passed through $HOME, so the explicit read is
// redundant; it matters for projects OUTSIDE $HOME (e.g. devcontainers/Codespaces
// mount the repo under /workspaces while $HOME is /home/<user>), where Yarn still
// reads the home rc and the ancestor walk would otherwise miss it.
function resolveEffectiveYarnConfigValue(
  workspaceRootDir: string,
  key: string,
  envVar: string,
): string | undefined {
  const fromEnv = process.env[envVar]?.trim();
  if (fromEnv) {
    return fromEnv;
  }
  let dir = path.resolve(workspaceRootDir);
  for (;;) {
    const value = readYarnrcValue(dir, key);
    if (value !== undefined) {
      return value;
    }
    const parent = path.dirname(dir);
    if (parent === dir) {
      break;
    }
    dir = parent;
  }
  const home = os.homedir();
  return home ? readYarnrcValue(home, key) : undefined;
}

// True when `dir`'s package.json declares a `workspaces` field — i.e. `dir` is a
// workspace (Yarn project) root. `workspaces` may be an array or an object
// (`{ packages: [...] }`); both are truthy.
function dirIsWorkspaceRoot(dir: string): boolean {
  const pkgJsonPath = path.join(dir, 'package.json');
  if (!fs.existsSync(pkgJsonPath)) {
    return false;
  }
  try {
    const pkg = readJsonFile(pkgJsonPath) as { workspaces?: unknown };
    return pkg.workspaces != null;
  } catch {
    return false;
  }
}

// Walk up from a workspace directory to the nearest ancestor that IS a workspace
// root (its package.json declares `workspaces`) — the real Yarn project root — and
// return that directory plus the EFFECTIVE `nmHoistingLimits` and `nodeLinker`
// resolved across env + the `.yarnrc.yml` chain at and above that root. Keying on the
// workspace-root marker (NOT the nearest `.yarnrc.yml`) is deliberate: a package-local
// `.yarnrc.yml` written under a sub-package (e.g. by `vp create` / install) must not
// shadow the real root's limit, while a limit set in an ancestor `.yarnrc.yml` above
// the root is still honoured (Yarn merges the ancestor chain). This lets
// `rewriteMonorepoProject` discover the layout for ANY caller without it being
// threaded as an argument (the omitted-arg path was a missed-auto-fix bug class), and
// lets the caller tell whether the workspace it is rewriting IS the root (the root's
// deps already hoist to the top, so it must never be opted out). `nodeLinker` gates
// the fix: `nmHoistingLimits` only splits packages under the `node-modules` linker, so
// a PnP project (Yarn's default) is left untouched. undefined when no workspace root
// is found up to the filesystem root.
function findYarnWorkspaceHoisting(
  startDir: string,
): { rootDir: string; limit: string | undefined; nodeLinker: string | undefined } | undefined {
  let dir = path.resolve(startDir);
  for (;;) {
    if (dirIsWorkspaceRoot(dir)) {
      return {
        rootDir: dir,
        limit: resolveEffectiveYarnConfigValue(dir, 'nmHoistingLimits', 'YARN_NM_HOISTING_LIMITS'),
        nodeLinker: resolveEffectiveYarnConfigValue(dir, 'nodeLinker', 'YARN_NODE_LINKER'),
      };
    }
    const parent = path.dirname(dir);
    if (parent === dir) {
      return undefined;
    }
    dir = parent;
  }
}

// Opt a single workspace OUT of the INHERITED root `nmHoistingLimits` isolation by
// setting its own `installConfig.hoistingLimits: none`, so its `vite-plus` (and
// thus the bundled `vitest` family) hoists to the single shared root copy the
// runner bin resolves to. Scoped to workspaces the migration adds `vite-plus` to,
// so unrelated workspaces are untouched. `none` is Yarn's DEFAULT hoisting
// behaviour, so this only re-enables ordinary deduping — it never force-promotes a
// conflicting version to root.
//
// Only relaxes the INHERITED root limit: if the workspace already carries an
// EXPLICIT `installConfig.hoistingLimits` we leave it as-is. Overwriting it would
// clobber an intentional per-workspace invariant (e.g. a React Native `example`
// that isolates its whole tree for Metro and happens to also use Vite+ for tests),
// and that field governs the workspace's ENTIRE dependency tree, not just the
// vitest family. Idempotent: a no-op when any explicit value is already present.
function setYarnWorkspaceHoistingOptOut(pkg: {
  installConfig?: { hoistingLimits?: string };
}): void {
  if (pkg.installConfig?.hoistingLimits !== undefined) {
    return;
  }
  pkg.installConfig = { ...pkg.installConfig, hoistingLimits: 'none' };
}

// Resolve the Yarn workspace-hoisting isolation for a workspace that now depends on
// `vite-plus`. `rootLimit` is the effective `nmHoistingLimits` and `nodeLinker` the
// effective linker (both undefined for non-Yarn repos or an unset key). Either
// auto-fixes the workspace (mutating `pkg`) or, when the split cannot be fixed from
// package.json, warns so the migration never reports success while `vp test` is still
// known-broken.
function applyYarnWorkspaceHoistingFix(
  pkg: { installConfig?: { hoistingLimits?: string } },
  rootLimit: string | undefined,
  nodeLinker: string | undefined,
  workspaceLabel: string,
  report?: MigrationReport,
): void {
  // `nmHoistingLimits`/`installConfig.hoistingLimits` only govern the `node-modules`
  // linker — they physically isolate copies there. Under Plug'n'Play (Yarn's DEFAULT
  // when `nodeLinker` is unset) resolution is virtual: no duplicate `@vitest/runner`
  // can exist, so neither the auto-fix nor the warning applies. Writing an opt-out
  // there would be a spurious source mutation that weakens isolation if the repo later
  // switches linkers, so skip everything unless the linker is `node-modules`.
  if (nodeLinker !== 'node-modules') {
    return;
  }
  // `workspaces` isolation with no explicit per-workspace limit is the one layout a
  // `none` opt-out deduplicates — fix it silently.
  if (rootLimit === 'workspaces' && pkg.installConfig?.hoistingLimits === undefined) {
    setYarnWorkspaceHoistingOptOut(pkg);
    return;
  }
  // Layouts we must NOT (or cannot) auto-fix, but which still isolate this
  // workspace's `vitest`/`vite-plus` copy so `vp test` can crash with a split
  // `@vitest/runner`:
  //   - the INHERITED root `dependencies` limit (a `none` opt-out does not dedupe
  //     it — verified), and
  //   - the workspace's OWN explicit isolating `installConfig.hoistingLimits`
  //     (`workspaces`/`dependencies`), which isolates it regardless of the root
  //     value (incl. root unset or `none`) and is intentional, so it is preserved
  //     rather than clobbered.
  // Surface a manual step for both rather than report a silently broken migration.
  const explicit = pkg.installConfig?.hoistingLimits;
  const isolatedByRoot = rootLimit === 'dependencies';
  const isolatedByWorkspace = explicit === 'workspaces' || explicit === 'dependencies';
  if (isolatedByRoot || isolatedByWorkspace) {
    warnMigration(
      `Yarn workspace "${workspaceLabel}" isolates dependency hoisting ` +
        `(hoistingLimits: ${explicit ?? rootLimit}), so it keeps its own ` +
        `\`vitest\`/\`vite-plus\` copy and \`vp test\` may crash with a split ` +
        `\`@vitest/runner\`. Dedupe them to a single copy — relax this workspace's ` +
        `hoisting isolation or pin one \`vitest\` for the workspace.`,
      report,
    );
  }
}

function rewriteYarnrcYml(projectPath: string): void {
  const yarnrcYmlPath = path.join(projectPath, '.yarnrc.yml');
  if (!fs.existsSync(yarnrcYmlPath)) {
    fs.writeFileSync(yarnrcYmlPath, '');
  }

  editYamlFile(yarnrcYmlPath, (doc) => {
    if (!doc.has('nodeLinker')) {
      doc.set('nodeLinker', 'node-modules');
    }
    // Vite+ pins the vitest family to exact, sometimes freshly published,
    // versions. Yarn 4 hardened mode (auto-enabled for public-PR installs)
    // quarantines packages younger than `npmMinimalAgeGate`, which makes
    // `yarn install` fail on a just-released vitest pin. Preapprove the family
    // so the Vite+-managed versions install regardless of release age; the
    // `@vitest/*` glob also covers the optional `@vitest/browser-*` peers that
    // are not in the override set. MERGE into any existing list (e.g. a project
    // that already preapproves private packages) instead of skipping when set,
    // otherwise the gate could still reject the freshly pinned vitest.
    let npmPreapprovedPackages = doc.getIn(['npmPreapprovedPackages']) as YAMLSeq<Scalar<string>>;
    if (!npmPreapprovedPackages) {
      npmPreapprovedPackages = new YAMLSeq();
    }
    const existingPreapproved = new Set(npmPreapprovedPackages.items.map((n) => n.value));
    for (const pkg of VITEST_AGE_GATE_EXEMPT_PACKAGES) {
      if (!existingPreapproved.has(pkg)) {
        npmPreapprovedPackages.add(scalarString(pkg));
      }
    }
    doc.setIn(['npmPreapprovedPackages'], npmPreapprovedPackages);
    // catalog
    rewriteCatalog(doc);
  });
}

/**
 * Rewrite catalog in pnpm-workspace.yaml or .yarnrc.yml
 * @param doc - The document to rewrite
 */
function getCatalogDependencySpec(
  currentValue: string | undefined,
  version: string,
  supportCatalog: boolean,
  options?: {
    dependencyField?: PackageJsonDependencyField;
    dependencyName?: string;
    packageManager?: PackageManager;
    catalogDependencyResolver?: CatalogDependencyResolver;
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
  return currentValue?.startsWith('catalog:') ? currentValue : 'catalog:';
}

function isVitePlusOverrideSpec(value: string): boolean {
  return (
    Object.values(VITE_PLUS_OVERRIDE_PACKAGES).includes(value) ||
    value.startsWith('npm:@voidzero-dev/vite-plus-')
  );
}

function createCatalogDependencyResolver(
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
    const workspacesObj =
      pkg.workspaces && !Array.isArray(pkg.workspaces) ? pkg.workspaces : undefined;
    const fromWorkspaces = createCatalogDependencyResolverFromCatalogs(
      workspacesObj?.catalog,
      workspacesObj?.catalogs,
    );
    const fromPkg = createCatalogDependencyResolverFromCatalogs(pkg.catalog, pkg.catalogs);
    return (catalogSpec, dependencyName) =>
      fromWorkspaces(catalogSpec, dependencyName) ?? fromPkg(catalogSpec, dependencyName);
  }
  return undefined;
}

function createCatalogDependencyResolverFromCatalogs(
  catalog: Record<string, string> | undefined,
  catalogs: Record<string, Record<string, string>> | undefined,
): CatalogDependencyResolver {
  return (catalogSpec, dependencyName) => {
    const catalogName = catalogSpec.slice('catalog:'.length);
    // pnpm/bun reserve `default` as the name of the top-level `catalog:` map,
    // so `catalog:default` resolves there, not a named `catalogs` entry.
    if (catalogName && catalogName !== 'default') {
      return catalogs?.[catalogName]?.[dependencyName];
    }
    return catalog?.[dependencyName];
  };
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

function rewriteCatalog(doc: YamlDocument): void {
  for (const [key, value] of Object.entries(VITE_PLUS_OVERRIDE_PACKAGES)) {
    // ERR_PNPM_CATALOG_IN_OVERRIDES  Could not resolve a catalog in the overrides: The entry for 'vite' in catalog 'default' declares a dependency using the 'file' protocol
    // ignore setting catalog if value starts with 'file:'
    if (value.startsWith('file:')) {
      continue;
    }
    doc.setIn(['catalog', key], scalarString(value));
  }
  if (!VITE_PLUS_VERSION.startsWith('file:')) {
    doc.setIn(['catalog', VITE_PLUS_NAME], scalarString(VITE_PLUS_VERSION));
  }
  for (const name of REMOVE_PACKAGES) {
    const path = ['catalog', name];
    if (doc.hasIn(path)) {
      doc.deleteIn(path);
    }
  }
  // Drop any entry still pointing at the deleted `vite-plus-test` wrapper.
  pruneYamlMapLegacyWrapperAliases(doc.getIn(['catalog']));

  const catalogs = doc.getIn(['catalogs']);
  if (!(catalogs instanceof YAMLMap)) {
    return;
  }
  for (const item of catalogs.items) {
    const catalogName = item.key instanceof Scalar ? item.key.value : undefined;
    if (typeof catalogName !== 'string' || !(item.value instanceof YAMLMap)) {
      continue;
    }
    for (const [key, value] of Object.entries(VITE_PLUS_OVERRIDE_PACKAGES)) {
      const catalogPath = ['catalogs', catalogName, key];
      if (!value.startsWith('file:') && doc.hasIn(catalogPath)) {
        doc.setIn(catalogPath, scalarString(value));
      }
    }
    const vitePlusPath = ['catalogs', catalogName, VITE_PLUS_NAME];
    if (!VITE_PLUS_VERSION.startsWith('file:') && doc.hasIn(vitePlusPath)) {
      doc.setIn(vitePlusPath, scalarString(VITE_PLUS_VERSION));
    }
    for (const name of REMOVE_PACKAGES) {
      const catalogPath = ['catalogs', catalogName, name];
      if (doc.hasIn(catalogPath)) {
        doc.deleteIn(catalogPath);
      }
    }
    pruneYamlMapLegacyWrapperAliases(item.value);
  }
}

function rewriteCatalogObject(catalog: Record<string, string>, addMissing: boolean): void {
  for (const [key, value] of Object.entries(VITE_PLUS_OVERRIDE_PACKAGES)) {
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
}

function rewriteCatalogsObject(catalogs: Record<string, Record<string, string>>): void {
  for (const catalog of Object.values(catalogs)) {
    rewriteCatalogObject(catalog, false);
  }
}

/**
 * Bun rejects vitest@4.1.9's `vite^6/^7/^8` peer-dep when the user's project
 * overrides `vite` to `@voidzero-dev/vite-plus-core` (whose package.json version
 * does not match those ranges). pnpm/yarn/npm all tolerate this redirect; bun
 * does not, and there is no `peerDependencyRules`-style escape hatch — only the
 * `[install] peer = false` setting in `bunfig.toml`.
 *
 * `vite-plus`/`@voidzero-dev/vite-plus-core` already provide the vite surface
 * the user needs, so disabling bun's auto-install of *missing* peers is safe in
 * this configuration: any vitest peer that's not already pulled in transitively
 * (jsdom, happy-dom, etc.) is marked optional upstream anyway.
 *
 * Writes/merges `bunfig.toml` at `projectPath` so the suppression applies on
 * the migration's reinstall AND every subsequent `bun install` the user runs.
 */
function ensureBunfigPeerSuppression(projectPath: string): void {
  const bunfigPath = path.join(projectPath, 'bunfig.toml');
  const block = '[install]\npeer = false\n';
  if (!fs.existsSync(bunfigPath)) {
    fs.writeFileSync(bunfigPath, block);
    return;
  }
  const existing = fs.readFileSync(bunfigPath, 'utf8');
  // Already configured? Leave the user's setting alone — they may have set
  // `peer = true` deliberately for some other reason and we shouldn't override.
  if (/^\s*peer\s*=\s*(true|false)\s*$/m.test(existing)) {
    return;
  }
  // Append under existing [install] section, or add a new section.
  const installSectionRe = /^\[install\][^[]*/m;
  const next = installSectionRe.test(existing)
    ? existing.replace(installSectionRe, (section) => `${section.trimEnd()}\npeer = false\n`)
    : `${existing.trimEnd()}\n\n${block}`;
  fs.writeFileSync(bunfigPath, next);
}

/**
 * Write catalog entries to root package.json for bun.
 * Bun stores catalogs in package.json under the `catalog` key,
 * unlike pnpm which uses pnpm-workspace.yaml.
 * @see https://bun.sh/docs/pm/catalogs
 */
function rewriteBunCatalog(projectPath: string): void {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }

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

    rewriteCatalogObject(catalog, true);
    pruneLegacyWrapperAliases(catalog);

    if (useWorkspacesCatalog) {
      workspacesObj.catalog = catalog;
      if (pkg.catalog) {
        rewriteCatalogObject(pkg.catalog, false);
        pruneLegacyWrapperAliases(pkg.catalog);
      }
    } else {
      pkg.catalog = catalog;
      if (workspacesObj?.catalog) {
        rewriteCatalogObject(workspacesObj.catalog, false);
        pruneLegacyWrapperAliases(workspacesObj.catalog);
      }
    }
    if (workspacesObj?.catalogs) {
      rewriteCatalogsObject(workspacesObj.catalogs);
      for (const named of Object.values(workspacesObj.catalogs)) {
        pruneLegacyWrapperAliases(named);
      }
    }
    if (pkg.catalogs) {
      rewriteCatalogsObject(pkg.catalogs);
      for (const named of Object.values(pkg.catalogs)) {
        pruneLegacyWrapperAliases(named);
      }
    }

    // bun overrides support catalog: references
    const overrides: Record<string, string> = { ...pkg.overrides };
    pruneLegacyWrapperAliases(overrides);
    for (const [key, value] of Object.entries(VITE_PLUS_OVERRIDE_PACKAGES)) {
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

  ensureBunfigPeerSuppression(projectPath);
}

/**
 * Rewrite root workspace package.json to add vite-plus dependencies
 * @param projectPath - The path to the project
 */
function rewriteRootWorkspacePackageJson(
  projectPath: string,
  packageManager: PackageManager,
  skipStagedMigration?: boolean,
  catalogDependencyResolver?: CatalogDependencyResolver,
  // Forwarded to `rewriteMonorepoProject` so the per-root lint-config
  // sanitizer can see hoisted deps in sibling workspace packages, not
  // just the root's own `package.json`.
  packages?: WorkspacePackage[],
  pnpmMajorVersion?: number,
  shouldAllowBrowserBuilds = false,
): void {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return;
  }

  let remainingPnpmOverrides: Record<string, string> | undefined;
  editJsonFile<{
    resolutions?: Record<string, string>;
    overrides?: Record<string, string>;
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
    peerDependencies?: Record<string, string>;
    optionalDependencies?: Record<string, string>;
    pnpm?: {
      overrides?: Record<string, string>;
      peerDependencyRules?: {
        allowAny?: string[];
        allowedVersions?: Record<string, string>;
      };
      allowBuilds?: Record<string, boolean>;
      onlyBuiltDependencies?: string[];
    };
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
    if (packageManager === PackageManager.yarn) {
      pkg.resolutions = {
        ...pkg.resolutions,
        // FIXME: yarn don't support catalog on resolutions
        // https://github.com/yarnpkg/berry/issues/6979
        ...VITE_PLUS_OVERRIDE_PACKAGES,
      };
    } else if (packageManager === PackageManager.npm) {
      pkg.overrides = {
        ...pkg.overrides,
        ...VITE_PLUS_OVERRIDE_PACKAGES,
      };
    } else if (packageManager === PackageManager.bun) {
      // bun overrides are handled in rewriteBunCatalog() with catalog: references
      // Bun walks transitive peer-deps before resolving overrides; vitest 4.1.9
      // declares peer `vite ^6 || ^7 || ^8` and aborts unless `vite` is a direct
      // dep at the workspace root. Mirror the override as a devDep; the override
      // configured in rewriteBunCatalog still redirects it to vite-plus-core.
      // See https://github.com/oven-sh/bun/issues/8406.
      pkg.devDependencies = {
        ...pkg.devDependencies,
        vite: getCatalogDependencySpec(
          pkg.devDependencies?.vite,
          VITE_PLUS_OVERRIDE_PACKAGES.vite,
          true,
        ),
      };
    } else if (packageManager === PackageManager.pnpm) {
      const overrideKeys = Object.keys(VITE_PLUS_OVERRIDE_PACKAGES);
      if (isForceOverrideMode()) {
        // Strip selector-shaped overrides (e.g. `parent>@vitest/browser-playwright`)
        // whose target is a removed package, before re-merging the user's
        // overrides into the new pnpm config.
        dropRemovePackageOverrideKeys(pkg.pnpm?.overrides);
        // In force-override mode, keep overrides in package.json pnpm.overrides
        // because pnpm ignores pnpm-workspace.yaml overrides when pnpm.overrides
        // exists in package.json (even with unrelated entries like rollup).
        pkg.pnpm = {
          ...pkg.pnpm,
          overrides: {
            ...pkg.pnpm?.overrides,
            ...VITE_PLUS_OVERRIDE_PACKAGES,
            [VITE_PLUS_NAME]: VITE_PLUS_VERSION,
          },
        };
      } else {
        for (const key of [...overrideKeys, ...PROVIDER_OVERRIDE_DROP_NAMES]) {
          if (pkg.resolutions?.[key]) {
            delete pkg.resolutions[key];
          }
        }
        remainingPnpmOverrides = cleanupPnpmOverridesForWorkspaceYaml(pkg, overrideKeys);
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

    // ensure vite-plus is in devDependencies
    if (!pkg.devDependencies?.[VITE_PLUS_NAME]) {
      pkg.devDependencies = {
        ...pkg.devDependencies,
        [VITE_PLUS_NAME]:
          packageManager === PackageManager.npm || VITE_PLUS_VERSION.startsWith('file:')
            ? VITE_PLUS_VERSION
            : 'catalog:',
      };
    }
    return pkg;
  });

  // Move remaining non-Vite pnpm.overrides to pnpm-workspace.yaml
  if (remainingPnpmOverrides) {
    migratePnpmOverridesToWorkspaceYaml(projectPath, remainingPnpmOverrides);
  }

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
  );
}

const RULES_YAML_PATH = path.join(rulesDir, 'vite-tools.yml');
const PREPARE_RULES_YAML_PATH = path.join(rulesDir, 'vite-prepare.yml');

// Cache YAML content to avoid repeated disk reads (called once per package in monorepos)
let cachedRulesYaml: string | undefined;
let cachedRulesYamlNoLintStaged: string | undefined;
let cachedPrepareRulesYaml: string | undefined;
function readRulesYaml(): string {
  cachedRulesYaml ??= fs.readFileSync(RULES_YAML_PATH, 'utf8');
  return cachedRulesYaml;
}
function getScriptRulesYaml(skipStagedMigration?: boolean): string {
  const yaml = readRulesYaml();
  if (!skipStagedMigration) {
    return yaml;
  }
  cachedRulesYamlNoLintStaged ??= yaml
    .split('\n\n\n')
    .filter((block) => !block.includes('id: replace-lint-staged'))
    .join('\n\n\n');
  return cachedRulesYamlNoLintStaged;
}
function readPrepareRulesYaml(): string {
  cachedPrepareRulesYaml ??= fs.readFileSync(PREPARE_RULES_YAML_PATH, 'utf8');
  return cachedPrepareRulesYaml;
}

type CoreMigrationWorkspace = {
  rootDir: string;
  packages?: WorkspacePackage[];
};

export type PendingCoreMigration = {
  scripts: boolean;
  tsconfigTypes: boolean;
};

export type CoreMigrationFinalizationResult = {
  scripts: boolean;
  tsconfigTypes: boolean;
  imports: boolean;
};

function getCoreMigrationProjectPaths(workspaceInfo: CoreMigrationWorkspace): string[] {
  return [
    workspaceInfo.rootDir,
    ...(workspaceInfo.packages ?? []).map((pkg) => path.join(workspaceInfo.rootDir, pkg.path)),
  ];
}

function hasCorePackageScriptRewrites(projectPath: string): boolean {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return false;
  }
  const pkg = readJsonFile(packageJsonPath) as { scripts?: Record<string, string> };
  if (!pkg.scripts) {
    return false;
  }
  return !!rewriteScripts(JSON.stringify(pkg.scripts), getScriptRulesYaml(true));
}

function rewriteCorePackageScripts(projectPath: string): boolean {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return false;
  }

  let changed = false;
  editJsonFile<{ scripts?: Record<string, string> }>(packageJsonPath, (pkg) => {
    if (!pkg.scripts) {
      return undefined;
    }
    const updated = rewriteScripts(JSON.stringify(pkg.scripts), getScriptRulesYaml(true));
    if (!updated) {
      return undefined;
    }
    pkg.scripts = JSON.parse(updated);
    changed = true;
    return pkg;
  });
  return changed;
}

export function detectPendingCoreMigration(
  workspaceInfo: CoreMigrationWorkspace,
): PendingCoreMigration {
  const projectPaths = getCoreMigrationProjectPaths(workspaceInfo);
  return {
    scripts: projectPaths.some((projectPath) => hasCorePackageScriptRewrites(projectPath)),
    tsconfigTypes: projectPaths.some((projectPath) => hasTsconfigTypesToRewrite(projectPath)),
  };
}

export function finalizeCoreMigrationForExistingVitePlus(
  workspaceInfo: CoreMigrationWorkspace,
  silent = false,
  report?: MigrationReport,
  pending = detectPendingCoreMigration(workspaceInfo),
): CoreMigrationFinalizationResult {
  const projectPaths = getCoreMigrationProjectPaths(workspaceInfo);
  const result: CoreMigrationFinalizationResult = {
    scripts: false,
    tsconfigTypes: false,
    imports: false,
  };

  if (pending.scripts) {
    for (const projectPath of projectPaths) {
      result.scripts = rewriteCorePackageScripts(projectPath) || result.scripts;
    }
  }

  if (pending.tsconfigTypes) {
    for (const projectPath of projectPaths) {
      result.tsconfigTypes =
        rewriteTsconfigTypes(projectPath, silent, report) || result.tsconfigTypes;
    }
  }

  result.imports = rewriteAllImports(workspaceInfo.rootDir, silent, report);

  return result;
}

type BootstrapPackageJson = {
  overrides?: Record<string, string>;
  resolutions?: Record<string, string>;
  devDependencies?: Record<string, string>;
  dependencies?: Record<string, string>;
  optionalDependencies?: Record<string, string>;
  pnpm?: {
    overrides?: Record<string, string>;
    peerDependencyRules?: {
      allowAny?: string[];
      allowedVersions?: Record<string, string>;
    };
  };
  packageManager?: string;
  devEngines?: { packageManager?: unknown; [key: string]: unknown };
};

export type VitePlusBootstrapResult = {
  changed: boolean;
  packageJson: boolean;
  packageManagerConfig: boolean;
  packageManagerField: boolean;
};

function getVitePlusOverridePackageName(dependencyName: string): string | undefined {
  if (dependencyName === 'vite') {
    return '@voidzero-dev/vite-plus-core';
  }
  if (dependencyName === 'vitest') {
    return '@voidzero-dev/vite-plus-test';
  }
  return undefined;
}

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
  const packageName = getVitePlusOverridePackageName(dependencyName);
  return packageName !== undefined && spec.includes(packageName);
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

function overridesSatisfyVitePlus(
  overrides: Record<string, string> | undefined,
  catalogDependencyResolver?: CatalogDependencyResolver,
): boolean {
  return Object.keys(VITE_PLUS_OVERRIDE_PACKAGES).every((dependencyName) =>
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

function vitePlusDependencyNeedsConcreteVersion(pkg: BootstrapPackageJson): boolean {
  const dependencyGroups = [pkg.devDependencies, pkg.dependencies, pkg.optionalDependencies];
  return dependencyGroups.some(
    (dependencies) => dependencies?.[VITE_PLUS_NAME]?.startsWith('catalog:') ?? false,
  );
}

function defaultCatalogVitePlusDependencyPending(
  pkg: BootstrapPackageJson,
  catalogDependencyResolver: CatalogDependencyResolver | undefined,
): boolean {
  const dependencyGroups = [pkg.devDependencies, pkg.dependencies, pkg.optionalDependencies];
  return dependencyGroups.some((dependencies) => {
    const spec = dependencies?.[VITE_PLUS_NAME];
    if (spec !== 'catalog:' && spec !== 'catalog:default') {
      return false;
    }
    return catalogDependencyResolver?.(spec, VITE_PLUS_NAME) !== VITE_PLUS_VERSION;
  });
}

function pnpmPeerDependencyRulesSatisfyVitePlus(
  peerDependencyRules:
    | { allowAny?: string[]; allowedVersions?: Record<string, string> }
    | undefined,
): boolean {
  const overrideKeys = Object.keys(VITE_PLUS_OVERRIDE_PACKAGES);
  const allowAny = new Set(peerDependencyRules?.allowAny ?? []);
  const allowedVersions = peerDependencyRules?.allowedVersions ?? {};
  return overrideKeys.every((key) => allowAny.has(key) && allowedVersions[key] === '*');
}

function npmVitePlusManagedDependenciesPending(pkg: BootstrapPackageJson): boolean {
  const dependencyGroups = [pkg.devDependencies, pkg.dependencies, pkg.optionalDependencies];
  return Object.keys(VITE_PLUS_OVERRIDE_PACKAGES).some((dependencyName) =>
    dependencyGroups.some(
      (dependencies) =>
        dependencies?.[dependencyName] !== undefined &&
        !overrideSpecSatisfiesVitePlus(dependencyName, dependencies[dependencyName]),
    ),
  );
}

function readPnpmWorkspaceCatalogDependencyResolver(
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

function readPnpmWorkspaceOverrides(projectPath: string): Record<string, string> | undefined {
  const pnpmWorkspaceYamlPath = path.join(projectPath, 'pnpm-workspace.yaml');
  if (!fs.existsSync(pnpmWorkspaceYamlPath)) {
    return undefined;
  }
  const doc = readYamlFile(pnpmWorkspaceYamlPath) as { overrides?: Record<string, string> } | null;
  return doc?.overrides;
}

function readPnpmWorkspacePeerDependencyRules(
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

function yarnrcSatisfiesVitePlus(projectPath: string): boolean {
  const yarnrcYmlPath = path.join(projectPath, '.yarnrc.yml');
  if (!fs.existsSync(yarnrcYmlPath)) {
    return false;
  }
  const doc = readYamlFile(yarnrcYmlPath) as {
    nodeLinker?: string;
    catalog?: Record<string, string>;
  } | null;
  return (
    !!doc &&
    Object.hasOwn(doc, 'nodeLinker') &&
    overridesSatisfyVitePlus(doc.catalog) &&
    (VITE_PLUS_VERSION.startsWith('file:') || doc.catalog?.[VITE_PLUS_NAME] === VITE_PLUS_VERSION)
  );
}

function ensurePnpmWorkspacePackages(projectPath: string, workspacePatterns: string[]): boolean {
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

function readBunCatalogDependencyResolver(pkg: {
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
  return (catalogSpec, dependencyName) =>
    fromWorkspaces(catalogSpec, dependencyName) ?? fromPkg(catalogSpec, dependencyName);
}

export function detectVitePlusBootstrapPending(
  projectPath: string,
  packageManager: PackageManager | undefined,
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

  if (!pkg.devDependencies?.[VITE_PLUS_NAME] || !hasPackageManagerPin(pkg)) {
    return true;
  }

  if (packageManager === undefined) {
    return true;
  }

  if (packageManager === PackageManager.yarn) {
    return !overridesSatisfyVitePlus(pkg.resolutions) || !yarnrcSatisfiesVitePlus(projectPath);
  }
  if (packageManager === PackageManager.npm) {
    return (
      vitePlusDependencyNeedsConcreteVersion(pkg) ||
      !overridesSatisfyVitePlus(pkg.overrides) ||
      npmVitePlusManagedDependenciesPending(pkg)
    );
  }
  if (packageManager === PackageManager.bun) {
    return !overridesSatisfyVitePlus(pkg.overrides, readBunCatalogDependencyResolver(pkg));
  }
  if (packageManager === PackageManager.pnpm) {
    if (pkg.pnpm) {
      return (
        vitePlusDependencyNeedsConcreteVersion(pkg) ||
        !overridesSatisfyVitePlus(pkg.pnpm.overrides) ||
        !pnpmPeerDependencyRulesSatisfyVitePlus(pkg.pnpm.peerDependencyRules)
      );
    }
    const resolver = readPnpmWorkspaceCatalogDependencyResolver(projectPath);
    return (
      defaultCatalogVitePlusDependencyPending(pkg, resolver) ||
      !overridesSatisfyVitePlus(readPnpmWorkspaceOverrides(projectPath), resolver) ||
      !pnpmPeerDependencyRulesSatisfyVitePlus(readPnpmWorkspacePeerDependencyRules(projectPath))
    );
  }

  return false;
}

function ensureVitePlusDependencySpecs(pkg: BootstrapPackageJson, version: string): boolean {
  let changed = false;
  if (version !== 'catalog:') {
    const dependencyGroups = [pkg.devDependencies, pkg.dependencies, pkg.optionalDependencies];
    for (const dependencies of dependencyGroups) {
      if (dependencies?.[VITE_PLUS_NAME]?.startsWith('catalog:')) {
        dependencies[VITE_PLUS_NAME] = version;
        changed = true;
      }
    }
  }
  if (pkg.devDependencies?.[VITE_PLUS_NAME]) {
    return changed;
  }
  pkg.devDependencies = {
    ...pkg.devDependencies,
    [VITE_PLUS_NAME]: version,
  };
  return true;
}

function ensureOverrideEntries(
  overrides: Record<string, string> | undefined,
  catalogDependencyResolver?: CatalogDependencyResolver,
): { overrides: Record<string, string>; changed: boolean } {
  const next = { ...overrides };
  let changed = false;
  for (const [dependencyName, overrideSpec] of Object.entries(VITE_PLUS_OVERRIDE_PACKAGES)) {
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

function ensureNpmVitePlusManagedDependencies(pkg: BootstrapPackageJson): boolean {
  let changed = false;
  const dependencyGroups = [pkg.devDependencies, pkg.dependencies, pkg.optionalDependencies];
  for (const [dependencyName, version] of Object.entries(VITE_PLUS_OVERRIDE_PACKAGES)) {
    for (const dependencies of dependencyGroups) {
      if (
        dependencies?.[dependencyName] !== undefined &&
        !overrideSpecSatisfiesVitePlus(dependencyName, dependencies[dependencyName])
      ) {
        dependencies[dependencyName] = version;
        changed = true;
      }
    }
  }
  return changed;
}

function ensurePnpmPeerDependencyRules(pkg: BootstrapPackageJson): boolean {
  const overrideKeys = Object.keys(VITE_PLUS_OVERRIDE_PACKAGES);
  pkg.pnpm ??= {};
  const peerDependencyRules = {
    ...pkg.pnpm.peerDependencyRules,
    allowAny: [...new Set([...(pkg.pnpm.peerDependencyRules?.allowAny ?? []), ...overrideKeys])],
    allowedVersions: {
      ...pkg.pnpm.peerDependencyRules?.allowedVersions,
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

  editJsonFile<
    BootstrapPackageJson & {
      workspaces?: NpmWorkspaces;
      catalog?: Record<string, string>;
      catalogs?: Record<string, Record<string, string>>;
    }
  >(packageJsonPath, (pkg) => {
    const usePnpmWorkspaceYaml = workspaceInfo.packageManager === PackageManager.pnpm && !pkg.pnpm;
    const supportCatalog =
      !VITE_PLUS_VERSION.startsWith('file:') &&
      (usePnpmWorkspaceYaml || workspaceInfo.packageManager === PackageManager.bun);
    let packageJsonChanged = ensureVitePlusDependencySpecs(
      pkg,
      supportCatalog ? 'catalog:' : VITE_PLUS_VERSION,
    );
    if (workspaceInfo.packageManager === PackageManager.npm) {
      packageJsonChanged = ensureNpmVitePlusManagedDependencies(pkg) || packageJsonChanged;
    }

    if (workspaceInfo.packageManager === PackageManager.yarn) {
      const ensured = ensureOverrideEntries(pkg.resolutions);
      if (ensured.changed) {
        pkg.resolutions = ensured.overrides;
        packageJsonChanged = true;
      }
    } else if (workspaceInfo.packageManager === PackageManager.npm) {
      const ensured = ensureOverrideEntries(pkg.overrides);
      if (ensured.changed) {
        pkg.overrides = ensured.overrides;
        packageJsonChanged = true;
      }
    } else if (workspaceInfo.packageManager === PackageManager.bun) {
      const ensured = ensureOverrideEntries(pkg.overrides, readBunCatalogDependencyResolver(pkg));
      if (ensured.changed) {
        pkg.overrides = ensured.overrides;
        packageJsonChanged = true;
      }
    } else if (workspaceInfo.packageManager === PackageManager.pnpm && pkg.pnpm) {
      const ensured = ensureOverrideEntries(pkg.pnpm.overrides);
      if (ensured.changed) {
        pkg.pnpm.overrides = ensured.overrides;
        packageJsonChanged = true;
      }
      packageJsonChanged = ensurePnpmPeerDependencyRules(pkg) || packageJsonChanged;
    }

    result.packageJson = packageJsonChanged;
    return pkg;
  });

  if (workspaceInfo.packageManager === PackageManager.pnpm) {
    const pkg = readJsonFile(packageJsonPath) as BootstrapPackageJson;
    if (!pkg.pnpm) {
      const pnpmWorkspaceYamlPath = path.join(projectPath, 'pnpm-workspace.yaml');
      const before = fs.existsSync(pnpmWorkspaceYamlPath)
        ? fs.readFileSync(pnpmWorkspaceYamlPath, 'utf-8')
        : undefined;
      const catalogDependencyResolver = readPnpmWorkspaceCatalogDependencyResolver(projectPath);
      if (
        defaultCatalogVitePlusDependencyPending(pkg, catalogDependencyResolver) ||
        !overridesSatisfyVitePlus(
          readPnpmWorkspaceOverrides(projectPath),
          catalogDependencyResolver,
        ) ||
        !pnpmPeerDependencyRulesSatisfyVitePlus(readPnpmWorkspacePeerDependencyRules(projectPath))
      ) {
        // Bootstrap only completes the catalog / overrides / peer rules for a
        // project that already uses Vite+. Build-script allowance stays owned
        // by the full migration paths, so pass an undefined pnpm major to skip
        // it (mirrors the single-arg call this path used before the signature
        // grew the build-allowance parameters).
        rewritePnpmWorkspaceYaml(projectPath, undefined, false);
      }
      if (fs.existsSync(pnpmWorkspaceYamlPath)) {
        ensurePnpmWorkspacePackages(projectPath, workspaceInfo.workspacePatterns);
      }
      const after = fs.existsSync(pnpmWorkspaceYamlPath)
        ? fs.readFileSync(pnpmWorkspaceYamlPath, 'utf-8')
        : undefined;
      result.packageManagerConfig = before !== after;
    }
  } else if (workspaceInfo.packageManager === PackageManager.yarn) {
    const yarnrcYmlPath = path.join(projectPath, '.yarnrc.yml');
    const before = fs.existsSync(yarnrcYmlPath)
      ? fs.readFileSync(yarnrcYmlPath, 'utf-8')
      : undefined;
    rewriteYarnrcYml(projectPath);
    const after = fs.readFileSync(yarnrcYmlPath, 'utf-8');
    result.packageManagerConfig = before !== after;
  } else if (workspaceInfo.packageManager === PackageManager.bun) {
    const before = fs.readFileSync(packageJsonPath, 'utf-8');
    rewriteBunCatalog(projectPath);
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

// Specifier fragments that signal vitest browser mode. Matched as substrings
// against source (see `sourceTreeReferencesAny`), so subpath imports are
// covered too. Each indicates the package drives vitest's browser runner:
//   - `@vitest/browser`         upstream, pre-migration (incl. `/context`,
//                               `/client`, … subpaths)
//   - `vite-plus/test/browser`  migrated (re-run on an already-migrated
//                               project); also covers `…/browser/context` and
//                               the `…/browser/providers/*` provider forms
//   - `vite-plus/test/{client,context,locators,matchers,utils}`  the published
//                               bare browser shims (`build.ts`
//                               `createBareBrowserShims`): each re-exports
//                               `@vitest/browser/<sub>` but DROPS the `browser`
//                               segment, so they carry no `browser` substring.
//                               The import rewriter flattens
//                               `@vitest/browser/{client,locators,matchers,
//                               utils}` to four of these in already-migrated
//                               source; `vite-plus/test/context` is reachable
//                               as the published bare export (the rewriter
//                               instead routes `@vitest/browser/context` to
//                               `vite-plus/test/browser/context`, already
//                               covered above). All five are browser-only
//                               re-exports, so they never collide with a
//                               non-browser vitest export.
//   - `vite-plus/test/plugins/browser`  prefix for the generated plugin shims
//                               (`build.ts` `PLUGIN_SHIM_ENTRIES`:
//                               `plugins/browser`, `plugins/browser-context`,
//                               `plugins/browser-client`, `plugins/browser-
//                               locators`, `plugins/browser-playwright`,
//                               `plugins/browser-preview`, `plugins/browser-
//                               webdriverio`), which re-export `@vitest/browser*`
//                               under a `/plugins/` segment that the
//                               `vite-plus/test/browser` hint does not match.
//                               One prefix covers the whole family.
//   - `vite-plus/test/internal/browser`  the published internal browser shim
//                               (`./test/internal/browser`, re-exports
//                               `vitest/internal/browser`) — also a `/browser`
//                               surface with no `vite-plus/test/browser`
//                               substring.
// Without a matching hint a package importing only one of these published
// browser surfaces (with no `@vitest/browser*` dep) would miss browser mode and
// skip pinning the direct `vitest` the browser optimizer needs resolvable from
// the package root under pnpm strict / Yarn PnP. This set is verified complete
// against every browser-surface `./test/*` export in package.json (those that
// re-export `@vitest/browser*` or `vitest/internal/browser`).
const VITEST_BROWSER_SPECIFIER_HINTS = [
  '@vitest/browser',
  'vite-plus/test/browser',
  'vite-plus/test/plugins/browser',
  'vite-plus/test/internal/browser',
  'vite-plus/test/client',
  'vite-plus/test/context',
  'vite-plus/test/locators',
  'vite-plus/test/matchers',
  'vite-plus/test/utils',
] as const;

// Specifier fragments that signal the WEBDRIVERIO provider specifically. Each
// is a prefix, matched as a substring, so subpath imports (`/context`,
// `/provider`, …) are covered too:
//   - `@vitest/browser-webdriverio`            pre-migration (incl. `/provider`,
//                                              `/context` subpaths)
//   - `vite-plus/test/browser-webdriverio`     migrated (re-run); covers
//                                              `…/context`
//   - `vite-plus/test/browser/providers/webdriverio`  migrated provider-subpath
//                                              form — the import rewriter maps
//                                              `@vitest/browser-webdriverio/provider`
//                                              here, so an already-migrated
//                                              project can contain it. Without
//                                              this hint a re-run would skip the
//                                              provider injection and the import
//                                              would break under pnpm strict /
//                                              Yarn PnP once the provider is no
//                                              longer a vite-plus runtime dep.
//   - `vite-plus/test/plugins/browser-webdriverio`  generated plugin shim that
//                                              re-exports `@vitest/browser-
//                                              webdriverio` wholesale; importing
//                                              it pulls in the (now opt-in)
//                                              provider, so it signals usage too.
const WEBDRIVERIO_PROVIDER_SPECIFIER_HINTS = [
  '@vitest/browser-webdriverio',
  'vite-plus/test/browser-webdriverio',
  'vite-plus/test/browser/providers/webdriverio',
  'vite-plus/test/plugins/browser-webdriverio',
] as const;

// Specifier fragments that signal the PLAYWRIGHT provider specifically — the
// playwright analogue of WEBDRIVERIO_PROVIDER_SPECIFIER_HINTS (same prefix /
// substring matching for `/provider`, `/context` subpaths). Playwright is opt-in
// just like webdriverio: vite-plus no longer bundles `@vitest/browser-playwright`
// at runtime, so a source-only user (e.g. `vite.config.ts` importing the
// provider via a `vite-plus/test/browser-playwright` shim with no declared dep)
// must still have the provider kept/injected for the rewritten import to resolve.
const PLAYWRIGHT_PROVIDER_SPECIFIER_HINTS = [
  '@vitest/browser-playwright',
  'vite-plus/test/browser-playwright',
  'vite-plus/test/browser/providers/playwright',
  'vite-plus/test/plugins/browser-playwright',
] as const;

// Per-provider source-scan hint lists, used to build the `providerSourceModes`
// map passed to `rewritePackageJson`.
const BROWSER_PROVIDER_SPECIFIER_HINTS: Record<string, readonly string[]> = {
  [WEBDRIVERIO_PROVIDER]: WEBDRIVERIO_PROVIDER_SPECIFIER_HINTS,
  [PLAYWRIGHT_PROVIDER]: PLAYWRIGHT_PROVIDER_SPECIFIER_HINTS,
};

// TypeScript/JavaScript source extensions scanned for browser-mode hints.
const VITEST_SCAN_EXTENSIONS = new Set([
  '.ts',
  '.mts',
  '.cts',
  '.tsx',
  '.js',
  '.mjs',
  '.cjs',
  '.jsx',
]);

// Directories never worth scanning for browser-mode hints — generated output,
// installed deps, VCS metadata. Skipped at every recursion level.
const VITEST_SCAN_SKIP_DIRS = new Set([
  'node_modules',
  'dist',
  'build',
  'out',
  'coverage',
  '.git',
  '.next',
  '.nuxt',
  '.svelte-kit',
  '.vite',
  '.cache',
]);

/**
 * Detect whether a package uses vitest's browser mode.
 *
 * Upstream `@vitest/browser` injects `optimizeDeps.include` entries of the form
 * `vitest > expect-type` (and `vitest > @vitest/snapshot > magic-string`,
 * `vitest > @vitest/expect > chai`). Vite resolves the leading `vitest` segment
 * from the Vite config root, so `vitest` MUST be resolvable as a package from
 * the consuming package's directory. In a pnpm strict (non-hoisted) layout,
 * `vitest` pulled in only transitively via `vite-plus` is NOT reachable from the
 * package root — the optimizer then fails with `Failed to resolve dependency`
 * and the browser test page hangs forever.
 *
 * When this returns true the migration adds `vitest` as a direct
 * devDependency so it is hoisted next to the package and the optimizer chain
 * resolves. The signal is any of the package's TS/JS files (config, workspace
 * config under any name, or test file) referencing `@vitest/browser*` or
 * `vite-plus/test/browser*`. The scan recurses through the package directory
 * (skipping `node_modules`, build output, VCS metadata) so browser config in a
 * non-standard filename or browser imports in test files are all caught.
 *
 * Recursion stops at nested `package.json` boundaries: a workspace sub-package
 * is a separate package that the migration scans on its own pass, so the root
 * package must not inherit a browser-mode signal from a sub-package.
 */
function sourceTreeReferencesAny(projectPath: string, hints: readonly string[]): boolean {
  const matchesHint = (content: string): boolean => hints.some((hint) => content.includes(hint));

  const scanDir = (dir: string, isRoot: boolean): boolean => {
    let entries: fs.Dirent[];
    try {
      entries = fs.readdirSync(dir, { withFileTypes: true });
    } catch {
      return false;
    }
    // A nested package.json marks a separate workspace package — it is migrated
    // (and scanned) on its own pass, so don't let its files leak into this one.
    if (!isRoot && entries.some((e) => e.isFile() && e.name === 'package.json')) {
      return false;
    }
    for (const entry of entries) {
      const entryPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        if (VITEST_SCAN_SKIP_DIRS.has(entry.name)) {
          continue;
        }
        if (scanDir(entryPath, false)) {
          return true;
        }
      } else if (entry.isFile() && VITEST_SCAN_EXTENSIONS.has(path.extname(entry.name))) {
        try {
          if (matchesHint(fs.readFileSync(entryPath, 'utf8'))) {
            return true;
          }
        } catch {
          // Unreadable file — ignore and keep scanning.
        }
      }
    }
    return false;
  };

  return scanDir(projectPath, true);
}

function usesVitestBrowserMode(projectPath: string): boolean {
  return sourceTreeReferencesAny(projectPath, VITEST_BROWSER_SPECIFIER_HINTS);
}

// Source-only signal that a package targets the WEBDRIVERIO provider — used to
// allow the edgedriver/geckodriver builds even when no dep is declared yet (the
// webdriverio-specific postinstall hazard; playwright has no such drivers). See
// `usesVitestBrowserMode` for the shared traversal semantics (extensions, skip
// dirs, nested-package boundary).
function usesWebdriverioProvider(projectPath: string): boolean {
  return sourceTreeReferencesAny(projectPath, WEBDRIVERIO_PROVIDER_SPECIFIER_HINTS);
}

// Source-scan signal per opt-in browser provider, used to inject the (opt-in,
// no-longer-bundled) provider + its framework peer even when no dep is declared
// yet (e.g. a `vite.config.ts` importing the provider via a `vite-plus/test`
// shim). Mirrors `usesWebdriverioProvider`'s scan for each provider.
function collectProviderSourceModes(projectPath: string): Record<string, boolean> {
  const modes: Record<string, boolean> = {};
  for (const provider of OPT_IN_BROWSER_PROVIDERS) {
    modes[provider] = sourceTreeReferencesAny(
      projectPath,
      BROWSER_PROVIDER_SPECIFIER_HINTS[provider],
    );
  }
  return modes;
}

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
  for (const [key, version] of Object.entries(VITE_PLUS_OVERRIDE_PACKAGES)) {
    for (const { dependencyField, dependencies } of dependencyGroups) {
      if (dependencies?.[key]) {
        dependencies[key] = getCatalogDependencySpec(dependencies[key], version, supportCatalog, {
          dependencyField,
          dependencyName: key,
          packageManager,
          catalogDependencyResolver,
        });
        needVitePlus = true;
      }
    }
  }
  // Force-override mode (ecosystem CI / `vp create` E2E) must re-pin any
  // pre-existing `vite-plus` range to the local tgz. Otherwise pnpm reads the
  // published vite-plus metadata for transitive dep resolution (e.g.
  // `@voidzero-dev/vite-plus-test`) even though the override replaces the
  // vite-plus package itself, dragging the stale wrapper into node_modules.
  if (isForceOverrideMode()) {
    for (const { dependencies } of dependencyGroups) {
      if (dependencies?.[VITE_PLUS_NAME]) {
        dependencies[VITE_PLUS_NAME] = VITE_PLUS_VERSION;
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
    // e.g., removing @vitest/browser-playwright should keep `playwright` in devDeps
    const peerDep = BROWSER_PROVIDER_PEER_DEPS[name];
    if (
      wasRemoved &&
      peerDep &&
      !pkg.devDependencies?.[peerDep] &&
      !pkg.dependencies?.[peerDep] &&
      !pkg.peerDependencies?.[peerDep] &&
      !pkg.optionalDependencies?.[peerDep]
    ) {
      pkg.devDependencies ??= {};
      pkg.devDependencies[peerDep] = '*';
    }
  }
  // The browser providers (webdriverio, playwright) are opt-in: vite-plus no
  // longer bundles them at runtime (each drags a heavy non-optional framework
  // peer), so a user targeting a provider must own it themselves for the
  // rewritten `vite-plus/test/browser-<provider>` import to resolve. Unlike the
  // rest of the `@vitest/*` family they are deliberately NOT in
  // VITE_PLUS_OVERRIDE_PACKAGES (so projects not using a provider stay
  // untouched), which means the normalization loop above does not pin them. We
  // pin each used provider here, to a CONCRETE version (no catalog entry is
  // written for an opt-in provider) so it self-resolves and stays aligned with
  // the bundled vitest, and we ensure its runtime framework peer
  // (`webdriverio` / `playwright`). (`@vitest/browser`/preview stay bundled +
  // stripped, handled in the REMOVE_PACKAGES loop above.)
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
    const installGroup = [pkg.dependencies, pkg.devDependencies, pkg.optionalDependencies].find(
      (deps) => deps?.[provider] !== undefined,
    );
    if (installGroup) {
      installGroup[provider] = VITEST_VERSION;
    } else {
      pkg.devDependencies ??= {};
      pkg.devDependencies[provider] = VITEST_VERSION;
    }
    const peer = BROWSER_PROVIDER_PEER_DEPS[provider]; // 'webdriverio' / 'playwright'
    const peerPresent =
      pkg.dependencies?.[peer] ??
      pkg.devDependencies?.[peer] ??
      pkg.peerDependencies?.[peer] ??
      pkg.optionalDependencies?.[peer];
    if (peer && !peerPresent) {
      pkg.devDependencies ??= {};
      pkg.devDependencies[peer] = '*';
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
      pkg.devDependencies ??= {};
      pkg.devDependencies.vite = viteOverride;
      needVitePlus = true;
    }
  }
  // Promote dep-derived signal to the same flag the source-scan feeds, so the
  // downstream "add direct `vitest`" branch fires for config-only browser-mode
  // setups too.
  const effectiveBrowserMode = vitestBrowserMode || hasBrowserDepSignal;
  // Trigger vite-plus install when a project has a vitest-adjacent package
  // (e.g. `vitest-browser-svelte`) that declares vitest as a peer dep — even
  // if the project has no vite/oxlint/tsdown dep to migrate. The peer dep is
  // satisfied by the upstream vitest that vite-plus bundles as a direct dep.
  // Note: peerDependencies count as "adjacent signal" but NOT as installed.
  const installableNames = [
    ...Object.keys(pkg.dependencies ?? {}),
    ...Object.keys(pkg.devDependencies ?? {}),
    ...Object.keys(pkg.optionalDependencies ?? {}),
  ];
  const adjacentSignals = [...installableNames, ...Object.keys(pkg.peerDependencies ?? {})];
  const isVitestAdjacent =
    !installableNames.includes('vitest') &&
    adjacentSignals.some((name) => name !== 'vitest' && name.includes('vitest'));
  // Normalize a pre-existing pinned vite-plus so sub-packages don't drift
  // from siblings: in catalog-supporting monorepos that's `catalog:`, under
  // force-override (file:) it's the tgz path. Preserve protocol-prefixed
  // specs (catalog:named, workspace:*, link:, file:, npm:, github:, git+/git:,
  // http(s)://) so deliberate user pins survive; only vanilla version ranges
  // (e.g. `^0.1.20`, `latest`) are rewritten.
  const canonicalVitePlusSpec =
    supportCatalog && !VITE_PLUS_VERSION.startsWith('file:') ? 'catalog:' : VITE_PLUS_VERSION;
  const existingVitePlus = pkg.devDependencies?.[VITE_PLUS_NAME];
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
  const needDirectVitest = needVitePlus || effectiveBrowserMode || isVitestAdjacent;
  if (needVitePlus || shouldNormalizeExistingVitePlus) {
    pkg.devDependencies = {
      ...pkg.devDependencies,
      [VITE_PLUS_NAME]: canonicalVitePlusSpec,
    };
  }
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
      (effectiveBrowserMode || Object.keys(installableDeps).some((name) => name.includes('vitest')))
    ) {
      pkg.devDependencies ??= {};
      pkg.devDependencies.vitest = getCatalogDependencySpec(
        undefined,
        VITEST_VERSION,
        supportCatalog,
      );
    }
  }
  return extractedStagedConfig;
}

// Returns true if the spec uses a known protocol prefix (catalog:, workspace:,
// link:, file:, npm:, github:, git+/git:, http(s)://) and so represents a
// deliberate user choice that should not be silently rewritten.
function isProtocolPinnedSpec(spec: string): boolean {
  return /^(catalog:|workspace:|link:|file:|npm:|github:|git[+:]|https?:\/\/)/.test(spec);
}

// Remove the "lint-staged" key from package.json after config has been
// successfully merged into vite.config.ts.
function removeLintStagedFromPackageJson(packageJsonPath: string): void {
  editJsonFile<{ 'lint-staged'?: Record<string, string | string[]> }>(packageJsonPath, (pkg) => {
    if (pkg['lint-staged']) {
      delete pkg['lint-staged'];
      return pkg;
    }
    return undefined;
  });
}

// Migrate standalone lint-staged config files into staged in vite.config.ts.
// JSON-parseable files are inlined automatically; non-JSON files get a warning.
function rewriteLintStagedConfigFile(projectPath: string, report?: MigrationReport): void {
  let hasUnsupported = false;

  for (const filename of LINT_STAGED_JSON_CONFIG_FILES) {
    const configPath = path.join(projectPath, filename);
    if (!fs.existsSync(configPath)) {
      continue;
    }
    if (filename === '.lintstagedrc' && !isJsonFile(configPath)) {
      warnMigration(
        `${displayRelative(configPath)} is not JSON format — please migrate to "staged" in vite.config.ts manually`,
        report,
      );
      hasUnsupported = true;
      continue;
    }
    // Merge the JSON config into vite.config.ts as "staged" and delete the file.
    // Skip if staged already exists in vite.config.ts (already migrated by rewritePackageJson).
    if (!hasStagedConfigInViteConfig(projectPath)) {
      const config = readJsonFile(configPath);
      const updated = rewriteScripts(JSON.stringify(config), readRulesYaml());
      const finalConfig = updated ? JSON.parse(updated) : config;
      if (!mergeStagedConfigToViteConfig(projectPath, finalConfig, true, report)) {
        // Merge failed — preserve the original config file so the user doesn't lose their rules
        continue;
      }
      fs.unlinkSync(configPath);
      if (report) {
        report.inlinedLintStagedConfigCount++;
      }
    } else {
      warnMigration(
        `${displayRelative(configPath)} found but "staged" already exists in vite.config.ts — please merge manually`,
        report,
      );
    }
  }
  // Non-JSON standalone files — warn
  for (const filename of LINT_STAGED_OTHER_CONFIG_FILES) {
    const configPath = path.join(projectPath, filename);
    if (!fs.existsSync(configPath)) {
      continue;
    }
    warnMigration(
      `${displayRelative(configPath)} — please migrate to "staged" in vite.config.ts manually`,
      report,
    );
    hasUnsupported = true;
  }
  if (hasUnsupported) {
    infoMigration(
      'Only "staged" in vite.config.ts is supported. See https://viteplus.dev/guide/migrate#lint-staged',
      report,
    );
  }
}

/**
 * Ensure vite.config.ts exists, create it if not
 * @returns The vite config filename
 */
function ensureViteConfig(
  projectPath: string,
  configs: ConfigFiles,
  silent = false,
  report?: MigrationReport,
): string {
  if (!configs.viteConfig) {
    configs.viteConfig = 'vite.config.ts';
    const viteConfigPath = path.join(projectPath, 'vite.config.ts');
    fs.writeFileSync(
      viteConfigPath,
      `import { defineConfig } from '${VITE_PLUS_NAME}';

export default defineConfig({});
`,
    );
    if (report) {
      report.createdViteConfigCount++;
    }
    if (!silent) {
      prompts.log.success(`✔ Created vite.config.ts in ${displayRelative(viteConfigPath)}`);
    }
  }
  return configs.viteConfig;
}

/**
 * Merge tsdown.config.* into vite.config.ts
 * - For JSON files: merge content directly into `pack` field and delete the JSON file
 * - For TS/JS files: import the config file
 */
function mergeTsdownConfigFile(
  projectPath: string,
  silent = false,
  report?: MigrationReport,
): void {
  const configs = detectConfigs(projectPath);
  if (!configs.tsdownConfig) {
    return;
  }
  const viteConfig = ensureViteConfig(projectPath, configs, silent, report);

  const fullViteConfigPath = path.join(projectPath, viteConfig);
  const fullTsdownConfigPath = path.join(projectPath, configs.tsdownConfig);

  // For JSON files, merge content directly and delete the file
  if (configs.tsdownConfig.endsWith('.json')) {
    mergeAndRemoveJsonConfig(projectPath, viteConfig, configs.tsdownConfig, 'pack', silent, report);
    return;
  }

  // For TS/JS files, import the config file
  const tsdownRelativePath = `./${configs.tsdownConfig}`;
  const result = mergeTsdownConfig(fullViteConfigPath, tsdownRelativePath);
  if (result.updated) {
    fs.writeFileSync(fullViteConfigPath, result.content);
    if (report) {
      report.tsdownImportCount++;
    }
    if (!silent) {
      prompts.log.success(
        `✔ Added import for ${displayRelative(fullTsdownConfigPath)} in ${displayRelative(fullViteConfigPath)}`,
      );
    }
  }
  // Show documentation link for manual merging since we only added the import
  infoMigration(
    `Please manually merge ${displayRelative(fullTsdownConfigPath)} into ${displayRelative(fullViteConfigPath)}, see https://viteplus.dev/guide/migrate#tsdown`,
    report,
  );
}

/**
 * Best-effort: derive the Oxlint rule-namespace a JS plugin package
 * contributes. Mirrors the conventions @oxlint/migrate uses when
 * translating ESLint configs, and the conventions Oxlint-native plugin
 * authors use (`oxlint-plugin-<name>` — see posva/pinia-colada in the
 * wild):
 *   `eslint-plugin-unocss`         → `unocss`        (rules: `unocss/order`)
 *   `oxlint-plugin-posva`          → `posva`         (rules: `posva/foo`)
 *   `@stylistic/eslint-plugin`     → `@stylistic`    (rules: `@stylistic/indent`)
 *   `@stylistic/eslint-plugin-ts`  → `@stylistic/ts` (rules: `@stylistic/ts/indent`)
 *   `@scope/oxlint-plugin-x`       → `@scope/x`
 *   anything else                  → the package name verbatim
 */
function deriveJsPluginNamespace(packageName: string): string {
  for (const prefix of ['eslint-plugin-', 'oxlint-plugin-']) {
    if (packageName.startsWith(prefix)) {
      const suffix = packageName.slice(prefix.length);
      return suffix || packageName;
    }
  }
  const scoped = packageName.match(/^(@[^/]+)\/(?:eslint|oxlint)-plugin(?:-(.+))?$/);
  if (scoped) {
    return scoped[2] ? `${scoped[1]}/${scoped[2]}` : scoped[1];
  }
  return packageName;
}

/**
 * Collect every dependency name declared across the root + workspace
 * `package.json` files after the ESLint cleanup has run. Used to verify
 * that JS plugins referenced by the generated `.oxlintrc.json` are
 * actually installable.
 */
function collectInstalledPackageNames(
  projectPath: string,
  packages?: WorkspacePackage[],
): Set<string> {
  const names = new Set<string>();
  const paths = [projectPath, ...(packages ?? []).map((p) => path.join(projectPath, p.path))];
  for (const dir of paths) {
    const pkgJsonPath = path.join(dir, 'package.json');
    if (!fs.existsSync(pkgJsonPath)) {
      continue;
    }
    let pkg: Record<string, Record<string, string> | undefined>;
    try {
      pkg = readJsonFile(pkgJsonPath) as typeof pkg;
    } catch {
      continue;
    }
    for (const field of [
      'devDependencies',
      'dependencies',
      'peerDependencies',
      'optionalDependencies',
    ] as const) {
      const deps = pkg[field];
      if (deps) {
        for (const name of Object.keys(deps)) {
          names.add(name);
        }
      }
    }
  }
  return names;
}

/**
 * Test whether a rule key (e.g. `@stylistic/ts/indent`) belongs to any
 * namespace in `namespaces`. We can't just split on the first `/` —
 * `@stylistic/eslint-plugin-ts` contributes the multi-segment namespace
 * `@stylistic/ts`, so the lookup has to try progressively longer
 * prefixes until one matches or we run out of slashes.
 */
function ruleKeyMatchesNamespace(key: string, namespaces: Set<string>): boolean {
  if (!key.includes('/')) {
    return true;
  }
  let idx = key.indexOf('/');
  while (idx !== -1) {
    if (namespaces.has(key.slice(0, idx))) {
      return true;
    }
    idx = key.indexOf('/', idx + 1);
  }
  return false;
}

/** Filter a rules object to only entries whose namespace is recognized. */
function filterRulesAgainstNamespaces(
  rules: Record<string, unknown>,
  namespaces: Set<string>,
): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(rules)) {
    if (ruleKeyMatchesNamespace(key, namespaces)) {
      out[key] = value;
    }
  }
  return out;
}

/**
 * Sort a jsPlugins array into installed entries (kept) and string
 * entries for packages that aren't present in the workspace. Object-form
 * entries (`{ name, specifier }`) and string entries that look like
 * local paths (`./X`, `/X`, `../X`) are passed through — Oxlint resolves
 * them itself.
 */
function partitionJsPlugins(
  entries: NonNullable<OxlintConfig['jsPlugins']>,
  availablePackages: Set<string>,
): {
  kept: NonNullable<OxlintConfig['jsPlugins']>;
  dropped: string[];
} {
  const kept: NonNullable<OxlintConfig['jsPlugins']> = [];
  const dropped: string[] = [];
  for (const entry of entries) {
    if (typeof entry !== 'string') {
      kept.push(entry);
      continue;
    }
    // Local-path specifiers don't go through `package.json`; preserve
    // them so users with hand-authored local plugin imports survive
    // a `vp migrate` re-run.
    if (entry.startsWith('./') || entry.startsWith('../') || entry.startsWith('/')) {
      kept.push(entry);
      continue;
    }
    if (availablePackages.has(entry)) {
      kept.push(entry);
    } else {
      dropped.push(entry);
    }
  }
  return { kept, dropped };
}

/** Build the set of rule-key namespaces backed by a given jsPlugins set. */
function jsPluginsToNamespaces(entries: NonNullable<OxlintConfig['jsPlugins']>): Set<string> {
  const ns = new Set<string>();
  for (const entry of entries) {
    if (typeof entry === 'string') {
      ns.add(deriveJsPluginNamespace(entry));
    } else if (entry && typeof entry === 'object' && 'name' in entry && entry.name) {
      ns.add(entry.name);
    }
  }
  // Empty-string namespace (e.g. from `eslint-plugin-` with no suffix)
  // would smuggle slash-prefixed rules through; drop it defensively.
  ns.delete('');
  return ns;
}

/**
 * Sanitize the `.oxlintrc.json` produced by `@oxlint/migrate` (in-place)
 * before it gets merged into `vite.config.ts`. Drop references that
 * won't resolve at lint time and warn the user.
 *
 * Why: `@oxlint/migrate` can emit `jsPlugins[]` / `plugins[]` / `rules`
 * entries referring to packages the user never installed (e.g.
 * translating `@unocss/eslint-config` into `eslint-plugin-unocss`),
 * to plugins outside Oxlint's native set, or under namespaces no
 * surviving plugin contributes. Without sanitization, `vp lint` aborts
 * with "Failed to load JS plugin" / "Plugin not found" before running
 * any rule. This produces a degraded-but-functional config instead.
 *
 * Per-override entries (`overrides[].jsPlugins`, `.plugins`, `.rules`)
 * are sanitized independently — an override can introduce its own
 * jsPlugin, so namespace availability is computed per-override (base
 * namespaces ∪ the override's own surviving jsPlugins' namespaces).
 */
function sanitizeMigratedOxlintConfig(
  config: OxlintConfig,
  availablePackages: Set<string>,
  report?: MigrationReport,
): void {
  // Track everything we strip so we can warn the user.
  const allDroppedJsPlugins = new Set<string>();
  const allDroppedPlugins = new Set<string>();

  // 1. Sanitize base-level jsPlugins.
  const baseSplit = partitionJsPlugins(config.jsPlugins ?? [], availablePackages);
  for (const n of baseSplit.dropped) {
    allDroppedJsPlugins.add(n);
  }
  if (config.jsPlugins && baseSplit.dropped.length > 0) {
    config.jsPlugins = baseSplit.kept;
  }

  // 2. Base namespaces = native plugins + surviving jsPlugins' namespaces.
  const baseNamespaces = new Set<string>(OXLINT_NATIVE_PLUGINS);
  for (const ns of jsPluginsToNamespaces(baseSplit.kept)) {
    baseNamespaces.add(ns);
  }

  // 3. Sanitize base-level plugins[] against base namespaces.
  if (config.plugins) {
    type PluginEntry = NonNullable<OxlintConfig['plugins']>[number];
    const keptPlugins: PluginEntry[] = [];
    for (const p of config.plugins) {
      if (baseNamespaces.has(p)) {
        keptPlugins.push(p);
      } else {
        allDroppedPlugins.add(p);
      }
    }
    if (keptPlugins.length !== config.plugins.length) {
      config.plugins = keptPlugins;
    }
  }

  // 4. Sanitize base rules. Guard the reassignment to avoid adding a
  // `rules: undefined` property that would shift downstream key
  // emission in the merged vite.config.ts.
  if (config.rules) {
    const filtered = filterRulesAgainstNamespaces(config.rules, baseNamespaces);
    if (Object.keys(filtered).length !== Object.keys(config.rules).length) {
      config.rules = filtered as typeof config.rules;
    }
  }

  // 5. Sanitize each override INDEPENDENTLY. An override can declare
  // its own `jsPlugins` / `plugins`, so we compute a per-override
  // namespace set: base namespaces ∪ the override's own surviving
  // jsPlugins' namespaces. If `override.plugins` is present it
  // replaces base.plugins per Oxlint's schema, but for namespace
  // resolution we still include the base set (rules under a base
  // namespace are still valid inside the override).
  if (Array.isArray(config.overrides)) {
    for (const override of config.overrides) {
      // Override jsPlugins.
      let overrideSurvivors: NonNullable<OxlintConfig['jsPlugins']> = [];
      if (override.jsPlugins) {
        const split = partitionJsPlugins(override.jsPlugins, availablePackages);
        for (const n of split.dropped) {
          allDroppedJsPlugins.add(n);
        }
        if (split.dropped.length > 0) {
          override.jsPlugins = split.kept;
        }
        overrideSurvivors = split.kept;
      }
      const overrideNamespaces = new Set<string>(baseNamespaces);
      for (const ns of jsPluginsToNamespaces(overrideSurvivors)) {
        overrideNamespaces.add(ns);
      }

      // Override plugins[].
      if (override.plugins) {
        type OverridePluginEntry = NonNullable<typeof override.plugins>[number];
        const keptOverridePlugins: OverridePluginEntry[] = [];
        for (const p of override.plugins) {
          if (overrideNamespaces.has(p)) {
            keptOverridePlugins.push(p);
          } else {
            allDroppedPlugins.add(p);
          }
        }
        if (keptOverridePlugins.length !== override.plugins.length) {
          override.plugins = keptOverridePlugins;
        }
      }

      // Override rules.
      if (override.rules) {
        const filtered = filterRulesAgainstNamespaces(override.rules, overrideNamespaces);
        if (Object.keys(filtered).length !== Object.keys(override.rules).length) {
          override.rules = filtered as typeof override.rules;
        }
      }
    }
  }

  // 6. Warn.
  //
  // We deliberately don't try to distinguish "we just removed this
  // package as part of the ESLint-ecosystem cleanup" from "the user
  // never had it installed" — the only honest signal we have is "not
  // in any package.json after cleanup", and a name-based heuristic
  // (matches `eslint-plugin-*`?) misclassifies the @oxlint/migrate
  // phantom-reference case (e.g. `@unocss/eslint-config` translating
  // into `eslint-plugin-unocss` even though the user never had it).
  // A single accurate message covers both paths.
  if (allDroppedJsPlugins.size > 0) {
    warnMigration(
      `Stripped JS plugin reference(s) from the generated lint config: ${[...allDroppedJsPlugins].join(', ')}. ` +
        'No matching package is present in this workspace, so loading them at lint time would fail. ' +
        'If you want their Oxlint coverage back, install each package (e.g. `vp install <name>`) and add its name back to `lint.jsPlugins` in vite.config.ts.',
      report,
    );
  }
  if (allDroppedPlugins.size > 0) {
    warnMigration(
      `Stripped unknown plugin reference(s) from the generated lint config: ${[...allDroppedPlugins].join(', ')}. ` +
        "These aren't native Oxlint plugins and no surviving JS plugin contributes them.",
      report,
    );
  }
}

/**
 * Merge oxlint and oxfmt config into vite.config.ts
 */
export function mergeViteConfigFiles(
  projectPath: string,
  silent = false,
  report?: MigrationReport,
  packages?: WorkspacePackage[],
  // For per-sub-package callers: the workspace root that `packages[].path`
  // is relative to. When undefined we resolve relative to `projectPath`
  // (correct for the top-level standalone/monorepo callers, where
  // projectPath IS the workspace root).
  workspaceRoot?: string,
): void {
  const configs = detectConfigs(projectPath);
  if (!configs.oxfmtConfig && !configs.oxlintConfig) {
    return;
  }
  const viteConfig = ensureViteConfig(projectPath, configs, silent, report);
  if (configs.oxlintConfig) {
    // Inject options.typeAware and options.typeCheck defaults before merging
    const fullOxlintPath = path.join(projectPath, configs.oxlintConfig);
    const oxlintJson = readJsonFile(fullOxlintPath, true) as OxlintConfig;
    if (!oxlintJson.options) {
      oxlintJson.options = {};
    }
    // Skip typeAware/typeCheck when tsconfig.json has baseUrl (unsupported by tsgolint)
    if (!hasBaseUrlInTsconfig(projectPath)) {
      if (oxlintJson.options.typeAware === undefined) {
        oxlintJson.options.typeAware = true;
      }
      if (oxlintJson.options.typeCheck === undefined) {
        oxlintJson.options.typeCheck = true;
      }
    } else {
      warnMigration(BASEURL_TSCONFIG_WARNING, report);
    }
    // Drop references to plugins / jsPlugins / rules that won't resolve
    // at lint time (e.g. `@oxlint/migrate` translating `@unocss/eslint-config`
    // → `eslint-plugin-unocss` even when that package isn't installed).
    // Resolve workspace package paths against `workspaceRoot` when the
    // caller is processing a sub-package — otherwise the sanitizer would
    // mistakenly look for `subPath/<sibling-pkg-path>` and miss the
    // hoisted deps it's supposed to see.
    sanitizeMigratedOxlintConfig(
      oxlintJson,
      collectInstalledPackageNames(workspaceRoot ?? projectPath, packages),
      report,
    );
    const normalizedOxlintConfig = ensureVitePlusImportRuleDefaults(oxlintJson);
    fs.writeFileSync(fullOxlintPath, JSON.stringify(normalizedOxlintConfig, null, 2));
    // merge oxlint config into vite.config.ts
    mergeAndRemoveJsonConfig(projectPath, viteConfig, configs.oxlintConfig, 'lint', silent, report);
  }
  if (configs.oxfmtConfig) {
    // merge oxfmt config into vite.config.ts
    mergeAndRemoveJsonConfig(projectPath, viteConfig, configs.oxfmtConfig, 'fmt', silent, report);
  }
}

/**
 * Inject typeAware and typeCheck defaults into vite.config.ts lint config.
 * Called after mergeViteConfigFiles() to handle the case where no .oxlintrc.json exists
 * (e.g., newly created projects from create-vite templates).
 */
export function injectLintTypeCheckDefaults(
  projectPath: string,
  silent = false,
  report?: MigrationReport,
): void {
  if (hasBaseUrlInTsconfig(projectPath)) {
    warnMigration(BASEURL_TSCONFIG_WARNING, report);
    return;
  }
  injectConfigDefaults(
    projectPath,
    'lint',
    '.vite-plus-lint-init.oxlintrc.json',
    JSON.stringify(
      createDefaultVitePlusLintConfig({
        includeTypeAwareDefaults: true,
      }),
    ),
    silent,
    report,
  );
}

export function injectFmtDefaults(
  projectPath: string,
  silent = false,
  report?: MigrationReport,
): void {
  injectConfigDefaults(
    projectPath,
    'fmt',
    '.vite-plus-fmt-init.oxfmtrc.json',
    JSON.stringify({}),
    silent,
    report,
  );
}

/**
 * Wire `create.defaultTemplate: '<scope>'` into the new monorepo's
 * `vite.config.ts`. The caller is `bin.ts`, only when scaffolding a
 * monorepo from a bundled `@org` manifest entry — that's the case where
 * the user just picked a template from a specific org and naturally
 * wants subsequent `vp create` invocations from the workspace to default
 * to that same org's picker.
 */
export function injectCreateDefaultTemplate(
  projectPath: string,
  scope: string,
  silent = false,
  report?: MigrationReport,
): void {
  if (!scope) {
    return;
  }
  injectConfigDefaults(
    projectPath,
    'create',
    '.vite-plus-create-init.json',
    JSON.stringify({ defaultTemplate: scope }),
    silent,
    report,
  );
}

function injectConfigDefaults(
  projectPath: string,
  configKey: string,
  tempFileName: string,
  tempFileContent: string,
  silent: boolean,
  report?: MigrationReport,
): void {
  const configs = detectConfigs(projectPath);
  if (configs.viteConfig && hasConfigKey(path.join(projectPath, configs.viteConfig), configKey)) {
    return;
  }

  const viteConfig = ensureViteConfig(projectPath, configs, silent, report);
  const tempConfigPath = path.join(projectPath, tempFileName);
  fs.writeFileSync(tempConfigPath, tempFileContent);
  const fullViteConfigPath = path.join(projectPath, viteConfig);
  let result;
  try {
    result = mergeJsonConfig(fullViteConfigPath, tempConfigPath, configKey);
  } finally {
    fs.rmSync(tempConfigPath, { force: true });
  }
  if (result.updated) {
    fs.writeFileSync(fullViteConfigPath, result.content);
  }
}

function mergeAndRemoveJsonConfig(
  projectPath: string,
  viteConfigPath: string,
  jsonConfigPath: string,
  configKey: string,
  silent = false,
  report?: MigrationReport,
): void {
  const fullViteConfigPath = path.join(projectPath, viteConfigPath);
  const fullJsonConfigPath = path.join(projectPath, jsonConfigPath);
  // Skip merge when the key is already present in vite.config.ts — the Rust
  // merge step always prepends, so without this guard a template that ships
  // both an inline `${configKey}:` block and a standalone JSON file (e.g.
  // create-fate's vite.config.ts + .oxfmtrc.jsonc) ends up with two of them.
  // AST-based check ignores comments, string-literal occurrences, and nested
  // keys (e.g. `plugins: [{ fmt: ... }]`).
  if (hasConfigKey(fullViteConfigPath, configKey)) {
    fs.unlinkSync(fullJsonConfigPath);
    if (!silent) {
      prompts.log.info(
        `${configKey} config already present in ${displayRelative(fullViteConfigPath)} — removed redundant ${displayRelative(fullJsonConfigPath)}`,
      );
    }
    return;
  }
  const result = mergeJsonConfig(fullViteConfigPath, fullJsonConfigPath, configKey);
  if (result.updated) {
    fs.writeFileSync(fullViteConfigPath, result.content);
    fs.unlinkSync(fullJsonConfigPath);
    if (report) {
      report.mergedConfigCount++;
    }
    if (!silent) {
      prompts.log.success(
        `✔ Merged ${displayRelative(fullJsonConfigPath)} into ${displayRelative(fullViteConfigPath)}`,
      );
    }
  } else {
    warnMigration(
      `Failed to merge ${displayRelative(fullJsonConfigPath)} into ${displayRelative(fullViteConfigPath)}`,
      report,
    );
    infoMigration(
      'Please complete the merge manually and follow the instructions in the documentation: https://viteplus.dev/config/',
      report,
    );
  }
}

/**
 * Merge a staged config object into vite.config.ts as `staged: { ... }`.
 * Writes the config to a temp JSON file, calls mergeJsonConfig NAPI, then cleans up.
 */
export function mergeStagedConfigToViteConfig(
  projectPath: string,
  stagedConfig: Record<string, string | string[]>,
  silent = false,
  report?: MigrationReport,
): boolean {
  const configs = detectConfigs(projectPath);
  const viteConfig = ensureViteConfig(projectPath, configs, silent, report);
  const fullViteConfigPath = path.join(projectPath, viteConfig);

  // Write staged config to a temp JSON file for mergeJsonConfig NAPI
  const tempJsonPath = path.join(projectPath, '.staged-config-temp.json');
  fs.writeFileSync(tempJsonPath, JSON.stringify(stagedConfig, null, 2));

  let result;
  try {
    result = mergeJsonConfig(fullViteConfigPath, tempJsonPath, 'staged');
  } finally {
    fs.unlinkSync(tempJsonPath);
  }

  if (result.updated) {
    fs.writeFileSync(fullViteConfigPath, result.content);
    if (report) {
      report.mergedStagedConfigCount++;
    }
    if (!silent) {
      prompts.log.success(`✔ Merged staged config into ${displayRelative(fullViteConfigPath)}`);
    }
    return true;
  } else {
    warnMigration(
      `Failed to merge staged config into ${displayRelative(fullViteConfigPath)}`,
      report,
    );
    infoMigration(
      `Please add staged config to ${displayRelative(fullViteConfigPath)} manually, see https://viteplus.dev/guide/migrate#lint-staged`,
      report,
    );
    return false;
  }
}

/**
 * Check if vite.config.ts already has a `staged` config key.
 */
export function hasStagedConfigInViteConfig(projectPath: string): boolean {
  const configs = detectConfigs(projectPath);
  if (!configs.viteConfig) {
    return false;
  }
  const viteConfigPath = path.join(projectPath, configs.viteConfig);
  const content = fs.readFileSync(viteConfigPath, 'utf8');
  return /\bstaged\s*:/.test(content);
}

/**
 * Wrap safe inline Vite plugin arrays with lazyPlugins so check/lint/fmt do not
 * eagerly execute plugin factories while loading vite.config.ts.
 */
function wrapLazyPluginsInViteConfig(
  projectPath: string,
  silent = false,
  report?: MigrationReport,
): void {
  const configs = detectConfigs(projectPath);
  if (!configs.viteConfig) {
    return;
  }

  const viteConfigPath = path.join(projectPath, configs.viteConfig);
  const result = wrapLazyPlugins(viteConfigPath);
  if (!result.updated) {
    return;
  }

  fs.writeFileSync(viteConfigPath, result.content);
  if (report) {
    report.wrappedPluginConfigCount++;
  }
  if (!silent) {
    prompts.log.success(
      `✔ Wrapped inline Vite plugins with lazyPlugins in ${displayRelative(viteConfigPath)}`,
    );
  }
}

/**
 * Rewrite imports in all TypeScript/JavaScript files under a directory
 * This rewrites vite/vitest imports to @voidzero-dev/vite-plus
 * @param projectPath - The root directory to search for files
 */
function rewriteAllImports(projectPath: string, silent = false, report?: MigrationReport): boolean {
  const result = rewriteImportsInDirectory(projectPath);
  const modified = result.modifiedFiles.length;
  const errors = result.errors.length;

  if (report) {
    report.rewrittenImportFileCount += modified;
    report.rewrittenImportErrors.push(
      ...result.errors.map((error) => ({
        path: displayRelative(error.path),
        message: error.message,
      })),
    );
  }

  if (!silent && modified > 0) {
    prompts.log.success(`Rewrote imports in ${modified === 1 ? 'one file' : `${modified} files`}`);
    prompts.log.info(result.modifiedFiles.map((file) => `  ${displayRelative(file)}`).join('\n'));
  }

  if (errors > 0) {
    if (report) {
      warnMigration(
        `${errors === 1 ? 'one file had an error' : `${errors} files had errors`} while rewriting imports`,
        report,
      );
    } else {
      prompts.log.warn(
        `⚠ ${errors === 1 ? 'one file had an error' : `${errors} files had errors`}:`,
      );
      for (const error of result.errors) {
        prompts.log.error(`  ${displayRelative(error.path)}: ${error.message}`);
      }
    }
  }
  return modified > 0;
}

/**
 * Check if the project has an unsupported husky version (<9.0.0).
 * Uses `semver.coerce` to handle ranges like `^8.0.0` → `8.0.0`.
 * When the specifier is a catalog reference (e.g. `"catalog:"`), resolves
 * it from the active package manager's catalog first — a `catalog:` spec is
 * only meaningful to the manager that owns the workspace, so we never read a
 * leftover/foreign catalog file. When it is still not coercible (e.g.
 * `"latest"`), falls back to the installed version in node_modules via
 * `detectPackageMetadata`.
 * Returns a reason string if hooks migration should be skipped, or null
 * if husky is absent or compatible.
 */
function checkUnsupportedHuskyVersion(
  projectPath: string,
  deps: Record<string, string> | undefined,
  prodDeps: Record<string, string> | undefined,
  packageManager: PackageManager | undefined,
): string | null {
  const huskyVersion = deps?.husky ?? prodDeps?.husky;
  if (!huskyVersion) {
    return null;
  }
  let coerced = semver.coerce(huskyVersion);
  if (coerced == null && packageManager != null && huskyVersion.startsWith('catalog:')) {
    const resolved = createCatalogDependencyResolver(projectPath, packageManager)?.(
      huskyVersion,
      'husky',
    );
    if (resolved) {
      coerced = semver.coerce(resolved);
    }
  }
  if (coerced == null) {
    const installed = detectPackageMetadata(projectPath, 'husky');
    if (installed) {
      coerced = semver.coerce(installed.version);
    }
    if (coerced == null) {
      return `Could not determine husky version from "${huskyVersion}" — please specify a semver-compatible version (e.g., "^9.0.0") and re-run migration.`;
    }
  }
  if (semver.satisfies(coerced, '<9.0.0')) {
    return 'Detected husky <9.0.0 — please upgrade to husky v9+ first, then re-run migration.';
  }
  return null;
}

const OTHER_HOOK_TOOLS = ['simple-git-hooks', 'lefthook', 'yorkie'] as const;

// Packages replaced by vite-plus built-in commands and should be removed from devDependencies
const REPLACED_HOOK_PACKAGES = ['husky', 'lint-staged'] as const;

function removeReplacedHookPackages(packageJsonPath: string): void {
  editJsonFile<{
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
  }>(packageJsonPath, (pkg) => {
    for (const name of REPLACED_HOOK_PACKAGES) {
      if (pkg.devDependencies?.[name]) {
        delete pkg.devDependencies[name];
      }
      if (pkg.dependencies?.[name]) {
        delete pkg.dependencies[name];
      }
    }
    return pkg;
  });
}

export function detectLegacyGitHooksMigrationCandidate(projectPath: string): boolean {
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return false;
  }
  const pkg = readJsonFile(packageJsonPath) as {
    scripts?: Record<string, string>;
    'lint-staged'?: unknown;
  };
  return getOldHooksDir(projectPath) !== undefined || pkg['lint-staged'] !== undefined;
}

/**
 * Walk up from `startPath` looking for `.git` (directory or file — submodules
 * use a `.git` file).  Returns the directory that contains `.git`, or `null`.
 */
function findGitRoot(startPath: string): string | null {
  let dir = startPath;
  while (true) {
    if (fs.existsSync(path.join(dir, '.git'))) {
      return dir;
    }
    const parent = path.dirname(dir);
    if (parent === dir) {
      return null;
    }
    dir = parent;
  }
}

/**
 * Normalize "husky install [dir]" → "husky [dir]" so downstream regex
 * and ast-grep rules can match a single pattern.
 */
function collapseHuskyInstall(script: string): string {
  return script.replace('husky install ', 'husky ').replace('husky install', 'husky');
}

/**
 * High-level helper: detect old hooks dir, set up git hooks, and rewrite
 * the prepare script.  Returns true if hooks were successfully installed.
 */
export function installGitHooks(
  projectPath: string,
  silent = false,
  report?: MigrationReport,
  packageManager?: PackageManager,
): boolean {
  const oldHooksDir = getOldHooksDir(projectPath);
  if (setupGitHooks(projectPath, oldHooksDir, silent, report, packageManager)) {
    rewritePrepareScript(projectPath);
    return true;
  }
  return false;
}

/**
 * Read-only probe: extract the old husky hooks directory from `scripts.prepare`
 * without modifying package.json. Returns undefined when no husky reference is found.
 */
export function getOldHooksDir(rootDir: string): string | undefined {
  const packageJsonPath = path.join(rootDir, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return undefined;
  }
  const pkg = readJsonFile(packageJsonPath) as { scripts?: { prepare?: string } };
  if (!pkg.scripts?.prepare) {
    return undefined;
  }
  const prepare = collapseHuskyInstall(pkg.scripts.prepare);
  const match = prepare.match(/\bhusky(?:\s+([\w./-]+))?/);
  if (!match) {
    return undefined;
  }
  return match[1] ?? '.husky';
}

/**
 * Pre-flight check: verify that git hooks can be set up for this project.
 * Returns `null` if hooks setup can proceed, or a warning reason string
 * explaining why hooks setup should be skipped.
 *
 * These checks are deterministic and read-only — they do not modify
 * the project in any way, making them safe to call before migration.
 *
 * `packageManager` is the project's detected manager; it scopes `catalog:`
 * resolution to that manager's catalog so a foreign catalog file is ignored.
 */
export function preflightGitHooksSetup(
  projectPath: string,
  packageManager?: PackageManager,
): string | null {
  const gitRoot = findGitRoot(projectPath);
  if (gitRoot && path.resolve(projectPath) !== path.resolve(gitRoot)) {
    return 'Subdirectory project detected — skipping git hooks setup. Configure hooks at the repository root.';
  }
  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return null; // silently skip
  }
  const pkgContent = readJsonFile(packageJsonPath);
  const deps = pkgContent.devDependencies as Record<string, string> | undefined;
  const prodDeps = pkgContent.dependencies as Record<string, string> | undefined;
  for (const tool of OTHER_HOOK_TOOLS) {
    if (deps?.[tool] || prodDeps?.[tool] || pkgContent[tool]) {
      return `Detected ${tool} — skipping git hooks setup. Please configure git hooks manually.`;
    }
  }
  const huskyReason = checkUnsupportedHuskyVersion(projectPath, deps, prodDeps, packageManager);
  if (huskyReason) {
    return huskyReason;
  }
  if (hasUnsupportedLintStagedConfig(projectPath)) {
    return 'Unsupported lint-staged config format — skipping git hooks setup. Please configure git hooks manually.';
  }
  return null;
}

/**
 * Set up git hooks with husky + lint-staged via vp commands.
 * Skips if another hook tool is detected (warns user).
 * Returns true if hooks were successfully set up, false if skipped.
 */
export function setupGitHooks(
  projectPath: string,
  oldHooksDir?: string,
  silent = false,
  report?: MigrationReport,
  packageManager?: PackageManager,
): boolean {
  const reason = preflightGitHooksSetup(projectPath, packageManager);
  if (reason) {
    warnMigration(reason, report);
    return false;
  }

  const packageJsonPath = path.join(projectPath, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return false;
  }

  const gitRoot = findGitRoot(projectPath);

  // Custom husky dirs (e.g. .config/husky) stay unchanged;
  // only the default .husky dir gets migrated to .vite-hooks.
  const isCustomDir = oldHooksDir != null && oldHooksDir !== '.husky';
  const hooksDir = isCustomDir ? oldHooksDir : '.vite-hooks';

  editJsonFile<{
    scripts?: Record<string, string>;
    devDependencies?: Record<string, string>;
    dependencies?: Record<string, string>;
  }>(packageJsonPath, (pkg) => {
    // Ensure vp config is present for projects that didn't have husky.
    // Skip when prepare contains "husky" — rewritePrepareScript (called after
    // setupGitHooks succeeds) will transform husky → vp config.
    if (!pkg.scripts) {
      pkg.scripts = {};
    }
    if (!pkg.scripts.prepare) {
      pkg.scripts.prepare = 'vp config';
    } else if (
      !pkg.scripts.prepare.includes('vp config') &&
      !/\bhusky\b/.test(pkg.scripts.prepare)
    ) {
      pkg.scripts.prepare = `vp config && ${pkg.scripts.prepare}`;
    }

    return pkg;
  });

  // Add staged config to vite.config.ts if not present
  let stagedMerged = hasStagedConfigInViteConfig(projectPath);
  const hasStandaloneConfig = hasStandaloneLintStagedConfig(projectPath);
  if (!stagedMerged && !hasStandaloneConfig) {
    // Use lint-staged config from package.json if available, otherwise use default
    const pkgData = readJsonFile(packageJsonPath) as {
      'lint-staged'?: Record<string, string | string[]>;
    };
    const stagedConfig = pkgData?.['lint-staged'] ?? DEFAULT_STAGED_CONFIG;
    const updated = rewriteScripts(JSON.stringify(stagedConfig), readRulesYaml());
    const finalConfig: Record<string, string | string[]> = updated
      ? JSON.parse(updated)
      : stagedConfig;
    stagedMerged = mergeStagedConfigToViteConfig(projectPath, finalConfig, silent, report);
  }

  // Only remove lint-staged key from package.json after staged config is
  // confirmed in vite.config.ts — prevents losing config on merge failure
  if (stagedMerged) {
    removeLintStagedFromPackageJson(packageJsonPath);
  }

  // Copy default .husky/ hooks to .vite-hooks/ before creating pre-commit hook.
  // Custom dirs (e.g. .config/husky) are kept in-place — no copy needed.
  if (oldHooksDir && !isCustomDir) {
    const oldDir = path.join(projectPath, oldHooksDir);
    if (fs.existsSync(oldDir)) {
      const targetDir = path.join(projectPath, hooksDir);
      fs.mkdirSync(targetDir, { recursive: true });
      for (const entry of fs.readdirSync(oldDir, { withFileTypes: true })) {
        if (entry.isDirectory() || entry.name.startsWith('.')) {
          continue;
        }
        const src = path.join(oldDir, entry.name);
        const dest = path.join(targetDir, entry.name);
        fs.copyFileSync(src, dest);
        fs.chmodSync(dest, 0o755);
      }
      // Remove old .husky/ directory after copying hooks to .vite-hooks/
      fs.rmSync(oldDir, { recursive: true, force: true });
    }
  }

  // Only create pre-commit hook if staged config was merged into vite.config.ts.
  // Standalone lint-staged config files are NOT sufficient — `vp staged` only
  // reads from vite.config.ts, so a hook without merged config would fail.
  if (stagedMerged) {
    createPreCommitHook(projectPath, hooksDir);
  }

  // vp config requires a git workspace — skip if no .git found
  if (!gitRoot) {
    removeReplacedHookPackages(packageJsonPath);
    return true;
  }

  // Clear husky's core.hooksPath so vp config can set the new one.
  // Only clear if it matches the old husky directory — preserve genuinely custom paths.
  if (oldHooksDir) {
    const checkResult = spawn.sync('git', ['config', '--local', 'core.hooksPath'], {
      cwd: projectPath,
      stdio: 'pipe',
    });
    const existingPath = checkResult.status === 0 ? checkResult.stdout?.toString().trim() : '';
    if (existingPath === `${oldHooksDir}/_` || existingPath === oldHooksDir) {
      spawn.sync('git', ['config', '--local', '--unset', 'core.hooksPath'], {
        cwd: projectPath,
        stdio: 'pipe',
      });
    }
  }

  const vpBin = process.env.VP_CLI_BIN ?? 'vp';

  // Install git hooks via vp config (--no-agent to skip agent setup, handled by migration)
  const configArgs = isCustomDir
    ? ['config', '--no-agent', '--hooks-dir', hooksDir]
    : ['config', '--no-agent'];
  const configResult = spawn.sync(vpBin, configArgs, {
    cwd: projectPath,
    stdio: 'pipe',
  });
  if (configResult.status === 0) {
    // vp config outputs skip/info messages to stdout via log().
    // An empty message means hooks were installed successfully;
    // any non-empty output indicates a skip (HUSKY=0, hooksPath
    // already set, .git not found, etc.).
    const stdout = configResult.stdout?.toString().trim() ?? '';
    if (stdout) {
      warnMigration(`Git hooks not configured — ${stdout}`, report);
      return false;
    }
    removeReplacedHookPackages(packageJsonPath);
    if (report) {
      report.gitHooksConfigured = true;
    }
    if (!silent) {
      prompts.log.success('✔ Git hooks configured');
    }
    return true;
  }
  warnMigration('Failed to install git hooks', report);
  return false;
}

/**
 * Check if a standalone lint-staged config file exists
 */
function hasStandaloneLintStagedConfig(projectPath: string): boolean {
  return LINT_STAGED_ALL_CONFIG_FILES.some((file) => fs.existsSync(path.join(projectPath, file)));
}

/**
 * Check if a standalone lint-staged config exists in a format that can't be
 * auto-migrated to "staged" in vite.config.ts (non-JSON files like .yaml,
 * .mjs, .cjs, .js, or a non-JSON .lintstagedrc).
 */
function hasUnsupportedLintStagedConfig(projectPath: string): boolean {
  for (const filename of LINT_STAGED_OTHER_CONFIG_FILES) {
    if (fs.existsSync(path.join(projectPath, filename))) {
      return true;
    }
  }
  const lintstagedrcPath = path.join(projectPath, '.lintstagedrc');
  if (fs.existsSync(lintstagedrcPath) && !isJsonFile(lintstagedrcPath)) {
    return true;
  }
  return false;
}

/**
 * Create pre-commit hook file in the hooks directory.
 */
// Lint-staged invocation patterns — replaced in-place with `vp staged`.
// The optional prefix group captures env var assignments like `NODE_OPTIONS=... `.
// We still detect old lint-staged patterns to migrate existing hooks.
const STALE_LINT_STAGED_PATTERNS = [
  /^((?:[A-Z_][A-Z0-9_]*(?:=\S*)?\s+)*)(pnpm|pnpm exec|npx|yarn|yarn run|npm exec|npm run|bunx|bun run|bun x)\s+lint-staged\b/,
  /^((?:[A-Z_][A-Z0-9_]*(?:=\S*)?\s+)*)lint-staged\b/,
];

const DEFAULT_STAGED_CONFIG: Record<string, string> = { '*': 'vp check --fix' };

/**
 * Ensure the pre-commit hook exists with `vp staged`, and that
 * vite.config.ts contains a `staged` block (using the default config
 * if none is present). Called by `vp config` after hook installation.
 */
export function ensurePreCommitHook(projectPath: string, dir = '.vite-hooks'): void {
  if (!hasStagedConfigInViteConfig(projectPath)) {
    mergeStagedConfigToViteConfig(projectPath, DEFAULT_STAGED_CONFIG, true);
  }
  createPreCommitHook(projectPath, dir);
}

export function createPreCommitHook(projectPath: string, dir = '.vite-hooks'): void {
  const huskyDir = path.join(projectPath, dir);
  fs.mkdirSync(huskyDir, { recursive: true });
  const hookPath = path.join(huskyDir, 'pre-commit');
  if (fs.existsSync(hookPath)) {
    const existing = fs.readFileSync(hookPath, 'utf8');
    if (existing.includes('vp staged')) {
      return; // already has vp staged
    }
    // Replace old lint-staged invocations in-place, preserve everything else
    const lines = existing.split('\n');
    let replaced = false;
    const result: string[] = [];
    for (const line of lines) {
      const trimmed = line.trim();
      if (!replaced) {
        let matched = false;
        for (const pattern of STALE_LINT_STAGED_PATTERNS) {
          const match = pattern.exec(trimmed);
          if (match) {
            // Preserve env var prefix (capture group 1) and flags/chained commands after lint-staged
            const envPrefix = match[1]?.trim() ?? '';
            const rest = trimmed.slice(match[0].length).trim();
            const parts = [envPrefix, 'vp staged', rest].filter(Boolean);
            result.push(parts.join(' '));
            replaced = true;
            matched = true;
            break;
          }
        }
        if (matched) {
          continue;
        }
      }
      result.push(line);
    }
    if (!replaced) {
      // No lint-staged line found — append after existing content
      fs.writeFileSync(hookPath, `${result.join('\n').trimEnd()}\nvp staged\n`);
    } else {
      fs.writeFileSync(hookPath, result.join('\n'));
    }
  } else {
    fs.writeFileSync(hookPath, 'vp staged\n');
    fs.chmodSync(hookPath, 0o755);
  }
}

/**
 * Rewrite only `scripts.prepare` in the root package.json using vite-prepare.yml rules.
 * Collapses "husky install" → "husky" before applying ast-grep so that the
 * replace-husky rule produces "vp config" with any directory argument preserved.
 * Returns the old husky hooks dir (if any) for migration to .vite-hooks.
 * Called only when hooks are being set up (not with --no-hooks).
 */
export function rewritePrepareScript(rootDir: string): string | undefined {
  const packageJsonPath = path.join(rootDir, 'package.json');
  if (!fs.existsSync(packageJsonPath)) {
    return undefined;
  }

  let oldDir: string | undefined;

  editJsonFile<{ scripts?: Record<string, string> }>(packageJsonPath, (pkg) => {
    if (!pkg.scripts?.prepare) {
      return pkg;
    }

    // Collapse "husky install" → "husky" so the ast-grep rule
    // produces "vp config" with any directory argument preserved.
    const prepare = collapseHuskyInstall(pkg.scripts.prepare);

    const prepareJson = JSON.stringify({ prepare });
    const updated = rewriteScripts(prepareJson, readPrepareRulesYaml());
    if (updated) {
      let newPrepare: string = JSON.parse(updated).prepare;
      newPrepare = newPrepare.replace(
        /\bvp config(?:\s+(?!-)([\w./-]+))?/,
        (_match: string, dir: string | undefined) => {
          // Capture the old husky dir for hook migration.
          // Default husky dir is .husky; custom dirs keep --hooks-dir flag.
          oldDir = dir ?? '.husky';
          return dir ? `vp config --hooks-dir ${dir}` : 'vp config';
        },
      );
      pkg.scripts.prepare = newPrepare;
    } else if (prepare !== pkg.scripts.prepare) {
      // Pre-processing changed the script (husky install → husky)
      // but no rule matched — keep the collapsed form
      pkg.scripts.prepare = prepare;
    }
    return pkg;
  });

  return oldDir;
}

export function setPackageManager(
  projectDir: string,
  downloadPackageManager: DownloadPackageManagerResult,
) {
  // Set the package manager pin. Compatibility-first rule (rfcs/dev-engines.md):
  // an existing `packageManager` field or `devEngines.packageManager` declaration
  // is the source of truth and is left as-is; otherwise the exact resolved version
  // is written to `devEngines.packageManager` (the recommended standard field).
  editJsonFile<{
    packageManager?: string;
    devEngines?: { packageManager?: unknown; [key: string]: unknown };
  }>(path.join(projectDir, 'package.json'), (pkg) => {
    if (!pkg.packageManager && !pkg.devEngines?.packageManager) {
      // Only spread a well-formed object: spreading a malformed devEngines value
      // (string/array) would corrupt the field with numeric index keys
      const devEngines =
        typeof pkg.devEngines === 'object' &&
        pkg.devEngines !== null &&
        !Array.isArray(pkg.devEngines)
          ? pkg.devEngines
          : undefined;
      pkg.devEngines = {
        ...devEngines,
        packageManager: {
          name: downloadPackageManager.name,
          version: downloadPackageManager.version,
          onFail: 'download',
        },
      };
    }
    return pkg;
  });
}

export type NodeVersionManagerDetection =
  | { file: '.nvmrc'; voltaPresent?: true }
  | { file: 'package.json'; voltaNodeVersion: string };

/**
 * Detect a .nvmrc file in the project directory.
 * If not found, check for a Volta node version in package.json.
 * If either is found, return the relevant info for migration.
 * Returns undefined if not found or .node-version already exists.
 */
export function detectNodeVersionManagerFile(
  projectPath: string,
): NodeVersionManagerDetection | undefined {
  // already has .node-version — skip detection to avoid false positives and preserve existing file
  if (fs.existsSync(path.join(projectPath, '.node-version'))) {
    return undefined;
  }

  const configs = detectConfigs(projectPath);

  // .nvmrc takes priority over volta.node when both are present.
  // voltaPresent is carried through so the migration step can remind the user
  // to remove the now-redundant volta field from package.json.
  if (configs.nvmrcFile) {
    return configs.voltaNode ? { file: '.nvmrc', voltaPresent: true } : { file: '.nvmrc' };
  }

  if (configs.voltaNode) {
    return { file: 'package.json', voltaNodeVersion: configs.voltaNode };
  }

  return undefined;
}

/**
 * Parse a version alias from a .nvmrc file into a .node-version compatible string.
 * Accepts the first line of .nvmrc (pre-trimmed).
 * Returns null for unsupported aliases like "system", "default", "iojs".
 */
export function parseNvmrcVersion(alias: string): string | null {
  const version = alias.trim();

  if (!version) {
    return null;
  }

  // "node" and "stable" mean "latest stable release" which maps closely to lts/*.
  // Starting from Node 27, all releases will be LTS, so the gap is shrinking.
  // We map these to lts/* and log the conversion so users are aware.
  if (version === 'node' || version === 'stable') {
    return 'lts/*';
  }

  // "iojs", "system", and "default" have no meaningful equivalent and cannot be auto-migrated.
  if (version === 'iojs' || version === 'system' || version === 'default') {
    return null;
  }

  // LTS aliases (lts/*, lts/iron, etc.) pass through as-is
  if (version.startsWith('lts/')) {
    return version;
  }

  // Strip optional 'v' prefix, then validate as a semver version or range
  const normalized = version.startsWith('v') ? version.slice(1) : version;
  if (!normalized || !semver.validRange(normalized)) {
    return null;
  }
  return normalized;
}

/**
 * Migrate .nvmrc or Volta node version from package.json to .node-version.
 * - For .nvmrc: the source file is removed after migration.
 * - For package.json (Volta): the volta field is left as-is; removal is left to the user's discretion.
 * Returns true on success, false if migration was skipped or failed.
 */
export function migrateNodeVersionManagerFile(
  projectPath: string,
  detection: NodeVersionManagerDetection,
  report?: MigrationReport,
): boolean {
  const nodeVersionPath = path.join(projectPath, '.node-version');

  // Volta: node version was already extracted during detection — no package.json re-read needed
  if (detection.file === 'package.json') {
    const { voltaNodeVersion } = detection;

    // Normalize Volta's "lts" alias to the .node-version compatible form
    const resolvedVersion = voltaNodeVersion === 'lts' ? 'lts/*' : voltaNodeVersion;

    if (!semver.valid(resolvedVersion) && resolvedVersion !== 'lts/*') {
      warnMigration(
        `package.json volta.node "${voltaNodeVersion}" is not an exact version. Pin an exact version (e.g. ${voltaNodeVersion}.0 or run \`volta pin node@${voltaNodeVersion}\`) then re-run migration.`,
        report,
      );
      return false;
    }

    fs.writeFileSync(nodeVersionPath, `${resolvedVersion}\n`);
    if (report) {
      report.manualSteps.push('Remove the "volta" field from package.json');
      report.nodeVersionFileMigrated = true;
    } else {
      prompts.log.info('You can now remove the "volta" field from package.json manually.');
    }
    return true;
  }

  // .nvmrc: parse version alias and write to .node-version
  const sourcePath = path.join(projectPath, '.nvmrc');
  const content = fs.readFileSync(sourcePath, 'utf8');
  const originalAlias = content.split('\n')[0]?.trim() ?? '';
  const version = parseNvmrcVersion(originalAlias);

  if (!version) {
    warnMigration(
      '.nvmrc contains an unsupported version alias. Create .node-version manually with your desired Node.js version.',
      report,
    );
    return false;
  }

  // TODO: remove this log once Node 27+ makes all releases LTS, at which point
  // "node"/"stable" and "lts/*" will be effectively equivalent.
  if (version === 'lts/*' && (originalAlias === 'node' || originalAlias === 'stable')) {
    prompts.log.info(
      `"${originalAlias}" in .nvmrc is not a specific version; automatically mapping to "lts/*"`,
    );
  }

  fs.writeFileSync(nodeVersionPath, `${version}\n`);
  fs.unlinkSync(sourcePath);

  if (report) {
    report.nodeVersionFileMigrated = true;
    // Both .nvmrc and volta were present; .nvmrc was migrated but volta still lingers.
    if (detection.voltaPresent) {
      report.manualSteps.push('Remove the "volta" field from package.json');
    }
  } else if (detection.voltaPresent) {
    prompts.log.info('You can now remove the "volta" field from package.json manually.');
  }
  return true;
}

export function warnPackageLevelEslint() {
  prompts.log.warn(
    'ESLint detected in workspace packages but no root config found. Package-level ESLint must be migrated manually.',
  );
}

// Framework-ESLint integration packages we can't migrate cleanly today.
// When any of these is present, the ESLint migration is skipped entirely
// — the user's ESLint setup stays intact and they get told how to proceed
// manually.
//
// `@nuxt/eslint` is a Nuxt module that loads ESLint at runtime via the
// dev server and writes a generated config to `.nuxt/eslint.config.mjs`,
// which the user's `eslint.config.mjs` re-exports. Migrating it
// produces a broken state: `vite.config.ts` references `@nuxt/eslint-plugin`
// (no longer installed) and `nuxt.config.ts` still tries to load the
// removed module. Track at https://github.com/voidzero-dev/vite-plus/issues
// once an issue exists.
const INCOMPATIBLE_ESLINT_INTEGRATIONS = ['@nuxt/eslint'] as const;

/**
 * Detect framework-ESLint integration packages whose ESLint migration is
 * known to be incompatible. Returns the offending package name, or
 * `undefined` if none is present.
 */
export function detectIncompatibleEslintIntegration(
  projectPath: string,
  packages?: WorkspacePackage[],
): string | undefined {
  const candidates = [projectPath, ...(packages ?? []).map((p) => path.join(projectPath, p.path))];
  for (const candidate of candidates) {
    const pkgJsonPath = path.join(candidate, 'package.json');
    if (!fs.existsSync(pkgJsonPath)) {
      continue;
    }
    let pkg: { devDependencies?: Record<string, string>; dependencies?: Record<string, string> };
    try {
      pkg = readJsonFile(pkgJsonPath) as typeof pkg;
    } catch {
      continue;
    }
    for (const name of INCOMPATIBLE_ESLINT_INTEGRATIONS) {
      if (pkg.devDependencies?.[name] || pkg.dependencies?.[name]) {
        return name;
      }
    }
  }
  return undefined;
}

export function warnIncompatibleEslintIntegration(name: string): void {
  prompts.log.warn(
    `${name} detected — automatic ESLint migration is skipped. ` +
      `${name} wires ESLint into a framework-specific flow that Vite+ cannot migrate cleanly yet. ` +
      'Your ESLint setup is preserved. ' +
      `To migrate manually, remove ${name} from package.json and re-run \`vp migrate\`.`,
  );
}

export function warnLegacyEslintConfig(legacyConfigFile: string) {
  prompts.log.warn(
    `Legacy ESLint configuration detected (${legacyConfigFile}). ` +
      'Automatic migration to Oxlint requires ESLint v9+ with flat config format (eslint.config.*). ' +
      'Please upgrade to ESLint v9 first: https://eslint.org/docs/latest/use/migrate-to-9.0.0',
  );
}

export async function confirmEslintMigration(interactive: boolean): Promise<boolean> {
  if (interactive) {
    const confirmed = await prompts.confirm({
      message:
        'Migrate ESLint rules to Oxlint using @oxlint/migrate?\n  ' +
        styleText(
          'gray',
          "Oxlint is Vite+'s built-in linter — significantly faster than ESLint with compatible rule support. @oxlint/migrate converts your existing rules automatically.",
        ),
      initialValue: true,
    });
    if (prompts.isCancel(confirmed)) {
      cancelAndExit();
    }
    return confirmed;
  }
  return true;
}

export async function promptEslintMigration(
  projectPath: string,
  interactive: boolean,
  packages?: WorkspacePackage[],
): Promise<boolean> {
  const incompatible = detectIncompatibleEslintIntegration(projectPath, packages);
  if (incompatible) {
    warnIncompatibleEslintIntegration(incompatible);
    return false;
  }
  const eslintProject = detectEslintProject(projectPath, packages);
  if (eslintProject.hasDependency && !eslintProject.configFile && eslintProject.legacyConfigFile) {
    warnLegacyEslintConfig(eslintProject.legacyConfigFile);
    return false;
  }
  if (!eslintProject.hasDependency) {
    return false;
  }
  if (!eslintProject.configFile) {
    // Packages have eslint but no root config → warn and skip
    warnPackageLevelEslint();
    return false;
  }
  const confirmed = await confirmEslintMigration(interactive);
  if (!confirmed) {
    return false;
  }
  const ok = await migrateEslintToOxlint(
    projectPath,
    interactive,
    eslintProject.configFile,
    packages,
  );
  if (!ok) {
    cancelAndExit('ESLint migration failed.', 1);
  }
  return true;
}

export function warnPackageLevelPrettier() {
  prompts.log.warn(
    'Prettier detected in workspace packages but no root config found. Package-level Prettier must be migrated manually.',
  );
}

export async function confirmPrettierMigration(interactive: boolean): Promise<boolean> {
  if (interactive) {
    const confirmed = await prompts.confirm({
      message:
        'Migrate Prettier to Oxfmt?\n  ' +
        styleText(
          'gray',
          "Oxfmt is Vite+'s built-in formatter that replaces Prettier with faster performance. Your configuration will be converted automatically.",
        ),
      initialValue: true,
    });
    if (prompts.isCancel(confirmed)) {
      cancelAndExit();
    }
    return confirmed;
  }
  prompts.log.info('Prettier configuration detected. Auto-migrating to Oxfmt...');
  return true;
}

export async function promptPrettierMigration(
  projectPath: string,
  interactive: boolean,
  packages?: WorkspacePackage[],
): Promise<boolean> {
  const prettierProject = detectPrettierProject(projectPath, packages);
  if (!prettierProject.hasDependency) {
    return false;
  }
  if (!prettierProject.configFile) {
    // Packages have prettier but no root config → warn and skip
    warnPackageLevelPrettier();
    return false;
  }
  const confirmed = await confirmPrettierMigration(interactive);
  if (!confirmed) {
    return false;
  }
  const ok = await migratePrettierToOxfmt(
    projectPath,
    interactive,
    prettierProject.configFile,
    packages,
  );
  if (!ok) {
    cancelAndExit('Prettier migration failed.', 1);
  }
  return true;
}
