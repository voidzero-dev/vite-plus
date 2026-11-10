import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { describe, expect, it, vi } from 'vitest';

import { VITE_PLUS_VERSION } from '../../utils/constants.ts';
import { readJsonFile } from '../../utils/json.ts';
import { executeBuiltinTemplate } from '../templates/builtin.js';

const { mockLogError } = vi.hoisted(() => ({ mockLogError: vi.fn() }));

vi.mock('../templates/remote.js', () => ({
  runRemoteTemplateCommand: vi.fn(),
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

describe('executeBuiltinTemplate', () => {
  it('returns exitCode 1 for unknown vite: template', async () => {
    const { runRemoteTemplateCommand } = await import('../templates/remote.js');

    const result = await executeBuiltinTemplate(workspaceInfo, {
      ...baseTemplateInfo,
      command: 'vite:test',
    });

    expect(result.exitCode).toBe(1);
    expect(runRemoteTemplateCommand).not.toHaveBeenCalled();
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
});

describe('builtin library toolchain version', () => {
  it.each([false, true])(
    'aligns the downloaded template with the CLI (monorepo: %s)',
    async (isMonorepo) => {
      const rootDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-library-template-'));
      try {
        const { runRemoteTemplateCommand } = await import('../templates/remote.js');
        vi.mocked(runRemoteTemplateCommand).mockImplementationOnce(async () => {
          const dir = path.join(rootDir, baseTemplateInfo.targetDir);
          fs.mkdirSync(dir, { recursive: true });
          fs.writeFileSync(
            path.join(dir, 'package.json'),
            JSON.stringify({
              name: 'template-name',
              devDependencies: { 'vite-plus': '^0.2.4', typescript: '^7.0.2' },
              peerDependencies: { react: '^19' },
            }),
          );
          return { exitCode: 0 };
        });
        const result = await executeBuiltinTemplate(
          {
            ...workspaceInfo,
            rootDir,
            isMonorepo,
            parentDirs: [],
            packages: [],
            downloadPackageManager: { binPrefix: '' },
          },
          { ...baseTemplateInfo, command: 'vite:library' },
          { silent: true },
        );
        expect(result.exitCode).toBe(0);
        expect(
          readJsonFile(path.join(rootDir, baseTemplateInfo.targetDir, 'package.json')),
        ).toEqual({
          name: baseTemplateInfo.packageName,
          devDependencies: { 'vite-plus': VITE_PLUS_VERSION, typescript: '^7.0.2' },
          peerDependencies: { react: '^19' },
        });
      } finally {
        fs.rmSync(rootDir, { recursive: true, force: true });
      }
    },
  );
});
