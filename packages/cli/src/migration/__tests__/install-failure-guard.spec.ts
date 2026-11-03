import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { handleInstallResult } from '../install-result.ts';
import { createMigrationReport } from '../report.ts';

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

  it.each(['skipped', 'formatted'] as const)('stays silent for a %s install', (status) => {
    const report = createMigrationReport();

    expect(handleInstallResult({ status, durationMs: 1234 }, rootDir, report)).toBe(0);
    expect(report.warnings).toEqual([]);
    expect(process.exitCode).toBeUndefined();
    expect(consoleError).not.toHaveBeenCalled();
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
