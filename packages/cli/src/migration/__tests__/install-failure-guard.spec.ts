import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const binPath = path.resolve(__dirname, '..', 'bin.ts');

/**
 * `runViteInstall` returns `{ status: 'failed', exitCode }` (it does not throw)
 * when the install process exits non-zero. Earlier versions of `migration/bin.ts`
 * only summed `installSummary.durationMs` and reported success regardless of
 * `status`, which left the project's `node_modules` desynced from a freshly
 * mutated `package.json` (network/auth failures, registry blips, pnpm lockfile
 * conflicts after override resolution) while still printing
 * "Dependencies installed" to the user.
 *
 * Two install sites need this handling:
 *   1. The full migration path (`executeMigrationPlan`'s final reinstall, run
 *      AFTER manifest/source rewrites land).
 *   2. The early-return path (`main` when `vite-plus` is already a dep but a
 *      stale-wrapper repair or ESLint/Prettier migration touches package.json).
 *
 * Both paths must funnel install results through `handleInstallResult` so that
 * failures warn the user, append to `report.warnings`, and flip
 * `process.exitCode`. This is a guard test — if a future refactor drops the
 * helper or stops calling it from either path, this fails loudly so reviewers
 * can re-evaluate the install-failure UX before it ships.
 */
describe('migration install failure handling', () => {
  const binSource = fs.readFileSync(binPath, 'utf8');

  describe('handleInstallResult helper', () => {
    it('defines a `handleInstallResult` function in bin.ts', () => {
      expect(binSource).toMatch(/function handleInstallResult\s*\(/);
    });

    it('branches on `installSummary.status === "installed"` and credits duration', () => {
      expect(binSource).toMatch(/installSummary\.status === 'installed'/);
      expect(binSource).toMatch(/return installSummary\.durationMs/);
    });

    it('branches on `installSummary.status === "failed"` and warns + flips exitCode', () => {
      expect(binSource).toMatch(/installSummary\.status === 'failed'/);
      expect(binSource).toMatch(/warnMsg\(/);
      expect(binSource).toMatch(/report\.warnings\.push\(/);
      expect(binSource).toMatch(/process\.exitCode\s*=/);
    });
  });

  describe('full migration path (executeMigrationPlan)', () => {
    it('reconciles `finalInstallSummary` through `handleInstallResult`', () => {
      // The full-path final reinstall must run through the helper so a failed
      // install after manifest/source rewrites does not silently report a
      // desynced project as a successful migration.
      expect(binSource).toMatch(/finalInstallSummary[\s\S]{0,1500}handleInstallResult\(/);
    });

    it('does NOT add `finalInstallSummary.durationMs` directly to the return value', () => {
      // The buggy round-7 version returned
      //   `initialInstallSummary.durationMs + finalInstallSummary.durationMs`
      // unconditionally. The fix routes `finalInstallSummary` through the
      // helper which returns 0 on failure. Catch any regression that re-adds
      // the raw `.durationMs` access.
      expect(binSource).not.toMatch(/finalInstallSummary\.durationMs/);
    });
  });

  describe('early-return path (main, hasVitePlusDependency branch)', () => {
    it('reconciles `installSummary` through `handleInstallResult`', () => {
      expect(binSource).toMatch(
        /const installSummary = await runViteInstall\([\s\S]{0,1500}handleInstallResult\(/,
      );
    });
  });
});
