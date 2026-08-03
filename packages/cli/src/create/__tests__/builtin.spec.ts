import { describe, expect, it, vi } from 'vitest';

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
