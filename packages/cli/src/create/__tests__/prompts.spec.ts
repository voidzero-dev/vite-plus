import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import { emptyDir, isTargetDirAvailable, suggestAvailableTargetDir } from '../prompts.js';

const tempDirs: string[] = [];

afterEach(() => {
  for (const dir of tempDirs.splice(0)) {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

function makeTempDir() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'vite-plus-create-'));
  tempDirs.push(dir);
  return dir;
}

describe('target directory helpers', () => {
  it('reports missing directories as available', () => {
    const cwd = makeTempDir();
    expect(isTargetDirAvailable(path.join(cwd, 'new-project'))).toBe(true);
  });

  it('reports non-empty directories as unavailable', () => {
    const cwd = makeTempDir();
    const targetDir = path.join(cwd, 'existing-project');
    fs.mkdirSync(targetDir, { recursive: true });
    fs.writeFileSync(path.join(targetDir, 'package.json'), '{}');

    expect(isTargetDirAvailable(targetDir)).toBe(false);
  });

  it('suggests a different target directory when the default already exists', () => {
    const cwd = makeTempDir();
    fs.mkdirSync(path.join(cwd, 'fate-template'), { recursive: true });
    fs.writeFileSync(path.join(cwd, 'fate-template', 'package.json'), '{}');

    expect(suggestAvailableTargetDir('fate-template', cwd)).not.toBe('fate-template');
  });
});

describe('emptyDir', () => {
  it('removes existing files but keeps .git', () => {
    const targetDir = makeTempDir();
    fs.mkdirSync(path.join(targetDir, '.git'));
    fs.mkdirSync(path.join(targetDir, 'src'));
    fs.writeFileSync(path.join(targetDir, 'package.json'), '{}');

    emptyDir(targetDir);

    expect(fs.readdirSync(targetDir)).toEqual(['.git']);
  });

  it('removes a symlinked target without touching the linked directory', () => {
    const cwd = makeTempDir();
    const linkedDir = path.join(cwd, 'linked-directory');
    const targetDir = path.join(cwd, 'new-project');
    fs.mkdirSync(linkedDir);
    fs.writeFileSync(path.join(linkedDir, 'keep.txt'), 'keep\n');
    fs.symlinkSync(linkedDir, targetDir, 'dir');

    emptyDir(targetDir);

    expect(fs.existsSync(targetDir)).toBe(false);
    expect(fs.readFileSync(path.join(linkedDir, 'keep.txt'), 'utf8')).toBe('keep\n');
  });

  it('ignores a missing target', () => {
    expect(() => emptyDir(path.join(makeTempDir(), 'missing'))).not.toThrow();
  });
});
