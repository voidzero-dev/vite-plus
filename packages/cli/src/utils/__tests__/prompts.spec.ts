import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { downloadPackageManager as downloadPackageManagerBinding } from '../../../binding/index.js';
import { PackageManager } from '../../types/index.ts';
import { runCommandSilently } from '../command.ts';
import {
  downloadPackageManager,
  resolveGitInit,
  runViteInstall,
  shouldIgnoreScriptsForAutoInstall,
} from '../prompts.ts';

vi.mock('../../../binding/index.js', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../../binding/index.js')>()),
  downloadPackageManager: vi.fn(),
}));

vi.mock('../command.ts', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../command.ts')>()),
  runCommandSilently: vi.fn(),
}));

describe('runViteInstall', () => {
  let cwd: string;

  beforeEach(() => {
    cwd = mkdtempSync(join(tmpdir(), 'vp-pnpm-install-'));
    vi.stubEnv('VP_SKIP_INSTALL', undefined);
    vi.stubEnv('VP_VERSION', '1.0.0');
    vi.stubEnv('VP_CLI_BIN', undefined);
  });

  afterEach(() => {
    vi.mocked(runCommandSilently).mockReset();
    vi.unstubAllEnvs();
    rmSync(cwd, { recursive: true, force: true });
  });

  it('requests pnpm resolution events without rewriting logging preferences', async () => {
    const configPath = join(cwd, '.npmrc');
    const config = 'reporter=silent\nloglevel=silent\njson=true\nparseable=true\n';
    writeFileSync(configPath, config);
    vi.mocked(runCommandSilently).mockResolvedValueOnce({
      exitCode: 0,
      stdout: Buffer.from('progress'),
      stderr: Buffer.alloc(0),
    });
    const result = await runViteInstall(cwd, false, ['--no-frozen-lockfile'], {
      packageManager: PackageManager.pnpm,
      packageManagerVersion: '11.22.0',
      effectivePnpmVersion: '9.15.9',
      idleTimeoutMs: 2000,
      silent: true,
    });
    const command = vi.mocked(runCommandSilently).mock.lastCall?.[0];
    expect(command?.args.slice(-8)).toEqual([
      'install',
      '--no-frozen-lockfile',
      '--ignore-scripts',
      '--',
      '--config._reporter=ndjson',
      '--config._loglevel=info',
      '--config._json=false',
      '--config._parseable=false',
    ]);
    expect(command?.idleTimeoutMs).toBe(2000);
    expect(command?.shouldWatchIdle).toBeTypeOf('function');
    expect(result.status).toBe('installed');
    expect(readFileSync(configPath, 'utf8')).toBe(config);
  });

  it.each([
    ['reporter', 'silent'],
    ['loglevel', 'silent'],
    ['json', 'true'],
    ['parseable', 'true'],
    ['reporter', undefined],
  ])('keeps the native old pnpm reporter for %s=%s', async (key, value) => {
    const run = vi.mocked(runCommandSilently);
    run.mockImplementation(async ({ args }) => {
      if (args.includes('install')) {
        return { exitCode: 1, stdout: Buffer.alloc(0), stderr: Buffer.alloc(0) };
      }
      return {
        exitCode: args.includes(key) && value === undefined ? 1 : 0,
        stdout: Buffer.from(args.includes(key) ? (value ?? '') : 'undefined'),
        stderr: Buffer.alloc(0),
      };
    });
    const result = await runViteInstall(cwd, false, ['--no-frozen-lockfile'], {
      packageManager: PackageManager.pnpm,
      packageManagerVersion: '9.15.9',
      effectivePnpmVersion: '9.15.7',
      idleTimeoutMs: 4000,
      silent: true,
    });
    const install = run.mock.lastCall?.[0];
    expect(install?.args.slice(-2)).toEqual(['install', '--no-frozen-lockfile']);
    expect(install?.idleTimeoutMs).toBeUndefined();
    expect(install?.shouldWatchIdle).toBeUndefined();
    expect(result).toMatchObject({ status: 'failed', exitCode: 1 });
  });

  it.each([true, false])('reports the timeout for initialInstall=%s', async (initialInstall) => {
    const failureMessage = initialInstall
      ? 'Initial dependency resolution stopped after 300 seconds without output.'
      : 'Dependency resolution stopped after 300 seconds without output. If pnpm hangs again, ' +
        `update the project's pnpm pin. Run \`vp install --no-frozen-lockfile\` in ${cwd} to retry.`;
    vi.mocked(runCommandSilently).mockResolvedValueOnce({
      exitCode: 124,
      idleTimedOut: true,
      stdout: Buffer.alloc(0),
      stderr: Buffer.alloc(0),
    });

    const stderr = vi.spyOn(console, 'error').mockImplementation(() => {});
    try {
      const result = await runViteInstall(cwd, false, undefined, {
        idleTimeoutMs: 300_000,
        initialInstall,
        silent: true,
      });

      expect(result).toMatchObject({ status: 'failed', exitCode: 124, failureMessage });
      expect(stderr).toHaveBeenCalledTimes(1);
      expect(stderr).toHaveBeenCalledWith(expect.any(String), failureMessage);
    } finally {
      stderr.mockRestore();
    }
  });

  it.each([
    [false, 7],
    [true, 124],
  ] as const)(
    'preserves the silent install exit code with idleTimedOut=%s',
    async (timedOut, exitCode) => {
      const run = vi.mocked(runCommandSilently);
      run.mockResolvedValueOnce({
        exitCode,
        ...(timedOut && { idleTimedOut: true as const }),
        stdout: Buffer.from(
          [
            { name: 'pnpm:progress', level: 'debug', status: 'resolved' },
            {
              name: 'pnpm',
              level: 'error',
              code: 'ELIFECYCLE',
              errno: 124,
              err: { name: 'pnpm', message: 'Exit status 124' },
            },
          ]
            .map((log) => JSON.stringify(log))
            .join('\n'),
        ),
        stderr: Buffer.alloc(0),
      });
      run.mockImplementation(async ({ args }) => ({
        exitCode: 0,
        stdout: Buffer.from(args.includes('reporter') ? 'silent\n' : 'undefined\n'),
        stderr: Buffer.alloc(0),
      }));

      const stdout = vi.spyOn(process.stdout, 'write').mockImplementation(() => true);
      const stderr = vi.spyOn(console, 'error').mockImplementation(() => {});
      try {
        const result = await runViteInstall(cwd, false, undefined, {
          idleTimeoutMs: 300_000,
          packageManager: PackageManager.pnpm,
          packageManagerVersion: '9.15.9',
          silent: true,
        });
        expect(result).toMatchObject({ status: 'failed', exitCode });
        expect(stdout.mock.calls.flat().join('')).not.toContain('Progress:');
        if (timedOut) {
          expect(stderr).toHaveBeenCalledWith(
            expect.any(String),
            expect.stringContaining('vp install --no-frozen-lockfile'),
          );
        }
      } finally {
        stdout.mockRestore();
        stderr.mockRestore();
        run.mockReset();
      }
    },
  );
});

describe('downloadPackageManager', () => {
  it('forwards the target workspace directory to the native binding', async () => {
    const result = {
      name: 'pnpm',
      version: '11.24.0',
      installDir: '/cache/pnpm',
      binPrefix: '/cache/pnpm/bin',
      binPaths: {},
      packageName: 'pnpm',
    };
    vi.mocked(downloadPackageManagerBinding).mockResolvedValueOnce(result);
    expect(
      await downloadPackageManager(PackageManager.pnpm, 'latest', false, true, '/target/workspace'),
    ).toBe(result);
    expect(downloadPackageManagerBinding).toHaveBeenLastCalledWith({
      name: 'pnpm',
      version: 'latest',
      cwd: '/target/workspace',
      expectedHash: undefined,
    });
  });

  it('preserves the integrity hash from a package manager override', async () => {
    vi.mocked(downloadPackageManagerBinding).mockResolvedValueOnce({
      name: 'pnpm',
      version: '10.6.1',
      installDir: '/cache/pnpm',
      binPrefix: '/cache/pnpm/bin',
      binPaths: {},
      packageName: 'pnpm',
    });
    await downloadPackageManager(
      PackageManager.pnpm,
      '10.6.1',
      false,
      true,
      '/target/workspace',
      'sha512.0123456789abcdef',
    );
    expect(downloadPackageManagerBinding).toHaveBeenLastCalledWith({
      name: 'pnpm',
      version: '10.6.1',
      cwd: '/target/workspace',
      expectedHash: 'sha512.0123456789abcdef',
    });
  });
});

describe('resolveGitInit', () => {
  it('never initializes git when adding a package to an existing monorepo', async () => {
    // A sub-package shares the monorepo's repository, so git setup must be
    // skipped even if `--git` is forced — and no interactive prompt is shown.
    expect(await resolveGitInit({ git: true, interactive: true }, true)).toBe(false);
    expect(await resolveGitInit({ git: undefined, interactive: true }, true)).toBe(false);
    expect(await resolveGitInit({ git: undefined, interactive: false }, true)).toBe(false);
  });

  it('respects the git option for a new standalone project', async () => {
    expect(await resolveGitInit({ git: true, interactive: false }, false)).toBe(true);
    expect(await resolveGitInit({ git: false, interactive: false }, false)).toBe(false);
    // non-interactive default is no git
    expect(await resolveGitInit({ git: undefined, interactive: false }, false)).toBe(false);
  });
});

describe('shouldIgnoreScriptsForAutoInstall', () => {
  it('returns true for pnpm >= 11.0.0', () => {
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.pnpm, '11.0.0')).toBe(true);
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.pnpm, '11.0.8')).toBe(true);
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.pnpm, '12.0.0')).toBe(true);
  });

  it('returns false for pnpm < 11.0.0', () => {
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.pnpm, '10.33.2')).toBe(false);
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.pnpm, '9.5.0')).toBe(false);
  });

  it('returns false for non-pnpm package managers', () => {
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.npm, '11.0.0')).toBe(false);
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.yarn, '11.0.0')).toBe(false);
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.bun, '11.0.0')).toBe(false);
  });

  it('returns false when version is unknown or unparsable', () => {
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.pnpm, undefined)).toBe(false);
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.pnpm, 'latest')).toBe(false);
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.pnpm, '')).toBe(false);
  });

  it('returns false when packageManager is undefined', () => {
    expect(shouldIgnoreScriptsForAutoInstall(undefined, '11.0.0')).toBe(false);
  });

  it('coerces non-strict semver pnpm versions', () => {
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.pnpm, 'v11.0.0')).toBe(true);
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.pnpm, '11')).toBe(true);
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.pnpm, '10')).toBe(false);
  });
});
