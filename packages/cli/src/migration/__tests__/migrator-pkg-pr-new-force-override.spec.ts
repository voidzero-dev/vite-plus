import { describe, expect, it, vi } from 'vitest';

import { PackageManager } from '../../types/index.js';

const PKG_PR_NEW_URL =
  'https://pkg.pr.new/voidzero-dev/vite-plus@0c515e3fbf5c140db35280d700df0bd600838617';
const CORE_URL =
  'https://pkg.pr.new/voidzero-dev/vite-plus/@voidzero-dev/vite-plus-core@0c515e3fbf5c140db35280d700df0bd600838617';

// pkg.pr.new force-override runs (`VP_VERSION=https://pkg.pr.new/...`) make
// VITE_PLUS_VERSION a protocol-pinned http URL instead of the usual semver. The
// shared migrator spec mocks VITE_PLUS_VERSION to `latest`, which hides this
// case, so reproduce it in an isolated module with the URL-shaped version.
vi.mock('../../utils/constants.js', async (importOriginal) => {
  const mod = await importOriginal<typeof import('../../utils/constants.js')>();
  return {
    ...mod,
    VITE_PLUS_VERSION: PKG_PR_NEW_URL,
    VITE_PLUS_OVERRIDE_PACKAGES: {
      ...mod.VITE_PLUS_OVERRIDE_PACKAGES,
      vite: CORE_URL,
    },
  };
});

const { rewritePackageJson } = await import('../migrator.js');

function withForceOverride(fn: () => void): void {
  const saved = process.env.VP_FORCE_MIGRATE;
  process.env.VP_FORCE_MIGRATE = '1';
  try {
    fn();
  } finally {
    if (saved === undefined) {
      delete process.env.VP_FORCE_MIGRATE;
    } else {
      process.env.VP_FORCE_MIGRATE = saved;
    }
  }
}

describe('rewritePackageJson under pkg.pr.new force-override (URL VITE_PLUS_VERSION)', () => {
  // Regression: force-override re-pins vite-plus to the pkg.pr.new URL up front,
  // so the catalog-supporting rewrite must normalize the direct dep back to
  // `catalog:` (the catalog entry holds the URL). The dedup change stopped
  // normalizing protocol-pinned specs, leaving the raw URL as the direct dep and
  // breaking the migration-upgrade-pkg-pr-new-pnpm snapshot.
  it('normalizes a pre-existing range vite-plus to `catalog:`', () => {
    withForceOverride(() => {
      const pkg = { devDependencies: { 'vite-plus': '^0.1.20' } };
      rewritePackageJson(pkg, PackageManager.pnpm, true);
      expect(pkg.devDependencies['vite-plus']).toBe('catalog:');
    });
  });

  it('normalizes a vite-plus already pinned to the pkg.pr.new URL to `catalog:`', () => {
    withForceOverride(() => {
      const pkg = { devDependencies: { 'vite-plus': PKG_PR_NEW_URL } };
      rewritePackageJson(pkg, PackageManager.pnpm, true);
      expect(pkg.devDependencies['vite-plus']).toBe('catalog:');
    });
  });

  it('preserves a named catalog reference instead of collapsing onto the default catalog', () => {
    withForceOverride(() => {
      const pkg = { devDependencies: { 'vite-plus': 'catalog:tools' } };
      rewritePackageJson(pkg, PackageManager.pnpm, true);
      expect(pkg.devDependencies['vite-plus']).toBe('catalog:tools');
    });
  });
});
