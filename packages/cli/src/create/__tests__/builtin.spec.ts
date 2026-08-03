import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { executeBuiltinTemplate } from '../templates/builtin.js';

const { mockLogError, mockRunRemoteTemplateCommand } = vi.hoisted(() => ({
  mockLogError: vi.fn(),
  mockRunRemoteTemplateCommand: vi.fn(),
}));

vi.mock('../templates/remote.js', () => ({
  runRemoteTemplateCommand: mockRunRemoteTemplateCommand,
}));

vi.mock('@voidzero-dev/vite-plus-prompts', () => ({
  log: { error: mockLogError },
}));

const workspaceInfo = {
  rootDir: '/tmp/workspace',
} as any;

const baseTemplateInfo = {
  packageName: 'wage-meeting',
  targetDir: 'wage-meeting',
  args: [],
  envs: {},
  type: 'builtin' as any,
  interactive: false,
};

const tempDirs: string[] = [];

beforeEach(() => {
  mockLogError.mockClear();
  mockRunRemoteTemplateCommand.mockReset();
});

afterEach(() => {
  for (const dir of tempDirs.splice(0)) {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

function makeWorkspaceInfo() {
  const rootDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vite-plus-library-'));
  tempDirs.push(rootDir);
  return {
    rootDir,
    parentDirs: [],
    packageManager: 'pnpm',
    downloadPackageManager: { binPrefix: '' },
  } as any;
}

describe('executeBuiltinTemplate', () => {
  it('returns exitCode 1 for unknown vite: template', async () => {
    const result = await executeBuiltinTemplate(workspaceInfo, {
      ...baseTemplateInfo,
      command: 'vite:test',
    });

    expect(result.exitCode).toBe(1);
    expect(mockRunRemoteTemplateCommand).not.toHaveBeenCalled();
  });

  it('shows error message with template name and --list hint', async () => {
    mockLogError.mockClear();

    await executeBuiltinTemplate(workspaceInfo, {
      ...baseTemplateInfo,
      command: 'vite:unknown',
    });

    expect(mockLogError).toHaveBeenCalledOnce();
    const message = mockLogError.mock.calls[0][0] as string;
    expect(message).toContain('vite:unknown');
    expect(message).toContain('vp create --list');
  });

  it('does not show error message in silent mode', async () => {
    mockLogError.mockClear();

    await executeBuiltinTemplate(
      workspaceInfo,
      { ...baseTemplateInfo, command: 'vite:test' },
      { silent: true },
    );

    expect(mockLogError).not.toHaveBeenCalled();
  });

  it('uses degit --force for a worktree directory and preserves .git', async () => {
    const workspace = makeWorkspaceInfo();
    const gitFile = path.join(workspace.rootDir, '.git');
    fs.writeFileSync(gitFile, 'gitdir: ../.git/worktrees/library');
    mockRunRemoteTemplateCommand.mockImplementation(async (_workspace: unknown, cwd: string) => {
      fs.writeFileSync(path.join(cwd, 'package.json'), '{"name":"template"}\n');
      return { exitCode: 0, stdout: Buffer.alloc(0), stderr: Buffer.alloc(0) };
    });

    const result = await executeBuiltinTemplate(workspace, {
      ...baseTemplateInfo,
      command: 'vite:library',
      targetDir: '.',
    });

    expect(result).toMatchObject({ exitCode: 0, projectDir: '.' });
    expect(fs.readFileSync(gitFile, 'utf8')).toContain('gitdir:');
    expect(
      JSON.parse(fs.readFileSync(path.join(workspace.rootDir, 'package.json'), 'utf8')),
    ).toEqual({
      name: 'wage-meeting',
    });
    expect(mockRunRemoteTemplateCommand).toHaveBeenCalledOnce();
    expect(mockRunRemoteTemplateCommand.mock.calls[0][2]).toMatchObject({
      command: 'degit',
      args: ['sxzz/tsdown-templates/vite-plus', '.', '--force'],
    });
  });

  it('does not force degit into a directory containing user files', async () => {
    const workspace = makeWorkspaceInfo();
    const userFile = path.join(workspace.rootDir, 'keep.txt');
    fs.writeFileSync(userFile, 'keep me');

    const result = await executeBuiltinTemplate(workspace, {
      ...baseTemplateInfo,
      command: 'vite:library',
      targetDir: '.',
    });

    expect(result.exitCode).toBe(1);
    expect(mockRunRemoteTemplateCommand).not.toHaveBeenCalled();
    expect(fs.readFileSync(userFile, 'utf8')).toBe('keep me');
    expect(mockLogError).toHaveBeenCalledWith(expect.stringContaining('is not empty'));
  });

  it('fails when degit exits successfully without creating a project', async () => {
    const workspace = makeWorkspaceInfo();
    mockRunRemoteTemplateCommand.mockResolvedValue({
      exitCode: 0,
      stdout: Buffer.from('destination directory is not empty, aborting'),
      stderr: Buffer.alloc(0),
    });

    const result = await executeBuiltinTemplate(workspace, {
      ...baseTemplateInfo,
      command: 'vite:library',
      targetDir: '.',
    });

    expect(result.exitCode).toBe(1);
    expect(mockLogError).toHaveBeenCalledWith(
      expect.stringContaining('destination directory is not empty'),
    );
  });

  it('preserves a non-zero degit exit code and reports captured stderr', async () => {
    const workspace = makeWorkspaceInfo();
    mockRunRemoteTemplateCommand.mockResolvedValue({
      exitCode: 7,
      stdout: Buffer.alloc(0),
      stderr: Buffer.from('failed to download template'),
    });

    const result = await executeBuiltinTemplate(workspace, {
      ...baseTemplateInfo,
      command: 'vite:library',
      targetDir: '.',
    });

    expect(result.exitCode).toBe(7);
    expect(mockLogError).toHaveBeenCalledWith(expect.stringContaining('failed to download'));
  });
});
