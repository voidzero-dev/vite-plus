import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, describe, expect, it, vi } from 'vitest';

import { PackageManager, type WorkspaceInfo } from '../../types/index.ts';
import { VITE_PLUS_VERSION } from '../../utils/constants.ts';
import { readJsonFile } from '../../utils/json.ts';
import { executeMonorepoTemplate } from '../templates/monorepo.ts';
import { runRemoteTemplateCommand } from '../templates/remote.ts';
import { BuiltinTemplate, TemplateType } from '../templates/types.ts';

vi.mock('../templates/remote.ts', () => ({ runRemoteTemplateCommand: vi.fn() }));

let rootDir: string;
afterEach(() => {
  vi.resetAllMocks();
  if (rootDir) {
    fs.rmSync(rootDir, { recursive: true, force: true });
  }
});

describe('monorepo library toolchain version', () => {
  it.each([
    [PackageManager.npm, '12.2.0', VITE_PLUS_VERSION],
    [PackageManager.pnpm, '12.8.1', 'catalog:'],
    [PackageManager.yarn, '4.18.1', 'catalog:'],
    [PackageManager.bun, '1.4.2', 'catalog:'],
  ])('aligns the downloaded library for %s', async (packageManager, version, expected) => {
    rootDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-monorepo-library-version-'));
    const workspace: WorkspaceInfo = {
      rootDir,
      isMonorepo: false,
      monorepoScope: '',
      workspacePatterns: [],
      parentDirs: [],
      packages: [],
      packageManager,
      packageManagerVersion: version,
      downloadPackageManager: {
        name: packageManager,
        packageName: packageManager,
        version,
        installDir: '',
        binPrefix: '',
      },
    };
    vi.mocked(runRemoteTemplateCommand).mockImplementation(async (_workspace, cwd, template) => {
      const isLibrary = template.command === 'degit';
      const dir = path.join(cwd, isLibrary ? 'packages/utils' : 'apps/website');
      fs.mkdirSync(dir, { recursive: true });
      fs.writeFileSync(
        path.join(dir, 'package.json'),
        JSON.stringify({
          name: 'template',
          devDependencies: isLibrary
            ? { 'vite-plus': '^0.2.4', typescript: '^7.0.2', bumpp: '^11.1.0' }
            : { vite: '^8.0.0', typescript: '^7.0.2' },
        }),
      );
      return { exitCode: 0 };
    });

    const result = await executeMonorepoTemplate(
      workspace,
      {
        command: BuiltinTemplate.monorepo,
        type: TemplateType.builtin,
        packageName: 'my-monorepo',
        targetDir: 'project',
        interactive: false,
        args: [],
        envs: {},
      },
      { silent: true },
    );

    expect(result.exitCode).toBe(0);
    const pkg = readJsonFile(path.join(rootDir, 'project/packages/utils/package.json'));
    expect(pkg).toMatchObject({
      name: 'utils',
      devDependencies: { 'vite-plus': expected, bumpp: '^11.1.0' },
    });
  });
});
