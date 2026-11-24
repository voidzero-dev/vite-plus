import type {
  chmodSync as chmodSyncImplementation,
  mkdirSync as mkdirSyncImplementation,
  rmSync as rmSyncImplementation,
} from 'node:fs';
import { copyFileSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import { afterEach, beforeEach, expect, test, vi } from 'vite-plus/test';

import { publishNpmPackageFromEnv } from '../../../../.github/scripts/publish-npm-package.ts';
import type { waitForNpmPackagesFromEnv as waitForNpmPackagesFromEnvImplementation } from '../../../../.github/scripts/wait-for-npm-packages.ts';
import type { validateInstallLockfile as validateInstallLockfileImplementation } from '../../generate-install-lockfile.ts';
import type { editJsonFile as editJsonFileImplementation } from '../utils/json.ts';

vi.mock('node:fs', () => ({
  copyFileSync: vi.fn<typeof copyFileSync>(),
  existsSync: (path: string) => !path.endsWith('rust-cli-artifacts'),
  chmodSync: vi.fn<typeof chmodSyncImplementation>(),
  mkdirSync: vi.fn<typeof mkdirSyncImplementation>(),
  readFileSync: vi.fn<typeof readFileSync>(),
  rmSync: vi.fn<typeof rmSyncImplementation>(),
  writeFileSync: vi.fn<typeof writeFileSync>(),
}));
vi.mock('node:fs/promises', () => ({ readdir: async () => [] }));
vi.mock('@napi-rs/cli', () => ({
  NapiCli: class {
    async createNpmDirs() {}
    async artifacts() {}
    async prePublish() {}
  },
  parseTriple: () => ({ platform: 'darwin', arch: 'arm64', platformArchABI: 'darwin-arm64' }),
}));
vi.mock('../../package.json', () => ({
  default: {
    napi: { targets: ['aarch64-apple-darwin'], packageName: '@voidzero-dev/vite-plus' },
  },
}));
vi.mock('../utils/json.ts', () => ({
  editJsonFile: vi.fn<typeof editJsonFileImplementation>(),
  readJsonFile: () => ({
    version: '1.4.0',
    optionalDependencies: { '@voidzero-dev/vite-plus-darwin-arm64': '1.4.0' },
  }),
}));
vi.mock('../../generate-install-lockfile.ts', () => ({
  INSTALL_LOCKFILE: 'vp-install-lock.yaml',
  validateInstallLockfile: vi.fn<typeof validateInstallLockfileImplementation>(),
}));
vi.mock('../../../../.github/scripts/publish-npm-package.ts', () => ({
  publishNpmPackageFromEnv: vi.fn<typeof publishNpmPackageFromEnv>(async () => 'published'),
}));
vi.mock('../../../../.github/scripts/wait-for-npm-packages.ts', () => ({
  waitForNpmPackagesFromEnv: vi.fn<typeof waitForNpmPackagesFromEnvImplementation>(),
}));

const originalArgv = process.argv;

beforeEach(() => {
  vi.resetModules();
  vi.clearAllMocks();
  vi.stubEnv('VERSION', '1.4.0');
  vi.spyOn(console, 'log').mockImplementation(() => {});
});

afterEach(() => {
  process.argv = originalArgv;
  vi.unstubAllEnvs();
  vi.restoreAllMocks();
});

test.each([
  { mode: 'npm', nodeVersion: '22.23.4' },
  { mode: 'pkg-pr-new', nodeVersion: '24.22.1' },
])('embeds the repository Node pin in $mode platform packages', async ({ mode, nodeVersion }) => {
  vi.mocked(readFileSync).mockImplementation((path) =>
    String(path).endsWith('.node-version') ? `${nodeVersion}\n` : 'lockfile',
  );
  process.argv = ['node', 'publish-native-addons.ts', '--mode', mode];
  if (mode === 'npm') {
    process.argv.push('--phase', 'cli');
  }
  await import('../../publish-native-addons.ts');

  expect(readFileSync).toHaveBeenCalledWith(expect.stringMatching(/[/\\]\.node-version$/), 'utf8');
  expect(writeFileSync).toHaveBeenCalledOnce();
  const [path, contents] = vi.mocked(writeFileSync).mock.calls[0];
  expect(path).toEqual(expect.stringContaining(join('cli-npm', 'darwin-arm64', 'package.json')));
  if (typeof contents !== 'string') {
    throw new TypeError('Expected package.json text');
  }
  expect(JSON.parse(contents)).toMatchObject({
    name: '@voidzero-dev/vite-plus-cli-darwin-arm64',
    version: '1.4.0',
    vpNodeVersion: nodeVersion,
  });
  const expectedCall = [
    expect.objectContaining({ vpNodeVersion: nodeVersion }),
    'npm',
    expect.any(Array),
    expect.any(String),
  ];
  expect(vi.mocked(publishNpmPackageFromEnv).mock.calls).toEqual(
    mode === 'npm' ? [expectedCall] : [],
  );
});

test.each(['', 'lts', '24', '24.22', 'v24.22.1', '024.22.1', '24.22.1-rc.1', '24.22.1+build'])(
  'rejects an invalid Node pin before packaging: %j',
  async (pin) => {
    process.argv = ['node', 'publish-native-addons.ts', '--mode', 'npm', '--phase', 'cli'];
    vi.mocked(readFileSync).mockReturnValue(pin);
    await expect(import('../../publish-native-addons.ts')).rejects.toThrow(
      '.node-version must contain an exact stable Node.js version',
    );
    expect(copyFileSync).not.toHaveBeenCalled();
    expect(writeFileSync).not.toHaveBeenCalled();
    expect(publishNpmPackageFromEnv).not.toHaveBeenCalled();
  },
);
