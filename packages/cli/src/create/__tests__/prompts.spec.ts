import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it } from 'vitest';

import {
  checkProjectDirExists,
  isTargetDirAvailable,
  suggestAvailableTargetDir,
} from '../prompts.js';

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

  it('does not ask to keep files in an available directory', async () => {
    const cwd = makeTempDir();
    fs.mkdirSync(path.join(cwd, '.git'));

    await expect(
      checkProjectDirExists(cwd, false, { canKeepExisting: true, keepExisting: true }),
    ).resolves.toBe(false);
  });

  it('keeps the files of a non-empty directory when forced', async () => {
    const cwd = makeTempDir();
    fs.writeFileSync(path.join(cwd, 'README.md'), 'mine');

    await expect(
      checkProjectDirExists(cwd, false, { canKeepExisting: true, keepExisting: true }),
    ).resolves.toBe(true);
    expect(fs.readFileSync(path.join(cwd, 'README.md'), 'utf-8')).toBe('mine');
  });

  it('suggests a different target directory when the default already exists', () => {
    const cwd = makeTempDir();
    fs.mkdirSync(path.join(cwd, 'fate-template'), { recursive: true });
    fs.writeFileSync(path.join(cwd, 'fate-template', 'package.json'), '{}');

    expect(suggestAvailableTargetDir('fate-template', cwd)).not.toBe('fate-template');
  });
});
