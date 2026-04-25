import { createHash } from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { parseTarGzip } from 'nanotar';

import { fetchNpmResource } from '../utils/npm-config.ts';
import type { OrgManifest } from './org-manifest.ts';

function getCacheRoot(): string {
  const home = process.env.VP_HOME || path.join(os.homedir(), '.vite-plus');
  return path.join(home, 'tmp', 'create-org');
}

/**
 * Replace characters that are illegal in Windows path segments
 * (`\ / : * ? " < > |` plus the IPv6 bracket pair `[ ]`). The host
 * comes from `new URL(...).host` which can carry a port (`:4873`) or
 * IPv6 literal (`[::1]`); both end up in the cache path otherwise.
 */
export function sanitizeHostForPath(host: string): string {
  return host.replaceAll(/[\\/:*?"<>|[\]]/g, '_');
}

/**
 * Cache extracted tarballs under `<host>/<scope>/create/<version>` so two
 * repos resolving the same `<scope>@<version>` through different registries
 * (via `.npmrc` scope mappings) don't share a cache slot. The registry
 * guarantees `manifest.tarballUrl` is a valid URL, so any parse failure
 * here is a real bug worth surfacing.
 */
function getExtractionDir(manifest: OrgManifest): string {
  const { host } = new URL(manifest.tarballUrl);
  return path.join(
    getCacheRoot(),
    sanitizeHostForPath(host),
    manifest.scope,
    'create',
    manifest.version,
  );
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
    // Unknown integrity format — skip verification. Registry responses
    // normally advertise `sha512-<base64>`; anything else is unusual
    // enough that we'd rather let the extract continue than fail hard
    // on a format we don't understand.
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

const MAX_TARBALL_BYTES = 50 * 1024 * 1024;

async function downloadTarball(url: string): Promise<Uint8Array> {
  const response = await fetchNpmResource(url, { timeoutMs: 30_000 });
  if (!response.ok) {
    throw new Error(`failed to download tarball (${response.status}): ${url}`);
  }
  // Cheap pre-check when the server reports Content-Length; streaming loop
  // below is authoritative for servers that omit the header.
  const contentLength = Number(response.headers.get('content-length'));
  if (Number.isFinite(contentLength) && contentLength > MAX_TARBALL_BYTES) {
    throw new Error(`tarball exceeds ${MAX_TARBALL_BYTES} byte size limit: ${url}`);
  }
  // Stream the body so a 1 GB response is rejected before it's fully
  // buffered. Real-world create-* packages are tens of KB, so the cap is
  // only ever a safety net for malicious or misconfigured publishers.
  const reader = response.body?.getReader();
  if (!reader) {
    throw new Error(`tarball response has no body: ${url}`);
  }
  const chunks: Uint8Array[] = [];
  let total = 0;
  while (true) {
    const { done, value } = await reader.read();
    if (done) {
      break;
    }
    total += value.byteLength;
    if (total > MAX_TARBALL_BYTES) {
      await reader.cancel();
      throw new Error(`tarball exceeds ${MAX_TARBALL_BYTES} byte size limit: ${url}`);
    }
    chunks.push(value);
  }
  const bytes = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return bytes;
}

const STAGING_SUFFIX_PREFIX = '.tmp-';

/**
 * Parse a tar entry's stored mode (always octal) into the numeric
 * permission bits (low 9 bits — `rwxrwxrwx`). Returns `undefined` when
 * the mode is missing or unparsable so the caller leaves the file with
 * its default (umask-derived) permissions instead of downgrading.
 */
export function parseEntryMode(raw: string | undefined): number | undefined {
  if (!raw) {
    return undefined;
  }
  const parsed = Number.parseInt(raw, 8);
  if (!Number.isFinite(parsed)) {
    return undefined;
  }
  return parsed & 0o777;
}

/**
 * Strip the `package/` prefix from an `npm pack` tarball entry. Returns
 * `null` for entries to skip (root dir, PaxHeader, anything outside
 * `package/`).
 */
export function normalizeEntryName(rawName: string): string | null {
  const name = rawName.replace(/^\.\//, '').replace(/\\/g, '/');
  if (!name || name === 'package' || name === 'package/') {
    return null;
  }
  if (name.startsWith('PaxHeader/') || name.includes('/PaxHeader/')) {
    return null;
  }
  if (!name.startsWith('package/')) {
    return null;
  }
  return name.slice('package/'.length);
}

async function extractTarballTo(bytes: Uint8Array, destDir: string): Promise<void> {
  const entries = await parseTarGzip(bytes);
  // Extract into a staging directory first so partial failures don't leave
  // a half-populated final cache path that future runs would skip.
  const stagingDir = `${destDir}${STAGING_SUFFIX_PREFIX}${process.pid}-${Date.now()}`;
  await fs.promises.mkdir(stagingDir, { recursive: true });
  const resolvedStaging = path.resolve(stagingDir);
  try {
    for (const entry of entries) {
      const relativeName = normalizeEntryName(entry.name);
      if (relativeName === null) {
        continue;
      }
      const targetPath = path.join(stagingDir, relativeName);
      // Defense-in-depth: make sure the resolved path is still inside the
      // staging directory (no `..` escapes via crafted tar entries).
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
      // Preserve the tar entry's mode so bundled templates can ship
      // executable files (e.g. `gradlew`, `mvnw`, `scripts/*.sh`). Mask
      // to the permission bits only — setuid/setgid/sticky have no
      // business in a scaffolded project template.
      const mode = parseEntryMode(entry.attrs?.mode);
      if (mode !== undefined) {
        await fs.promises.chmod(targetPath, mode);
      }
    }
    try {
      await fs.promises.rename(stagingDir, destDir);
    } catch (error) {
      // `rename` reports ENOTEMPTY/EEXIST when a concurrent extractor
      // already populated `destDir`. Confirm that's actually what happened
      // (rather than permissions / read-only FS masquerading as a race)
      // before swallowing the error and treating our work as redundant.
      const code = (error as NodeJS.ErrnoException).code;
      if (
        (code === 'ENOTEMPTY' || code === 'EEXIST') &&
        fs.existsSync(path.join(destDir, 'package.json'))
      ) {
        await fs.promises.rm(stagingDir, { recursive: true, force: true }).catch(() => {});
        return;
      }
      throw error;
    }
  } catch (error) {
    await fs.promises.rm(stagingDir, { recursive: true, force: true }).catch(() => {});
    throw error;
  }
}

const STAGING_STALE_MS = 24 * 60 * 60 * 1000;

/**
 * Remove `<destDir>.tmp-*` siblings left behind by a previous crash so
 * repeated aborts don't accumulate orphaned staging trees. Only deletes
 * entries whose mtime is older than 24 hours — a concurrent `vp create`
 * that's still actively extracting will always be younger than that, so
 * the age gate keeps this safe to run at the top of every extract.
 */
export async function cleanupStaleStagingDirs(destDir: string): Promise<void> {
  const parent = path.dirname(destDir);
  const basename = path.basename(destDir);
  const prefix = `${basename}${STAGING_SUFFIX_PREFIX}`;
  let entries: string[];
  try {
    entries = await fs.promises.readdir(parent);
  } catch {
    return;
  }
  const cutoff = Date.now() - STAGING_STALE_MS;
  await Promise.all(
    entries
      .filter((name) => name.startsWith(prefix))
      .map(async (name) => {
        const fullPath = path.join(parent, name);
        try {
          const stats = await fs.promises.stat(fullPath);
          if (stats.mtimeMs < cutoff) {
            await fs.promises.rm(fullPath, { recursive: true, force: true });
          }
        } catch {
          // Entry vanished between readdir and stat/rm — nothing to do.
        }
      }),
  );
}

/**
 * Ensure the `@org/create` package tarball for the given manifest has been
 * downloaded and extracted locally. Returns the absolute path to the
 * extracted package root (i.e. the directory that contains
 * `package.json`).
 *
 * Idempotent: subsequent calls for the same `<scope, version>` reuse the
 * cached extraction. Concurrent calls race on the final rename; the loser
 * cleans up and returns the existing directory.
 */
export async function ensureOrgPackageExtracted(manifest: OrgManifest): Promise<string> {
  const extractedRoot = getExtractionDir(manifest);
  if (fs.existsSync(path.join(extractedRoot, 'package.json'))) {
    return extractedRoot;
  }
  const parent = path.dirname(extractedRoot);
  await fs.promises.mkdir(parent, { recursive: true });
  await cleanupStaleStagingDirs(extractedRoot);
  const bytes = await downloadTarball(manifest.tarballUrl);
  verifyIntegrity(bytes, manifest.integrity);
  await extractTarballTo(bytes, extractedRoot);
  return extractedRoot;
}

/**
 * Resolve a manifest entry's relative `./...` path against an already-
 * extracted package root, rejecting any path that escapes the root (via
 * `..` walks or an absolute specifier).
 *
 * Existence is NOT checked here — the subsequent `copyDir` surfaces any
 * missing-directory error with a clearer errno.
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
  return resolvedTarget;
}
