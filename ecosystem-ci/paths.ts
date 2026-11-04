import { readdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectDir = dirname(fileURLToPath(import.meta.url));

// Use RUNNER_TEMP in GitHub Actions, otherwise use system temp directory
const tempBase = process.env.RUNNER_TEMP ?? tmpdir();

export const ecosystemCiDir = join(tempBase, 'vite-plus-ecosystem-ci');

// tgz path: always use local tmp/tgz
export const tgzDir = join(projectDir, '..', 'tmp', 'tgz');

/**
 * The version of the packed vite-plus tgz that the local registry serves and
 * that `vp migrate` pins: 0.0.0 on CI (the e2e pack step pins it so a local
 * build is always distinguishable from a published version), the checkout
 * version on a local run.
 */
export function vitePlusTgzVersion(): string {
  return tgzVersion(/^vite-plus-(\d.*)\.tgz$/, 'vite-plus-<version>.tgz');
}

/**
 * The version of the packed core tgz. It encodes the bundled Vite version, so
 * it differs from {@link vitePlusTgzVersion} (rfcs/core-version-alignment.md).
 */
export function vitePlusCoreTgzVersion(): string {
  return tgzVersion(
    /^voidzero-dev-vite-plus-core-(\d.*)\.tgz$/,
    'voidzero-dev-vite-plus-core-<version>.tgz',
  );
}

function tgzVersion(pattern: RegExp, description: string): string {
  const version = readdirSync(tgzDir)
    .map((entry) => pattern.exec(entry)?.[1])
    .find(Boolean);
  if (!version) {
    throw new Error(`No ${description} found in ${tgzDir}`);
  }
  return version;
}
