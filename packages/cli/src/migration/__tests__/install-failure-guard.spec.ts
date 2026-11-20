import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { downloadPackageManager, getPackageManagerOverride } from '../../../binding/index.js';
import { PackageManager } from '../../types/index.ts';
import { runCommandSilently } from '../../utils/command.ts';
import { getMigrationInstallIdleTimeoutMs, handleInstallResult } from '../install-result.ts';
import { createMigrationReport } from '../report.ts';

vi.mock(import('../../../binding/index.js'), async (importOriginal) => ({
  ...(await importOriginal()),
  downloadPackageManager: vi.fn(),
  getPackageManagerOverride: vi.fn(),
}));

vi.mock(import('../../utils/command.ts'), async (importOriginal) => ({
  ...(await importOriginal()),
  runCommandSilently: vi.fn(),
}));

const here = path.dirname(fileURLToPath(import.meta.url));
const binPath = path.resolve(here, '..', 'bin.ts');

/**
 * `runViteInstall` returns `{ status: 'failed', exitCode }` (it does not throw)
 * when the install process exits non-zero. Earlier versions of `migration/bin.ts`
 * only summed `installSummary.durationMs` and reported success regardless of
 * `status`, which left the project's `node_modules` desynced from a freshly
 * mutated `package.json` while still printing "Dependencies installed".
 */
describe('handleInstallResult', () => {
  const rootDir = '/project';
  const failureMessage = (exitCode: number) =>
    `Dependency installation failed (exit code ${exitCode}). Run \`vp install\` manually in ${rootDir} to resync node_modules.`;
  let originalExitCode: typeof process.exitCode;
  let consoleError: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    originalExitCode = process.exitCode;
    process.exitCode = undefined;
    consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    process.exitCode = originalExitCode;
    consoleError.mockRestore();
  });

  it('credits the install duration on success', () => {
    const report = createMigrationReport();

    expect(
      handleInstallResult({ status: 'installed', durationMs: 1234, exitCode: 0 }, rootDir, report),
    ).toBe(1234);
    expect(report.warnings).toEqual([]);
    expect(process.exitCode).toBeUndefined();
    expect(consoleError).not.toHaveBeenCalled();
  });

  it.each([
    [7, 7],
    [undefined, 1],
  ])('warns and sets exitCode when the install fails with exit code %s', (exitCode, expected) => {
    const report = createMigrationReport();

    expect(
      handleInstallResult({ status: 'failed', durationMs: 1234, exitCode }, rootDir, report),
    ).toBe(0);
    expect(report.warnings).toEqual([failureMessage(expected)]);
    expect(process.exitCode).toBe(expected);
    expect(consoleError).toHaveBeenCalledWith(expect.any(String), failureMessage(expected));
  });

  it('warns without setting exitCode for a best-effort install that fails', () => {
    const report = createMigrationReport();

    expect(
      handleInstallResult({ status: 'failed', durationMs: 1234, exitCode: 7 }, rootDir, report, {
        propagateExitCode: false,
      }),
    ).toBe(0);
    expect(report.warnings).toEqual([failureMessage(7)]);
    expect(process.exitCode).toBeUndefined();
    expect(consoleError).toHaveBeenCalledWith(expect.any(String), failureMessage(7));
  });

  it('keeps an already reported timeout diagnostic and exit code', () => {
    const report = createMigrationReport();
    const message = 'Dependency resolution stopped after 300 seconds without output.';
    expect(
      handleInstallResult(
        { status: 'failed', durationMs: 300_000, exitCode: 124, failureMessage: message },
        rootDir,
        report,
      ),
    ).toBe(0);
    expect(report.warnings).toEqual([message]);
    expect(process.exitCode).toBe(124);
    expect(consoleError).not.toHaveBeenCalled();
  });

  it.each(['skipped', 'formatted'] as const)('stays silent for a %s install', (status) => {
    const report = createMigrationReport();

    expect(handleInstallResult({ status, durationMs: 1234 }, rootDir, report)).toBe(0);
    expect(report.warnings).toEqual([]);
    expect(process.exitCode).toBeUndefined();
    expect(consoleError).not.toHaveBeenCalled();
  });
});

describe('getMigrationInstallIdleTimeoutMs', () => {
  const cwd = '/project';

  beforeEach(() => {
    vi.resetAllMocks();
    vi.stubEnv('VP_SKIP_INSTALL', undefined);
    vi.stubEnv('VP_CLI_BIN', undefined);
  });

  afterEach(() => vi.unstubAllEnvs());

  it('bounds non-interactive pnpm 9 installs only', async () => {
    expect(await getMigrationInstallIdleTimeoutMs(cwd, false, PackageManager.pnpm, '9.9.0')).toBe(
      300_000,
    );
    expect(
      await getMigrationInstallIdleTimeoutMs(cwd, true, PackageManager.pnpm, '9.9.0'),
    ).toBeUndefined();
    expect(
      await getMigrationInstallIdleTimeoutMs(cwd, false, PackageManager.pnpm, '12.21.0'),
    ).toBeUndefined();
    expect(
      await getMigrationInstallIdleTimeoutMs(cwd, false, PackageManager.npm, '9.9.0'),
    ).toBeUndefined();
  });

  it('leaves a pnpm 9 project unguarded when the override selects Yarn', async () => {
    vi.stubEnv('VP_CLI_BIN', '/global/vp');
    vi.mocked(getPackageManagerOverride).mockReturnValue({ name: 'yarn', version: '4.10.3' });

    expect(
      await getMigrationInstallIdleTimeoutMs(cwd, false, PackageManager.pnpm, '9.15.9'),
    ).toBeUndefined();
    expect(runCommandSilently).not.toHaveBeenCalled();
  });

  it.each([
    ['12.21.0', '9.15.9', 300_000],
    ['9.15.9', '12.21.0', undefined],
  ])(
    'uses the global launcher selection over pnpm %s from the workspace',
    async (pin, version, expected) => {
      vi.stubEnv('VP_CLI_BIN', '/global/vp');
      vi.mocked(getPackageManagerOverride).mockReturnValue({ name: 'pnpm', version: 'default' });
      vi.mocked(runCommandSilently).mockResolvedValue({
        exitCode: 0,
        stdout: Buffer.from(JSON.stringify({ package_manager: { name: 'pnpm', version } })),
        stderr: Buffer.alloc(0),
      });

      expect(await getMigrationInstallIdleTimeoutMs(cwd, false, PackageManager.pnpm, pin)).toBe(
        expected,
      );
      expect(runCommandSilently).toHaveBeenCalledWith(
        expect.objectContaining({
          command: '/global/vp',
          args: ['env', 'current', 'pm', '--json'],
          cwd,
        }),
      );
    },
  );

  it('uses the resolved pnpm override for the local launcher', async () => {
    vi.mocked(getPackageManagerOverride).mockReturnValue({ name: 'pnpm', version: 'latest-9' });
    vi.mocked(downloadPackageManager).mockResolvedValue({
      name: 'pnpm',
      version: '9.15.9',
      installDir: '/pnpm',
      binPrefix: '/pnpm/bin',
      binPaths: { pnpm: '/pnpm/bin/pnpm' },
      packageName: 'pnpm',
    });

    expect(await getMigrationInstallIdleTimeoutMs(cwd, false, PackageManager.pnpm, '12.21.0')).toBe(
      300_000,
    );
    expect(downloadPackageManager).toHaveBeenCalledWith({ name: 'pnpm', version: 'latest-9', cwd });
  });
});

// `bin.ts` runs `main()` on import, so its install sites cannot be exercised
// from a unit test. These source guards keep them routed through the helper.
describe('migration bin install sites', () => {
  const binSource = fs.readFileSync(binPath, 'utf8');

  it('routes every install result through `handleInstallResult`', () => {
    const installs = Array.from(
      binSource.matchAll(/const (\w+) = await runViteInstall\(/g),
      (match) => match[1],
    );
    // Every call is captured, so an install whose result is ignored fails here.
    expect(installs).toHaveLength(binSource.match(/runViteInstall\(/g)?.length ?? 0);
    expect(installs.length).toBeGreaterThan(0);
    for (const name of installs) {
      expect(binSource).toMatch(new RegExp(`handleInstallResult\\(\\s*${name},`));
      // Reading the duration directly would credit failed installs.
      expect(binSource).not.toContain(`${name}.durationMs`);
    }
  });

  it('defers Vitest compatibility writes until all tool migration abort gates pass', () => {
    const full = binSource.slice(
      binSource.indexOf('async function executeMigrationPlan('),
      binSource.indexOf('async function main('),
    );
    const existing = binSource.slice(binSource.indexOf('async function main('));
    for (const branch of [full, existing]) {
      expect(branch).not.toContain('applyVitestV5Migration(vitestV5Plan)');
      const apply = branch.indexOf('vitestV5Plan = applyRefreshedVitestV5Migration(vitestV5Plan)');
      expect(apply).toBeGreaterThan(branch.indexOf('ESLint migration failed.'));
      expect(apply).toBeGreaterThan(branch.indexOf('Prettier migration failed.'));
      expect(apply).toBeGreaterThan(branch.indexOf('Complete the tsup migration manually'));
    }
    expect(full.indexOf('applyRefreshedVitestV5Migration(vitestV5Plan)')).toBeGreaterThan(
      full.indexOf('checkVitestVersion(workspaceInfo.rootDir)'),
    );
  });
});
