import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import semver from 'semver';
import { Scalar, YAMLSeq } from 'yaml';

import { type WorkspacePackage } from '../../types/index.ts';
import {
  VITEST_AGE_GATE_EXEMPT_PACKAGES,
  VITE_PLUS_NAME,
  VITE_PLUS_VERSION,
} from '../../utils/constants.ts';
import { readJsonFile } from '../../utils/json.ts';
import { editYamlFile, readYamlFile, scalarString } from '../../utils/yaml.ts';
import {
  createCatalogDependencyResolverFromCatalogs,
  overridesSatisfyVitePlus,
  rewriteCatalog,
  usesWebdriverioProvider,
} from '../migrator.ts';
import { type MigrationReport } from '../report.ts';
import {
  WEBDRIVERIO_PROVIDER,
  readPackageJsonIfExists,
  warnMigration,
  type DependencyBag,
} from './shared.ts';

// Webdriverio is the runtime peer that drags `edgedriver` / `geckodriver` in.
const WEBDRIVERIO_PEER_DEP = 'webdriverio';

// Dependencies whose presence before migration signals the user will end up
// with webdriverio after migration. `@vitest/browser-webdriverio` is the opt-in
// provider vite-plus keeps in the user's deps (pinned to the bundled vitest)
// and `webdriverio` is its runtime peer (added via `BROWSER_PROVIDER_PEER_DEPS`);
// either one means the edgedriver/geckodriver postinstalls must be allowed.
const WEBDRIVERIO_ALLOW_SIGNAL_DEPS = [WEBDRIVERIO_PEER_DEP, WEBDRIVERIO_PROVIDER] as const;

export function hasOwnWebdriverioDependency(pkg: DependencyBag): boolean {
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

export function workspaceUsesWebdriverio(
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

export interface YarnPnpDetection {
  source: 'environment' | 'configuration' | 'default';
}

/**
 * Detect Yarn Plug'n'Play using the same precedence Yarn applies to
 * `nodeLinker`. Yarn 2+ defaults to PnP when no value is configured, while
 * Yarn Classic defaults to node_modules. Unknown/`latest` Yarn versions are
 * treated as modern because that is the version `vp` will provision.
 */
export function detectYarnPnpMode(
  projectPath: string,
  yarnVersion: string,
): YarnPnpDetection | undefined {
  const coercedVersion = semver.coerce(yarnVersion);
  if (coercedVersion?.major === 1) {
    return undefined;
  }

  const environmentLinker = process.env.YARN_NODE_LINKER?.trim();
  if (environmentLinker) {
    return environmentLinker.toLowerCase() === 'pnp' ? { source: 'environment' } : undefined;
  }

  const configuredLinker = resolveEffectiveYarnConfigValue(
    projectPath,
    'nodeLinker',
    'YARN_NODE_LINKER',
  );
  if (configuredLinker) {
    return configuredLinker.toLowerCase() === 'pnp' ? { source: 'configuration' } : undefined;
  }

  return { source: 'default' };
}

/** Set the project-local Yarn linker while preserving every other rc setting. */
export function configureYarnNodeModulesMode(projectPath: string): boolean {
  const yarnrcYmlPath = path.join(projectPath, '.yarnrc.yml');
  const before = fs.existsSync(yarnrcYmlPath) ? fs.readFileSync(yarnrcYmlPath, 'utf8') : undefined;
  if (before === undefined) {
    fs.writeFileSync(yarnrcYmlPath, '');
  }
  editYamlFile(yarnrcYmlPath, (doc) => {
    doc.set('nodeLinker', 'node-modules');
  });
  return before !== fs.readFileSync(yarnrcYmlPath, 'utf8');
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
export function findYarnWorkspaceHoisting(
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
export function applyYarnWorkspaceHoistingFix(
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

export function rewriteYarnrcYml(
  projectPath: string,
  usesVitest: boolean,
  vitestEcosystemPackages: ReadonlySet<string>,
  catalogAdditions: ReadonlySet<string> = new Set(),
): void {
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
    rewriteCatalog(doc, usesVitest, vitestEcosystemPackages, catalogAdditions);
  });
}

export function yarnrcSatisfiesVitePlus(projectPath: string, usesVitest: boolean): boolean {
  const yarnrcYmlPath = path.join(projectPath, '.yarnrc.yml');
  if (!fs.existsSync(yarnrcYmlPath)) {
    return false;
  }
  const doc = readYamlFile(yarnrcYmlPath) as {
    nodeLinker?: string;
    catalog?: Record<string, string>;
    catalogs?: Record<string, Record<string, string>>;
  } | null;
  const resolver = createCatalogDependencyResolverFromCatalogs(doc?.catalog, doc?.catalogs);
  const catalogName = resolver.preferredCatalogSpec.slice('catalog:'.length);
  const managedCatalog =
    catalogName && catalogName !== 'default'
      ? doc?.catalogs?.[catalogName]
      : (doc?.catalog ?? doc?.catalogs?.default);
  return (
    !!doc &&
    Object.hasOwn(doc, 'nodeLinker') &&
    overridesSatisfyVitePlus(managedCatalog, usesVitest) &&
    (VITE_PLUS_VERSION.startsWith('file:') ||
      resolver(resolver.preferredCatalogSpec, VITE_PLUS_NAME) === VITE_PLUS_VERSION)
  );
}
