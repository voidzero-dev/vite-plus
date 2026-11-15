/**
 * Version scheme of `@voidzero-dev/vite-plus-core` (rfcs/core-version-alignment.md).
 *
 * The core version encodes the bundled Vite version, so package managers check
 * `vite` peer ranges against the alias `vite: npm:@voidzero-dev/vite-plus-core@...`
 * correctly:
 *
 *   stable Vite+ release:          <vite major>.<vite minor>.<vite patch * 1000 + n>
 *   Vite+ prerelease, stable Vite: <next stable core version>-vp.<k>
 *   Vite+ prerelease, Vite pre:    <vite version>.vp.<k>
 *
 * `n` counts the stable Vite+ releases on one Vite version (0 to 998). `k`
 * counts the Vite+ prereleases on one base (0 to 998). The counter value 999
 * is reserved for local and CI builds, so they never collide with a published
 * version: `<vite patch * 1000 + 999>` on a stable Vite, `<vite version>.vp.999`
 * on a Vite prerelease.
 *
 * A release records its `vite-plus` version in the core manifest
 * (`vitePlusVersion`). The release check uses it to tell a retry of the same
 * release from a core version that another release already published.
 *
 * Usage:
 *   node packages/tools/src/core-version.ts prepare --vite-plus <version>
 *   node packages/tools/src/core-version.ts local
 *   node packages/tools/src/core-version.ts check [--release]
 */

import { readFileSync, writeFileSync } from 'node:fs';
import { parseArgs } from 'node:util';

export const CORE_PACKAGE_NAME = '@voidzero-dev/vite-plus-core';
/** The `vite` alias to the core: `npm:@voidzero-dev/vite-plus-core@<version>`. */
export const CORE_ALIAS_PREFIX = `npm:${CORE_PACKAGE_NAME}@`;
// Same convention as oxlint-tsgolint, which Vite+ bundles: `7.0.2000` is its
// first release on TypeScript `7.0.2`, and `7.0.2001` the next one
// (https://github.com/oxc-project/tsgolint#versioning).
const PATCH_FACTOR = 1000;
const MAX_RELEASE_COUNTER = 998;
const LOCAL_BUILD_COUNTER = 999;

const PRERELEASE_TAG = 'vp';
const VERSION_RE =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*))?$/;

interface ParsedVersion {
  major: number;
  minor: number;
  patch: number;
  prerelease: string[];
}

export interface DecodedCoreVersion {
  /** The bundled Vite version. */
  vite: string;
  /** The stable release counter. Undefined on a Vite prerelease. */
  n?: number;
  /** The Vite+ prerelease counter. Undefined on a stable core version. */
  k?: number;
}

function parseVersion(version: string): ParsedVersion {
  const match = VERSION_RE.exec(version);
  if (!match) {
    throw new Error(`Invalid version: ${version}`);
  }
  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
    prerelease: match[4] ? match[4].split('.') : [],
  };
}

function isPrereleaseVersion(version: string): boolean {
  return parseVersion(version).prerelease.length > 0;
}

/** Returns the stable core version for release counter `n` on a stable Vite. */
export function encodeCoreVersion(vite: string, n: number): string {
  const { major, minor, patch, prerelease } = parseVersion(vite);
  if (prerelease.length > 0) {
    throw new Error(`A stable core version cannot encode the Vite prerelease ${vite}`);
  }
  if (!Number.isInteger(n) || n < 0 || n > LOCAL_BUILD_COUNTER) {
    throw new Error(`The release counter must be an integer from 0 to ${LOCAL_BUILD_COUNTER}`);
  }
  return `${major}.${minor}.${patch * PATCH_FACTOR + n}`;
}

/**
 * Returns the first core version for a bundled Vite version: counter 0 on a
 * stable Vite, `<vite>.vp.0` on a Vite prerelease.
 */
export function initialCoreVersion(vite: string): string {
  return isPrereleaseVersion(vite) ? `${vite}.${PRERELEASE_TAG}.0` : encodeCoreVersion(vite, 0);
}

/** Decodes a core version into the bundled Vite version and its counters. */
export function decodeCoreVersion(core: string): DecodedCoreVersion {
  const { major, minor, patch, prerelease } = parseVersion(core);
  const stableVite = `${major}.${minor}.${Math.floor(patch / PATCH_FACTOR)}`;
  if (prerelease.length === 0) {
    return { vite: stableVite, n: patch % PATCH_FACTOR };
  }
  const k = prerelease.at(-1)!;
  if (prerelease.at(-2) !== PRERELEASE_TAG || !/^(?:0|[1-9]\d*)$/.test(k)) {
    throw new Error(`${core} is not an encoded core version`);
  }
  if (prerelease.length === 2) {
    return { vite: stableVite, n: patch % PATCH_FACTOR, k: Number(k) };
  }
  return {
    vite: `${major}.${minor}.${patch}-${prerelease.slice(0, -2).join('.')}`,
    k: Number(k),
  };
}

export function tryDecodeCoreVersion(core: string): DecodedCoreVersion | undefined {
  try {
    return decodeCoreVersion(core);
  } catch {
    return undefined;
  }
}

// Returns the next release counter. The local build counter is ignored, and a
// counter above the release maximum fails.
function nextCounter(values: Iterable<number>, what: string): number {
  let next = 0;
  for (const value of values) {
    if (value <= MAX_RELEASE_COUNTER) {
      next = Math.max(next, value + 1);
    }
  }
  if (next > MAX_RELEASE_COUNTER) {
    throw new Error(`${what} already has ${MAX_RELEASE_COUNTER + 1} core releases, the maximum`);
  }
  return next;
}

/**
 * Returns the reserved core version for a local or CI build: counter 999 on a
 * stable Vite, `<vite>.vp.999` on a Vite prerelease. Releases never use it.
 */
export function localCoreVersion(vite: string): string {
  return isPrereleaseVersion(vite)
    ? `${vite}.${PRERELEASE_TAG}.${LOCAL_BUILD_COUNTER}`
    : encodeCoreVersion(vite, LOCAL_BUILD_COUNTER);
}

/**
 * Returns the core version for a release.
 *
 * @param vite - The bundled Vite version.
 * @param vitePlus - The `vite-plus` version of the release.
 * @param published - Every published core version. Versions from before the
 *   alignment (the `0.x` and `1.x` lines) never match a bundled Vite version,
 *   so they do not affect the counters.
 */
export function nextCoreVersion({
  vite,
  vitePlus,
  published,
}: {
  vite: string;
  vitePlus: string;
  published: readonly string[];
}): string {
  const isVitePlusPrerelease = isPrereleaseVersion(vitePlus);
  const decoded = published.flatMap((version) => {
    const result = tryDecodeCoreVersion(version);
    return result ? [result] : [];
  });

  if (isPrereleaseVersion(vite)) {
    if (!isVitePlusPrerelease) {
      throw new Error(`The stable vite-plus ${vitePlus} cannot bundle the Vite prerelease ${vite}`);
    }
    const k = nextCounter(
      decoded.flatMap((entry) => (entry.vite === vite && entry.k !== undefined ? [entry.k] : [])),
      `Vite ${vite}`,
    );
    return `${vite}.${PRERELEASE_TAG}.${k}`;
  }

  const n = nextCounter(
    decoded.flatMap((entry) =>
      entry.vite === vite && entry.k === undefined && entry.n !== undefined ? [entry.n] : [],
    ),
    `Vite ${vite}`,
  );
  const base = encodeCoreVersion(vite, n);
  if (!isVitePlusPrerelease) {
    return base;
  }
  const k = nextCounter(
    decoded.flatMap((entry) =>
      entry.vite === vite && entry.n === n && entry.k !== undefined ? [entry.k] : [],
    ),
    `Core ${base}`,
  );
  return `${base}-${PRERELEASE_TAG}.${k}`;
}

interface CoreManifest {
  version: string;
  bundledVersions?: { vite?: string };
  vitePlusVersion?: string;
}

/**
 * Asserts that a core manifest encodes its bundled Vite version. With
 * `release`, also asserts that:
 * - the core version is a prerelease exactly when the `vite-plus` version is
 *   one. Between releases, main can bundle a Vite prerelease with a stable
 *   `vite-plus`.
 * - the core version does not use the local build counter.
 * - the manifest records this `vite-plus` version. `prepare` writes it.
 */
export function assertCoreVersionMatchesVite(
  pkg: CoreManifest,
  vitePlus: string,
  { release = false }: { release?: boolean } = {},
): void {
  const vite = pkg.bundledVersions?.vite;
  if (!vite) {
    throw new Error(`${CORE_PACKAGE_NAME} has no bundledVersions.vite`);
  }
  const decoded = decodeCoreVersion(pkg.version);
  if (decoded.vite !== vite) {
    throw new Error(
      `${CORE_PACKAGE_NAME}@${pkg.version} encodes Vite ${decoded.vite}, but it bundles Vite ${vite}. ` +
        `Set its version to ${initialCoreVersion(vite)}.`,
    );
  }
  if (!release) {
    return;
  }
  const isCorePrerelease = decoded.k !== undefined;
  if (isCorePrerelease !== isPrereleaseVersion(vitePlus)) {
    throw new Error(
      `${CORE_PACKAGE_NAME}@${pkg.version} and vite-plus@${vitePlus} must both be prereleases or both be stable`,
    );
  }
  if (decoded.n === LOCAL_BUILD_COUNTER || decoded.k === LOCAL_BUILD_COUNTER) {
    throw new Error(
      `${CORE_PACKAGE_NAME}@${pkg.version} uses the counter ${LOCAL_BUILD_COUNTER}, which is reserved for local builds`,
    );
  }
  if (pkg.vitePlusVersion !== vitePlus) {
    throw new Error(
      `${CORE_PACKAGE_NAME}@${pkg.version} records vite-plus ${pkg.vitePlusVersion ?? '(none)'}, but the release is vite-plus ${vitePlus}. ` +
        'Run the Prepare Release workflow to select the core version.',
    );
  }
}

/**
 * Asserts that npm still accepts the core version for this release. It fails
 * when another release published the version, and when the version was
 * unpublished: npm never accepts that number again, and the publish script
 * would take npm's rejection for an earlier upload. A retry of the same
 * release passes.
 */
export function assertCoreVersionUnclaimed(
  packument: CorePackument,
  core: string,
  vitePlus: string,
): void {
  const published = packument.versions?.[core];
  const rerun = 'Run the Prepare Release workflow again to select an unused core version.';
  if (published) {
    if (published.vitePlusVersion !== vitePlus) {
      throw new Error(
        `${CORE_PACKAGE_NAME}@${core} is already published for vite-plus ${published.vitePlusVersion ?? '(unknown)'}. ${rerun}`,
      );
    }
    return;
  }
  if (usedVersionNumbers(packument).includes(core)) {
    throw new Error(
      `${CORE_PACKAGE_NAME}@${core} was unpublished, and npm does not accept it again. ${rerun}`,
    );
  }
}

const CORE_MANIFEST_URL = new URL('../../core/package.json', import.meta.url);

function readManifest(url: URL): CoreManifest {
  return JSON.parse(readFileSync(url, 'utf8'));
}

const DEFAULT_NPM_REGISTRY = 'https://registry.npmjs.org';

export interface CorePackument {
  versions?: Record<string, { vitePlusVersion?: string }>;
  time?: Record<string, unknown> & { unpublished?: { versions?: unknown } };
}

/**
 * Returns every version number that npm has used for the package. npm never
 * accepts a version number again after an unpublish, but it removes that
 * version from `versions` and keeps it in `time` (or in `time.unpublished`
 * when every version is unpublished).
 */
export function usedVersionNumbers(packument: CorePackument): string[] {
  const used = new Set(Object.keys(packument.versions ?? {}));
  for (const key of Object.keys(packument.time ?? {})) {
    if (key !== 'created' && key !== 'modified' && key !== 'unpublished') {
      used.add(key);
    }
  }
  const unpublished = packument.time?.unpublished?.versions;
  if (Array.isArray(unpublished)) {
    for (const version of unpublished) {
      if (typeof version === 'string') {
        used.add(version);
      }
    }
  }
  return [...used];
}

/**
 * Reads the full packument of the core package. `prepare` uses the version
 * numbers, and `check --release` uses the version numbers and the
 * `vitePlusVersion` field of one manifest. Returns an empty packument when the
 * package is not published.
 */
async function readCorePackument(): Promise<CorePackument> {
  const registry = (process.env.PUBLISH_REGISTRY ?? DEFAULT_NPM_REGISTRY).replace(/\/+$/, '');
  const url = `${registry}/${CORE_PACKAGE_NAME.replace('/', '%2f')}`;
  const response = await fetch(url, {
    headers: { accept: 'application/json' },
    signal: AbortSignal.timeout(60_000),
  });
  if (response.status === 404) {
    return {};
  }
  if (!response.ok) {
    throw new Error(`Registry returned HTTP ${response.status} for ${url}`);
  }
  return (await response.json()) as CorePackument;
}

async function main(argv: string[]): Promise<void> {
  const { positionals, values } = parseArgs({
    args: argv,
    allowPositionals: true,
    options: {
      'vite-plus': { type: 'string' },
      release: { type: 'boolean', default: false },
    },
  });
  const core = readManifest(CORE_MANIFEST_URL);
  const vite = core.bundledVersions?.vite;
  if (!vite) {
    throw new Error(`packages/core/package.json has no bundledVersions.vite`);
  }

  switch (positionals[0]) {
    case 'prepare': {
      const vitePlus = values['vite-plus'];
      if (!vitePlus) {
        throw new Error('prepare requires --vite-plus <version>');
      }
      core.version = nextCoreVersion({
        vite,
        vitePlus,
        published: usedVersionNumbers(await readCorePackument()),
      });
      core.vitePlusVersion = vitePlus;
      writeFileSync(CORE_MANIFEST_URL, `${JSON.stringify(core, null, 2)}\n`);
      console.log(core.version);
      break;
    }
    case 'local':
      console.log(localCoreVersion(vite));
      break;
    case 'check': {
      const cli = readManifest(new URL('../../cli/package.json', import.meta.url));
      assertCoreVersionMatchesVite(core, cli.version, { release: values.release });
      if (values.release) {
        assertCoreVersionUnclaimed(await readCorePackument(), core.version, cli.version);
      }
      console.log(`${CORE_PACKAGE_NAME}@${core.version} encodes Vite ${vite}`);
      break;
    }
    default:
      throw new Error(`Unknown command: ${positionals[0] ?? ''}. Use prepare, local, or check.`);
  }
}

if (import.meta.main) {
  try {
    await main(process.argv.slice(2));
  } catch (error) {
    console.error(`[core-version] ${(error as Error).message}`);
    process.exit(1);
  }
}
