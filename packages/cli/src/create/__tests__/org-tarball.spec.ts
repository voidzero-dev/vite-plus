import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import {
  cleanupStaleStagingDirs,
  normalizeEntryName,
  parseEntryMode,
  resolveBundledPath,
  sanitizeHostForPath,
} from '../org-tarball.js';

describe('resolveBundledPath', () => {
  const scratchDirs: string[] = [];

  afterEach(() => {
    for (const dir of scratchDirs.splice(0)) {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });

  function tmpExtractedRoot(): string {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-org-tarball-'));
    scratchDirs.push(dir);
    // Populate it with a fake template directory.
    fs.mkdirSync(path.join(dir, 'templates', 'demo'), { recursive: true });
    fs.writeFileSync(path.join(dir, 'templates', 'demo', 'package.json'), '{"name":"demo"}');
    return dir;
  }

  it('resolves a simple ./subdir path', () => {
    const root = tmpExtractedRoot();
    expect(resolveBundledPath(root, './templates/demo')).toBe(path.join(root, 'templates', 'demo'));
  });

  it('rejects paths that escape the root via ..', () => {
    const root = tmpExtractedRoot();
    expect(() => resolveBundledPath(root, '../outside')).toThrow(/escapes the package root/);
  });

  it('rejects absolute paths', () => {
    const root = tmpExtractedRoot();
    expect(() => resolveBundledPath(root, '/etc/passwd')).toThrow(/must be relative/);
  });

  it('returns the resolved path even when it does not exist (caller handles ENOENT)', () => {
    const root = tmpExtractedRoot();
    expect(resolveBundledPath(root, './templates/ghost')).toBe(
      path.join(root, 'templates', 'ghost'),
    );
  });

  it('normalizes trailing slashes', () => {
    const root = tmpExtractedRoot();
    expect(resolveBundledPath(root, './templates/demo/')).toBe(
      path.join(root, 'templates', 'demo'),
    );
  });
});

describe('normalizeEntryName', () => {
  it('strips the `package/` prefix', () => {
    expect(normalizeEntryName('package/README.md')).toBe('README.md');
    expect(normalizeEntryName('package/src/index.ts')).toBe('src/index.ts');
  });

  it('normalizes leading `./` and backslashes to forward slashes', () => {
    expect(normalizeEntryName('./package/src/index.ts')).toBe('src/index.ts');
    expect(normalizeEntryName('package\\src\\index.ts')).toBe('src/index.ts');
  });

  it('returns null for the root `package/` directory and empty names', () => {
    expect(normalizeEntryName('package')).toBeNull();
    expect(normalizeEntryName('package/')).toBeNull();
    expect(normalizeEntryName('')).toBeNull();
  });

  it('returns null for PaxHeader metadata entries', () => {
    expect(normalizeEntryName('PaxHeader/foo')).toBeNull();
    expect(normalizeEntryName('package/PaxHeader/foo')).toBeNull();
  });

  it('returns null for entries outside the `package/` root', () => {
    expect(normalizeEntryName('not-package/foo.ts')).toBeNull();
    expect(normalizeEntryName('node_modules/foo/package.json')).toBeNull();
  });
});

describe('sanitizeHostForPath', () => {
  it('passes through plain hostnames untouched', () => {
    expect(sanitizeHostForPath('registry.npmjs.org')).toBe('registry.npmjs.org');
    expect(sanitizeHostForPath('private.example.com')).toBe('private.example.com');
  });

  it('replaces the port `:` so the cache path is valid on Windows', () => {
    expect(sanitizeHostForPath('localhost:4873')).toBe('localhost_4873');
  });

  it('strips IPv6 brackets and colons from the literal', () => {
    expect(sanitizeHostForPath('[::1]:4873')).toBe('___1__4873');
  });
});

describe('parseEntryMode', () => {
  it('parses octal `755` as 0o755', () => {
    expect(parseEntryMode('755')).toBe(0o755);
  });

  it('parses a longer octal string and masks to permission bits', () => {
    expect(parseEntryMode('100755')).toBe(0o755);
    // `104755` carries the setuid bit (0o4000) — drop it.
    expect(parseEntryMode('104755')).toBe(0o755);
  });

  it('returns undefined for missing or unparsable modes', () => {
    expect(parseEntryMode(undefined)).toBeUndefined();
    expect(parseEntryMode('')).toBeUndefined();
    expect(parseEntryMode('not-a-number')).toBeUndefined();
  });
});

describe('cleanupStaleStagingDirs', () => {
  const scratchDirs: string[] = [];

  afterEach(() => {
    for (const dir of scratchDirs.splice(0)) {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });

  function tmpDestDir(): { destDir: string; parent: string; base: string } {
    const parent = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-org-cleanup-'));
    scratchDirs.push(parent);
    const base = 'create-1.0.0';
    return { destDir: path.join(parent, base), parent, base };
  }

  function makeStaging(parent: string, base: string, ageMs: number): string {
    const name = `${base}.tmp-${process.pid}-${Date.now() - ageMs}`;
    const dir = path.join(parent, name);
    fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(path.join(dir, 'marker'), '');
    const then = new Date(Date.now() - ageMs);
    fs.utimesSync(dir, then, then);
    return dir;
  }

  it('deletes siblings older than 24h', async () => {
    const { destDir, parent, base } = tmpDestDir();
    const stale = makeStaging(parent, base, 25 * 60 * 60 * 1000);
    await cleanupStaleStagingDirs(destDir);
    expect(fs.existsSync(stale)).toBe(false);
  });

  it('leaves fresh siblings in place (concurrency safety)', async () => {
    const { destDir, parent, base } = tmpDestDir();
    const fresh = makeStaging(parent, base, 60 * 1000);
    await cleanupStaleStagingDirs(destDir);
    expect(fs.existsSync(fresh)).toBe(true);
  });

  it('ignores unrelated siblings (different basename prefix)', async () => {
    const { destDir, parent } = tmpDestDir();
    const other = makeStaging(parent, 'unrelated-2.0.0', 48 * 60 * 60 * 1000);
    await cleanupStaleStagingDirs(destDir);
    expect(fs.existsSync(other)).toBe(true);
  });

  it('tolerates a missing parent directory', async () => {
    const destDir = path.join(os.tmpdir(), 'vp-org-cleanup-missing', 'nope');
    await expect(cleanupStaleStagingDirs(destDir)).resolves.toBeUndefined();
  });
});
