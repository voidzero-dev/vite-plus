import { copyFileSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import { afterEach, beforeEach, expect, test, vi } from 'vite-plus/test';

import { publishNpmPackageFromEnv } from '../../../../.github/scripts/publish-npm-package.ts';

vi.mock('node:fs', () => ({
  copyFileSync: vi.fn(),
  existsSync: (path: string) => !path.endsWith('rust-cli-artifacts'),
  chmodSync: vi.fn(),
  mkdirSync: vi.fn(),
  readFileSync: vi.fn(),
  rmSync: vi.fn(),
  writeFileSync: vi.fn(),
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
  editJsonFile: vi.fn(),
  readJsonFile: () => ({
    version: '1.4.0',
    optionalDependencies: { '@voidzero-dev/vite-plus-darwin-arm64': '1.4.0' },
  }),
}));
vi.mock('../../generate-install-lockfile.ts', () => ({
  INSTALL_LOCKFILE: 'vp-install-lock.yaml',
  validateInstallLockfile: vi.fn(),
}));
vi.mock('../../../../.github/scripts/publish-npm-package.ts', () => ({
  publishNpmPackageFromEnv: vi.fn(async () => 'published'),
}));
vi.mock('../../../../.github/scripts/wait-for-npm-packages.ts', () => ({
  waitForNpmPackagesFromEnv: vi.fn(),
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
  expect(JSON.parse(contents as string)).toMatchObject({
    name: '@voidzero-dev/vite-plus-cli-darwin-arm64',
    version: '1.4.0',
    vpNodeVersion: nodeVersion,
  });
  if (mode === 'npm') {
    expect(publishNpmPackageFromEnv).toHaveBeenCalledWith(
      expect.objectContaining({ vpNodeVersion: nodeVersion }),
      'npm',
      expect.any(Array),
      expect.any(String),
    );
  } else {
    expect(publishNpmPackageFromEnv).not.toHaveBeenCalled();
  }
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
