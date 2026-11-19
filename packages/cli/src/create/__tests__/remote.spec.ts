import { describe, expect, it, vi } from 'vitest';

import type { WorkspaceInfo } from '../../types/index.ts';
import { runCommandSilently } from '../../utils/command.ts';
import { runRemoteTemplateCommand } from '../templates/remote.ts';
import type { TemplateInfo } from '../templates/types.ts';

vi.mock('../../utils/command.ts', () => ({
  runCommand: vi.fn(),
  runCommandSilently: vi.fn(async () => ({ exitCode: 0 })),
}));

describe('runRemoteTemplateCommand', () => {
  // A Vite+ shim ahead of the selected system pnpm on PATH would run a different pnpm by name.
  it('launches the runner from the selected installation', async () => {
    const workspaceInfo = {
      packageManager: 'pnpm',
      packageManagerVersion: '10.32.0',
      downloadPackageManager: { binPaths: { pnpm: '/usr/local/bin/pnpm' } },
    } as unknown as WorkspaceInfo;
    const templateInfo = {
      command: 'create-foo',
      args: ['app'],
      envs: {},
    } as unknown as TemplateInfo;

    await runRemoteTemplateCommand(workspaceInfo, '/workspace', templateInfo, false, true);

    expect(runCommandSilently).toHaveBeenLastCalledWith({
      command: '/usr/local/bin/pnpm',
      args: ['dlx', 'create-foo', 'app'],
      cwd: '/workspace',
      envs: {},
    });
  });
});
