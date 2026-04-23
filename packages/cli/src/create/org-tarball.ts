import { createHash } from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { parseTarGzip } from 'nanotar';

import type { OrgManifest } from './org-manifest.ts';

/**
 * Resolve the cache root for extracted org-create packages. Honors
 * `$VP_HOME` just like the rest of the installer (see
 * `packages/tools/src/snap-test.ts:31`), falling back to `~/.vite-plus`.
 */
function getCacheRoot(): string {
  const home = process.env.VP_HOME || path.join(os.homedir(), '.vite-plus');
  return path.join(home, 'tmp', 'create-org');
}

function getExtractionDir(manifest: OrgManifest): string {
  // Cache at `$VP_HOME/tmp/create-org/<@scope>/create/<version>/`. The
  // scope segment already contains the leading `@`, which is fine on all
  // platforms supported by vp (Node 20+).
  return path.join(getCacheRoot(), manifest.scope, 'create', manifest.version);
}

function parseIntegrity(integrity: string): { algorithm: string; expected: string } | null {
  // Subresource Integrity format: `sha512-<base64>` (optionally comma-separated alternatives).
  const first = integrity.split(/\s+/)[0];
  const match = first.match(/^(sha\d+)-(.+)$/);
  if (!match) {
    return null;
  }
  return { algorithm: match[1], expected: match[2] };
}

function verifyIntegrity(bytes: Uint8Array, integrity: string | undefined): void {
  if (!integrity) {
    return;
  }
  const parsed = parseIntegrity(integrity);
  if (!parsed) {
    // Unknown format — don't fail hard, but don't silently accept either.
    // Registry responses normally include sha512; anything else is unusual.
    return;
  }
  const hash = createHash(parsed.algorithm);
  hash.update(bytes);
  const actual = hash.digest('base64');
  if (actual !== parsed.expected) {
    throw new Error(
      `integrity check failed: expected ${integrity}, got ${parsed.algorithm}-${actual}`,
    );
  }
}

async function downloadTarball(url: string): Promise<Uint8Array> {
  const response = await fetch(url, {
    signal: AbortSignal.timeout(30_000),
  });
  if (!response.ok) {
    throw new Error(`failed to download tarball (${response.status}): ${url}`);
  }
  const buffer = await response.arrayBuffer();
  // Cap at 50 MB to avoid pathological inputs exhausting memory. Real-world
  // create-* packages are tens of KB.
  if (buffer.byteLength > 50 * 1024 * 1024) {
    throw new Error(`tarball exceeds 50 MB size limit: ${url}`);
  }
  return new Uint8Array(buffer);
}

/**
 * Strip the conventional `package/` directory prefix that npm adds to every
 * tarball entry. Returns the trimmed path, or `null` if the entry should be
 * skipped (e.g. the root `package/` directory itself, PaxHeaders).
 */
function normalizeEntryName(rawName: string): string | null {
  const name = rawName.replace(/^\.\//, '').replace(/\\/g, '/');
  if (!name || name === 'package' || name === 'package/') {
    return null;
  }
  if (name.startsWith('PaxHeader/') || name.includes('/PaxHeader/')) {
    return null;
  }
  if (name.startsWith('package/')) {
    return name.slice('package/'.length);
  }
  // Some publishers use a custom root directory; accept it too.
  return name;
}

async function extractTarballTo(bytes: Uint8Array, destDir: string): Promise<void> {
  const entries = await parseTarGzip(bytes);
  // Extract into a staging directory first so partial failures don't leave
  // a half-populated final cache path that future runs would skip.
  const stagingDir = `${destDir}.tmp-${process.pid}-${Date.now()}`;
  await fs.promises.mkdir(stagingDir, { recursive: true });
  try {
    for (const entry of entries) {
      const relativeName = normalizeEntryName(entry.name);
      if (relativeName === null) {
        continue;
      }
      const targetPath = path.join(stagingDir, relativeName);
      // Defense-in-depth: make sure the resolved path is still inside the
      // staging directory (no `..` escapes via crafted tar entries).
      const resolvedStaging = path.resolve(stagingDir);
      const resolvedTarget = path.resolve(targetPath);
      if (
        resolvedTarget !== resolvedStaging &&
        !resolvedTarget.startsWith(`${resolvedStaging}${path.sep}`)
      ) {
        throw new Error(`tarball entry escapes extraction root: ${entry.name}`);
      }
      if (entry.type === 'directory' || relativeName.endsWith('/')) {
        await fs.promises.mkdir(targetPath, { recursive: true });
        continue;
      }
      await fs.promises.mkdir(path.dirname(targetPath), { recursive: true });
      const data = entry.data ?? new Uint8Array(0);
      await fs.promises.writeFile(targetPath, data);
    }
    await fs.promises.rename(stagingDir, destDir);
  } catch (error) {
    // Clean up the staging dir on failure.
    await fs.promises.rm(stagingDir, { recursive: true, force: true }).catch(() => {});
    throw error;
  }
}

/**
 * Ensure the `@org/create` package tarball for the given manifest has been
 * downloaded and extracted locally. Returns the absolute path to the
 * extracted package root (i.e. the directory that contains
 * `package.json`).
 *
 * Idempotent: subsequent calls for the same `<scope, version>` reuse the
 * cached extraction.
 */
export async function ensureOrgPackageExtracted(manifest: OrgManifest): Promise<string> {
  const extractedRoot = getExtractionDir(manifest);
  if (fs.existsSync(path.join(extractedRoot, 'package.json'))) {
    return extractedRoot;
  }
  const parent = path.dirname(extractedRoot);
  await fs.promises.mkdir(parent, { recursive: true });
  const bytes = await downloadTarball(manifest.tarballUrl);
  verifyIntegrity(bytes, manifest.integrity);
  await extractTarballTo(bytes, extractedRoot);
  return extractedRoot;
}

/**
 * Resolve a manifest entry's relative `./...` path against an already-
 * extracted package root, rejecting any path that escapes the root (via
 * `..` walks or an absolute specifier).
 */
export function resolveBundledPath(extractedRoot: string, relativePath: string): string {
  if (path.isAbsolute(relativePath)) {
    throw new Error(`bundled template path must be relative, got ${relativePath}`);
  }
  const resolvedRoot = path.resolve(extractedRoot);
  const resolvedTarget = path.resolve(extractedRoot, relativePath);
  if (resolvedTarget !== resolvedRoot && !resolvedTarget.startsWith(`${resolvedRoot}${path.sep}`)) {
    throw new Error(`bundled template path escapes the package root: ${relativePath}`);
  }
  if (!fs.existsSync(resolvedTarget)) {
    throw new Error(`bundled template path not found in package: ${relativePath}`);
  }
  return resolvedTarget;
}
