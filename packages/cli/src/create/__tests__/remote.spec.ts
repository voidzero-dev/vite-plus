import { describe, expect, it, vi } from 'vitest';

import type { WorkspaceInfo } from '../../types/index.ts';
import { runCommandSilently } from '../../utils/command.ts';
import { runRemoteTemplateCommand } from '../templates/remote.ts';
import type { TemplateInfo } from '../templates/types.ts';

vi.mock('../../utils/command.ts', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../utils/command.ts')>()),
  runCommand: vi.fn(),
  runCommandSilently: vi.fn(async () => ({ exitCode: 0 })),
}));

describe('runRemoteTemplateCommand', () => {
  // `vp dlx` resolves the selected package manager itself, so a Vite+ shim earlier on PATH cannot replace it.
  it('runs the template through vp dlx with the selected package manager', async () => {
    vi.stubEnv('VP_CLI_BIN', 'vp');
    const workspaceInfo = {
      packageManager: 'pnpm',
      packageManagerVersion: '10.32.0',
    } as unknown as WorkspaceInfo;
    const templateInfo = {
      command: 'create-foo',
      args: ['app'],
      envs: {},
    } as unknown as TemplateInfo;

    await runRemoteTemplateCommand(workspaceInfo, '/workspace', templateInfo, false, true);
    vi.unstubAllEnvs();

    expect(runCommandSilently).toHaveBeenLastCalledWith({
      command: 'vp',
      args: ['dlx', '--', 'create-foo', 'app'],
      cwd: '/workspace',
      envs: { VP_PACKAGE_MANAGER: 'pnpm@10.32.0' },
    });
  });

  // The generator may run inside the new workspace, whose registry could resolve a tag differently.
  it('forwards the resolved override instead of the tag the user gave', async () => {
    vi.stubEnv('VP_CLI_BIN', 'vp');
    const workspaceInfo = {
      packageManager: 'pnpm',
      packageManagerVersion: '10.18.0',
      packageManagerOverride: 'pnpm@10.18.0+sha512.abc',
    } as unknown as WorkspaceInfo;
    const templateInfo = {
      command: 'create-foo',
      args: [],
      envs: { VP_PACKAGE_MANAGER: 'pnpm@latest+sha512.abc' },
    } as unknown as TemplateInfo;

    await runRemoteTemplateCommand(workspaceInfo, '/workspace', templateInfo, false, true);
    vi.unstubAllEnvs();

    expect(runCommandSilently).toHaveBeenLastCalledWith(
      expect.objectContaining({ envs: { VP_PACKAGE_MANAGER: 'pnpm@10.18.0+sha512.abc' } }),
    );
  });
});
