import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

/** The registry used by the release workflow. */
export const DEFAULT_NPM_REGISTRY = 'https://registry.npmjs.org';

/**
 * npm installs resolve versions from the abbreviated packument, which is
 * cached separately from the full package document.
 */
const ABBREVIATED_PACKUMENT_ACCEPT = 'application/vnd.npm.install-v1+json';

export interface NpmPackageVersion {
  name: string;
  version: string;
}

interface AbbreviatedPackument {
  versions?: Record<string, { dist?: { tarball?: string } }>;
}

export type FetchLike = (
  url: string,
  init?: { method?: string; headers?: Record<string, string> },
) => Promise<{ ok: boolean; status: number; json: () => Promise<unknown> }>;

export interface WaitForNpmPackagesOptions {
  registry: string;
  fetchImpl: FetchLike;
  /** Wait this long after all versions become available. */
  minSeconds: number;
  timeoutSeconds: number;
  pollSeconds: number;
  sleep: (milliseconds: number) => Promise<void>;
  now: () => number;
  log: (message: string) => void;
}

function escapePackageName(name: string): string {
  return name.replace('/', '%2f');
}

async function fetchNpmPackument(
  name: string,
  options: Pick<WaitForNpmPackagesOptions, 'registry' | 'fetchImpl'>,
): Promise<AbbreviatedPackument | null> {
  const registry = options.registry.replace(/\/+$/, '');
  const response = await options.fetchImpl(`${registry}/${escapePackageName(name)}`, {
    headers: { accept: ABBREVIATED_PACKUMENT_ACCEPT },
  });
  if (response.status === 404) {
    return null;
  }
  if (!response.ok) {
    throw new Error(`registry returned HTTP ${response.status}`);
  }
  return (await response.json()) as AbbreviatedPackument;
}

/** Checks whether npm has made an immutable package version visible. */
export async function isNpmPackagePublished(
  pkg: NpmPackageVersion,
  options: Pick<WaitForNpmPackagesOptions, 'registry' | 'fetchImpl'>,
): Promise<boolean> {
  const packument = await fetchNpmPackument(pkg.name, options);
  return packument?.versions?.[pkg.version] !== undefined;
}

/**
 * Checks the same metadata document an npm install uses, then verifies that
 * the tarball referenced by that document can also be fetched.
 */
export async function isNpmPackageAvailable(
  pkg: NpmPackageVersion,
  options: Pick<WaitForNpmPackagesOptions, 'registry' | 'fetchImpl'>,
): Promise<boolean> {
  const packument = await fetchNpmPackument(pkg.name, options);
  const tarball = packument?.versions?.[pkg.version]?.dist?.tarball;
  if (!tarball) {
    return false;
  }

  const tarballResponse = await options.fetchImpl(tarball, { method: 'HEAD' });
  return tarballResponse.ok;
}

/**
 * Waits until every package version is installable before the release moves
 * on to a package that pins it as a dependency.
 */
export async function waitForNpmPackages(
  packages: readonly NpmPackageVersion[],
  options: WaitForNpmPackagesOptions,
): Promise<void> {
  if (packages.length === 0) {
    return;
  }

  const start = options.now();
  const deadline = start + options.timeoutSeconds * 1000;
  const pending = new Map(packages.map((pkg) => [pkg.name, pkg.version]));

  options.log(`Waiting for ${pending.size} npm package version(s) to become installable...`);

  while (pending.size > 0) {
    for (const [name, version] of pending) {
      try {
        if (await isNpmPackageAvailable({ name, version }, options)) {
          options.log(`  ${name}@${version}: available`);
          pending.delete(name);
        }
      } catch (error) {
        options.log(`  ${name}@${version}: check failed, retrying (${String(error)})`);
      }
    }

    if (pending.size === 0) {
      break;
    }
    if (options.now() >= deadline) {
      const packageList = [...pending].map(([name, version]) => `${name}@${version}`).join(', ');
      throw new Error(
        `Timed out after ${options.timeoutSeconds}s waiting for npm propagation: ${packageList}`,
      );
    }
    await options.sleep(options.pollSeconds * 1000);
  }

  if (options.minSeconds > 0) {
    const elapsedSeconds = Math.round((options.now() - start) / 1000);
    options.log(
      `All versions are installable after ${elapsedSeconds}s; settling for a further ${options.minSeconds}s.`,
    );
    await options.sleep(options.minSeconds * 1000);
  }
}

export function parseNpmPackageSpec(spec: string): NpmPackageVersion {
  const separator = spec.lastIndexOf('@');
  if (separator <= 0 || separator === spec.length - 1) {
    throw new Error(`Expected a package argument in the form name@version, received ${spec}`);
  }
  return { name: spec.slice(0, separator), version: spec.slice(separator + 1) };
}

function readNonNegativeInteger(name: string, fallback: number): number {
  const value = process.env[name];
  if (value === undefined || value.trim() === '') {
    return fallback;
  }
  if (!/^\d+$/.test(value)) {
    throw new Error(`Expected ${name} to be a non-negative integer, received ${value}`);
  }
  return Number(value);
}

/** Uses the release workflow's propagation settings. */
export async function waitForNpmPackagesFromEnv(
  packages: readonly NpmPackageVersion[],
): Promise<void> {
  if (process.env.PUBLISH_SKIP_PROPAGATION_WAIT === 'true') {
    console.log('Skipping npm propagation wait.');
    return;
  }

  await waitForNpmPackages(packages, {
    registry: process.env.PUBLISH_REGISTRY ?? DEFAULT_NPM_REGISTRY,
    fetchImpl: fetch,
    minSeconds: readNonNegativeInteger('PUBLISH_PROPAGATION_MIN_SECONDS', 60),
    timeoutSeconds: readNonNegativeInteger('PUBLISH_PROPAGATION_TIMEOUT_SECONDS', 600),
    pollSeconds: readNonNegativeInteger('PUBLISH_PROPAGATION_POLL_SECONDS', 5),
    sleep: (milliseconds) => new Promise((resolveSleep) => setTimeout(resolveSleep, milliseconds)),
    now: Date.now,
    log: console.log,
  });
}

async function main(): Promise<void> {
  const specs = process.argv.slice(2);
  if (specs.length === 0) {
    throw new Error('Usage: node .github/scripts/wait-for-npm-packages.ts <name@version> [...]');
  }
  await waitForNpmPackagesFromEnv(specs.map(parseNpmPackageSpec));
}

const invokedPath = process.argv[1];
if (invokedPath && pathToFileURL(resolve(invokedPath)).href === import.meta.url) {
  main().catch((error: unknown) => {
    console.error(`::error::${error instanceof Error ? error.message : String(error)}`);
    process.exitCode = 1;
  });
}
