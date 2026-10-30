/**
 * Version scheme of `@voidzero-dev/vite-plus-core` (rfcs/core-version-alignment.md).
 *
 * The core version encodes the bundled Vite version, so package managers check
 * `vite` peer ranges against the alias `vite: npm:@voidzero-dev/vite-plus-core@...`
 * correctly:
 *
 *   stable Vite+ release:          <vite major>.<vite minor>.<vite patch * 100 + n>
 *   Vite+ prerelease, stable Vite: <next stable core version>-vp.<k>
 *   Vite+ prerelease, Vite pre:    <vite version>.vp.<k>
 *
 * `n` counts the stable Vite+ releases on one Vite version (0 to 98). `n = 99`
 * is reserved for local and CI builds, so they never collide with a published
 * version. `k` counts the Vite+ prereleases on one base.
 *
 * Usage:
 *   node packages/tools/src/core-version.ts next --vite-plus <version>
 *   node packages/tools/src/core-version.ts local
 *   node packages/tools/src/core-version.ts check [--release]
 */

import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { parseArgs } from 'node:util';

export const CORE_PACKAGE_NAME = '@voidzero-dev/vite-plus-core';
export const MAX_RELEASE_COUNTER = 98;
export const LOCAL_BUILD_COUNTER = 99;

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

export function isPrereleaseVersion(version: string): boolean {
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
  return `${major}.${minor}.${patch * 100 + n}`;
}

/** Decodes a core version into the bundled Vite version and its counters. */
export function decodeCoreVersion(core: string): DecodedCoreVersion {
  const { major, minor, patch, prerelease } = parseVersion(core);
  const stableVite = `${major}.${minor}.${Math.floor(patch / 100)}`;
  if (prerelease.length === 0) {
    return { vite: stableVite, n: patch % 100 };
  }
  const k = prerelease.at(-1)!;
  if (prerelease.at(-2) !== PRERELEASE_TAG || !/^(?:0|[1-9]\d*)$/.test(k)) {
    throw new Error(`${core} is not an encoded core version`);
  }
  if (prerelease.length === 2) {
    return { vite: stableVite, n: patch % 100, k: Number(k) };
  }
  return {
    vite: `${major}.${minor}.${patch}-${prerelease.slice(0, -2).join('.')}`,
    k: Number(k),
  };
}

function tryDecodeCoreVersion(core: string): DecodedCoreVersion | undefined {
  try {
    return decodeCoreVersion(core);
  } catch {
    return undefined;
  }
}

function nextCounter(values: Iterable<number>): number {
  let next = 0;
  for (const value of values) {
    next = Math.max(next, value + 1);
  }
  return next;
}

/**
 * Returns the core version for a release.
 *
 * @param vite - The bundled Vite version.
 * @param vitePlus - The `vite-plus` version of the release.
 * @param published - Every published core version. Versions that do not decode
 *   (the `0.x` and `1.x` lines) are ignored.
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
    );
    return `${vite}.${PRERELEASE_TAG}.${k}`;
  }

  const n = nextCounter(
    decoded.flatMap((entry) =>
      entry.vite === vite &&
      entry.k === undefined &&
      entry.n !== undefined &&
      entry.n <= MAX_RELEASE_COUNTER
        ? [entry.n]
        : [],
    ),
  );
  if (n > MAX_RELEASE_COUNTER) {
    throw new Error(
      `Vite ${vite} already has ${MAX_RELEASE_COUNTER + 1} core releases, the maximum`,
    );
  }
  const base = encodeCoreVersion(vite, n);
  if (!isVitePlusPrerelease) {
    return base;
  }
  const k = nextCounter(
    published.flatMap((version) => {
      const match = version.startsWith(`${base}-${PRERELEASE_TAG}.`)
        ? tryDecodeCoreVersion(version)
        : undefined;
      return match?.k === undefined ? [] : [match.k];
    }),
  );
  return `${base}-${PRERELEASE_TAG}.${k}`;
}

/**
 * Asserts that a core manifest encodes its bundled Vite version, and that the
 * core version is a prerelease exactly when the `vite-plus` version is one.
 * With `release`, also rejects the local build counter.
 */
export function assertCoreVersionMatchesVite(
  pkg: { version: string; bundledVersions?: { vite?: string } },
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
        `Set its version to ${isPrereleaseVersion(vite) ? `${vite}.${PRERELEASE_TAG}.0` : encodeCoreVersion(vite, 0)}.`,
    );
  }
  const isCorePrerelease = decoded.k !== undefined;
  if (isCorePrerelease !== isPrereleaseVersion(vitePlus)) {
    throw new Error(
      `${CORE_PACKAGE_NAME}@${pkg.version} and vite-plus@${vitePlus} must both be prereleases or both be stable`,
    );
  }
  if (release && decoded.n === LOCAL_BUILD_COUNTER) {
    throw new Error(
      `${CORE_PACKAGE_NAME}@${pkg.version} uses the counter ${LOCAL_BUILD_COUNTER}, which is reserved for local builds`,
    );
  }
}

function readManifest(relativePath: string): {
  version: string;
  bundledVersions?: { vite?: string };
} {
  return JSON.parse(readFileSync(new URL(relativePath, import.meta.url), 'utf8'));
}

function readPublishedCoreVersions(): string[] {
  const output = execFileSync('npm', ['view', CORE_PACKAGE_NAME, 'versions', '--json'], {
    encoding: 'utf8',
  });
  const versions: unknown = JSON.parse(output);
  if (typeof versions === 'string') {
    return [versions];
  }
  if (!Array.isArray(versions) || versions.some((version) => typeof version !== 'string')) {
    throw new Error(`Unexpected output from npm view ${CORE_PACKAGE_NAME} versions`);
  }
  return versions;
}

function main(argv: string[]): void {
  const { positionals, values } = parseArgs({
    args: argv,
    allowPositionals: true,
    options: {
      'vite-plus': { type: 'string' },
      release: { type: 'boolean', default: false },
    },
  });
  const core = readManifest('../../core/package.json');
  const vite = core.bundledVersions?.vite;
  if (!vite) {
    throw new Error(`packages/core/package.json has no bundledVersions.vite`);
  }

  switch (positionals[0]) {
    case 'next': {
      const vitePlus = values['vite-plus'];
      if (!vitePlus) {
        throw new Error('next requires --vite-plus <version>');
      }
      console.log(nextCoreVersion({ vite, vitePlus, published: readPublishedCoreVersions() }));
      break;
    }
    case 'local':
      console.log(encodeCoreVersion(vite, LOCAL_BUILD_COUNTER));
      break;
    case 'check': {
      const cli = readManifest('../../cli/package.json');
      assertCoreVersionMatchesVite(core, cli.version, { release: values.release });
      console.log(`${CORE_PACKAGE_NAME}@${core.version} encodes Vite ${vite}`);
      break;
    }
    default:
      throw new Error(`Unknown command: ${positionals[0] ?? ''}. Use next, local, or check.`);
  }
}

if (import.meta.main) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    console.error(`[core-version] ${(error as Error).message}`);
    process.exit(1);
  }
}
