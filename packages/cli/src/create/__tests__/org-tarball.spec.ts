import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import { resolveBundledPath } from '../org-tarball.js';

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
