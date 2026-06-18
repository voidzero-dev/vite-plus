import path from 'node:path';

import { beforeEach, describe, expect, it, vi } from 'vitest';

import { executeBuiltinTemplate } from '../templates/builtin.js';

const { mockLogError, mockSetPackageName } = vi.hoisted(() => ({
  mockLogError: vi.fn(),
  mockSetPackageName: vi.fn(),
}));

vi.mock('../templates/remote.js', () => ({
  runRemoteTemplateCommand: vi.fn(),
}));

vi.mock('@voidzero-dev/vite-plus-prompts', () => ({
  log: { error: mockLogError },
}));

vi.mock('../utils.js', () => ({
  setPackageName: mockSetPackageName,
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
  beforeEach(async () => {
    mockLogError.mockClear();
    mockSetPackageName.mockClear();
    const { runRemoteTemplateCommand } = await import('../templates/remote.js');
    vi.mocked(runRemoteTemplateCommand).mockReset();
    vi.mocked(runRemoteTemplateCommand).mockResolvedValue({ exitCode: 0 });
  });

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
    await executeBuiltinTemplate(
      workspaceInfo,
      { ...baseTemplateInfo, command: 'vite:test' },
      { silent: true },
    );

    expect(mockLogError).not.toHaveBeenCalled();
  });

  it('runs create-vite from the parent directory for nested monorepo applications', async () => {
    const { runRemoteTemplateCommand } = await import('../templates/remote.js');

    const result = await executeBuiltinTemplate(workspaceInfo, {
      ...baseTemplateInfo,
      command: 'vite:application',
      packageName: '@scope/temperature-symbol',
      targetDir: 'apps/temperature-symbol',
    });

    expect(result).toEqual({ exitCode: 0, projectDir: 'apps/temperature-symbol' });
    expect(runRemoteTemplateCommand).toHaveBeenCalledWith(
      workspaceInfo,
      path.join('/tmp/workspace', 'apps'),
      expect.objectContaining({
        command: 'create-vite@latest',
        args: ['temperature-symbol', '--no-interactive'],
      }),
      false,
      false,
    );
    expect(mockSetPackageName).toHaveBeenCalledWith(
      path.join('/tmp/workspace', 'apps', 'temperature-symbol'),
      '@scope/temperature-symbol',
    );
  });

  it('preserves current-directory application targets', async () => {
    const { runRemoteTemplateCommand } = await import('../templates/remote.js');

    const result = await executeBuiltinTemplate(workspaceInfo, {
      ...baseTemplateInfo,
      command: 'vite:application',
      packageName: 'workspace',
      targetDir: '.',
    });

    expect(result).toEqual({ exitCode: 0, projectDir: '.' });
    expect(runRemoteTemplateCommand).toHaveBeenCalledWith(
      workspaceInfo,
      '/tmp/workspace',
      expect.objectContaining({
        command: 'create-vite@latest',
        args: ['.', '--no-interactive'],
      }),
      false,
      false,
    );
    expect(mockSetPackageName).toHaveBeenCalledWith('/tmp/workspace', 'workspace');
  });
});
