import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { mockSelect } = vi.hoisted(() => ({
  mockSelect: vi.fn(),
}));

vi.mock('@voidzero-dev/vite-plus-prompts', () => ({
  isCancel: () => false,
  select: mockSelect,
}));

vi.mock('../../utils/prompts.ts', () => ({
  cancelAndExit: vi.fn(() => {
    throw new Error('Operation cancelled');
  }),
}));

vi.mock('../../utils/terminal.ts', () => ({
  accent: (value: string) => value,
}));

const { checkProjectDirExists, isTargetDirAvailable, suggestAvailableTargetDir } =
  await import('../prompts.js');

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
  beforeEach(() => {
    mockSelect.mockReset();
  });

  it('reports missing directories as available', () => {
    const cwd = makeTempDir();
    expect(isTargetDirAvailable(path.join(cwd, 'new-project'))).toBe(true);
  });

  it('reports empty directories as available', () => {
    const cwd = makeTempDir();
    const targetDir = path.join(cwd, 'empty-project');
    fs.mkdirSync(targetDir);

    expect(isTargetDirAvailable(targetDir)).toBe(true);
  });

  it('reports non-empty directories as unavailable', () => {
    const cwd = makeTempDir();
    const targetDir = path.join(cwd, 'existing-project');
    fs.mkdirSync(targetDir, { recursive: true });
    fs.writeFileSync(path.join(targetDir, 'package.json'), '{}');

    expect(isTargetDirAvailable(targetDir)).toBe(false);
  });

  it('reports a symlink to an empty directory as unavailable', () => {
    const cwd = makeTempDir();
    const linkedDir = makeTempDir();
    const targetDir = path.join(cwd, 'new-project');
    fs.symlinkSync(linkedDir, targetDir, process.platform === 'win32' ? 'junction' : 'dir');

    expect(isTargetDirAvailable(targetDir)).toBe(false);
  });

  it('suggests a different target directory when the default already exists', () => {
    const cwd = makeTempDir();
    fs.mkdirSync(path.join(cwd, 'fate-template'), { recursive: true });
    fs.writeFileSync(path.join(cwd, 'fate-template', 'package.json'), '{}');

    expect(suggestAvailableTargetDir('fate-template', cwd)).not.toBe('fate-template');
  });

  it('clears a regular target directory while preserving its .git directory', async () => {
    const cwd = makeTempDir();
    const targetDir = path.join(cwd, 'existing-project');
    fs.mkdirSync(path.join(targetDir, '.git'), { recursive: true });
    fs.mkdirSync(path.join(targetDir, 'src'));
    fs.writeFileSync(path.join(targetDir, '.git', 'config'), 'keep');
    fs.writeFileSync(path.join(targetDir, 'src', 'main.ts'), 'remove');
    fs.writeFileSync(path.join(targetDir, 'package.json'), '{}');
    mockSelect.mockResolvedValue('yes');

    await checkProjectDirExists(targetDir, true);

    expect(fs.readdirSync(targetDir)).toEqual(['.git']);
    expect(fs.readFileSync(path.join(targetDir, '.git', 'config'), 'utf8')).toBe('keep');
  });

  it('removes a target symlink without deleting files in the linked directory', async () => {
    const cwd = makeTempDir();
    const linkedDir = makeTempDir();
    const targetDir = path.join(cwd, 'new-project');
    const sentinel = path.join(linkedDir, 'keep.txt');
    fs.writeFileSync(sentinel, 'keep');
    fs.symlinkSync(linkedDir, targetDir, process.platform === 'win32' ? 'junction' : 'dir');
    mockSelect.mockResolvedValue('yes');

    await checkProjectDirExists(targetDir, true);

    expect(fs.existsSync(sentinel)).toBe(true);
    expect(fs.lstatSync(targetDir, { throwIfNoEntry: false })).toBeUndefined();
    expect(mockSelect).toHaveBeenCalledWith({
      message: `Target path "${targetDir}" is a symbolic link. Please choose how to proceed:`,
      options: [
        { label: 'Cancel operation', value: 'no' },
        { label: 'Remove symbolic link and continue', value: 'yes' },
      ],
    });
  });

  it('removes a target symlink with a trailing separator without deleting linked files', async () => {
    const cwd = makeTempDir();
    const linkedDir = makeTempDir();
    const targetDir = path.join(cwd, 'new-project');
    const targetDirWithSeparator = `${targetDir}${path.sep}`;
    const sentinel = path.join(linkedDir, 'keep.txt');
    fs.writeFileSync(sentinel, 'keep');
    fs.symlinkSync(linkedDir, targetDir, process.platform === 'win32' ? 'junction' : 'dir');
    mockSelect.mockResolvedValue('yes');

    expect(isTargetDirAvailable(targetDirWithSeparator)).toBe(false);

    await checkProjectDirExists(targetDirWithSeparator, true);

    expect(fs.existsSync(sentinel)).toBe(true);
    expect(fs.lstatSync(targetDir, { throwIfNoEntry: false })).toBeUndefined();
    expect(mockSelect).toHaveBeenCalledWith({
      message: `Target path "${targetDir}" is a symbolic link. Please choose how to proceed:`,
      options: [
        { label: 'Cancel operation', value: 'no' },
        { label: 'Remove symbolic link and continue', value: 'yes' },
      ],
    });
  });

  it('recognizes and removes a dangling target symlink', async () => {
    const cwd = makeTempDir();
    const targetDir = path.join(cwd, 'new-project');
    fs.symlinkSync(
      path.join(cwd, 'missing-directory'),
      targetDir,
      process.platform === 'win32' ? 'junction' : 'dir',
    );
    mockSelect.mockResolvedValue('yes');

    expect(isTargetDirAvailable(targetDir)).toBe(false);

    await checkProjectDirExists(targetDir, true);

    expect(fs.lstatSync(targetDir, { throwIfNoEntry: false })).toBeUndefined();
  });

  it('recognizes and removes an existing file at the target path', async () => {
    const cwd = makeTempDir();
    const targetPath = path.join(cwd, 'new-project');
    fs.writeFileSync(targetPath, 'remove');
    mockSelect.mockResolvedValue('yes');

    expect(isTargetDirAvailable(targetPath)).toBe(false);

    await checkProjectDirExists(targetPath, true);

    expect(fs.existsSync(targetPath)).toBe(false);
    expect(mockSelect).toHaveBeenCalledWith({
      message: `Target path "${targetPath}" already exists. Please choose how to proceed:`,
      options: [
        { label: 'Cancel operation', value: 'no' },
        { label: 'Remove existing path and continue', value: 'yes' },
      ],
    });
  });

  it('does not clear a directory that replaces a target symlink during confirmation', async () => {
    const cwd = makeTempDir();
    const linkedDir = makeTempDir();
    const targetPath = path.join(cwd, 'new-project');
    const replacementPath = path.join(cwd, 'replacement-project');
    const linkedSentinel = path.join(linkedDir, 'keep.txt');
    const replacementSentinel = path.join(targetPath, 'keep.txt');
    fs.writeFileSync(linkedSentinel, 'keep linked');
    fs.mkdirSync(replacementPath);
    fs.writeFileSync(path.join(replacementPath, 'keep.txt'), 'keep replacement');
    fs.symlinkSync(linkedDir, targetPath, process.platform === 'win32' ? 'junction' : 'dir');
    mockSelect.mockImplementation(() => {
      fs.rmSync(targetPath, { force: true });
      fs.renameSync(replacementPath, targetPath);
      return Promise.resolve('yes');
    });

    await expect(checkProjectDirExists(targetPath, true)).rejects.toThrow(
      `Target path "${targetPath}" changed while waiting for confirmation. No files were removed. Please retry the command.`,
    );

    expect(fs.readFileSync(linkedSentinel, 'utf8')).toBe('keep linked');
    expect(fs.readFileSync(replacementSentinel, 'utf8')).toBe('keep replacement');
  });

  it('does not remove a file that replaces the confirmed target path', async () => {
    const cwd = makeTempDir();
    const targetPath = path.join(cwd, 'new-project');
    const originalPath = path.join(cwd, 'original-project');
    const replacementPath = path.join(cwd, 'replacement-project');
    fs.writeFileSync(targetPath, 'original');
    fs.writeFileSync(replacementPath, 'replacement');
    mockSelect.mockImplementation(() => {
      fs.renameSync(targetPath, originalPath);
      fs.renameSync(replacementPath, targetPath);
      return Promise.resolve('yes');
    });

    await expect(checkProjectDirExists(targetPath, true)).rejects.toThrow(
      `Target path "${targetPath}" changed while waiting for confirmation. No files were removed. Please retry the command.`,
    );

    expect(fs.readFileSync(originalPath, 'utf8')).toBe('original');
    expect(fs.readFileSync(targetPath, 'utf8')).toBe('replacement');
  });
});
