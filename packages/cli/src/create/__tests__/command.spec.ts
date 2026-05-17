import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const { mockRunCommand } = vi.hoisted(() => ({
  mockRunCommand: vi.fn(),
}));

vi.mock('../../../binding/index.js', () => ({
  runCommand: mockRunCommand,
}));

const { runCommandAndDetectProjectDir } = await import('../command.js');

const tempDirs: string[] = [];

function makeTempDir() {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-create-command-'));
  tempDirs.push(dir);
  return dir;
}

describe('runCommandAndDetectProjectDir', () => {
  beforeEach(() => {
    mockRunCommand.mockReset();
  });

  afterEach(() => {
    for (const dir of tempDirs.splice(0)) {
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });

  it('detects a project created in the current directory', async () => {
    mockRunCommand.mockResolvedValueOnce({
      exitCode: 0,
      pathAccesses: {
        'package.json': { write: true },
      },
    });

    const result = await runCommandAndDetectProjectDir({
      command: 'node',
      args: ['create.js'],
      cwd: '/tmp/workspace',
      envs: {},
    });

    expect(result).toEqual({ exitCode: 0, projectDir: '.' });
  });

  it('prefers a generated child directory over the current directory', async () => {
    mockRunCommand.mockResolvedValueOnce({
      exitCode: 0,
      pathAccesses: {
        'package.json': { write: true },
        'my-app/package.json': { write: true },
      },
    });

    const result = await runCommandAndDetectProjectDir({
      command: 'node',
      args: ['create.js'],
      cwd: '/tmp/workspace',
      envs: {},
    });

    expect(result).toEqual({ exitCode: 0, projectDir: 'my-app' });
  });

  it('returns the parent directory when the project is created at that parent root', async () => {
    const cwd = makeTempDir();
    mockRunCommand.mockResolvedValueOnce({
      exitCode: 0,
      pathAccesses: {
        'package.json': { write: true },
      },
    });

    const result = await runCommandAndDetectProjectDir(
      {
        command: 'node',
        args: ['create.js'],
        cwd,
        envs: {},
      },
      'apps',
    );

    expect(result).toEqual({ exitCode: 0, projectDir: 'apps' });
    expect(mockRunCommand).toHaveBeenCalledWith({
      binName: 'node',
      args: ['create.js'],
      envs: {},
      cwd: path.join(cwd, 'apps'),
    });
  });
});
