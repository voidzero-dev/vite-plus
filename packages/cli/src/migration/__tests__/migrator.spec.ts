import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { parse as parseYaml } from 'yaml';

import { PackageManager } from '../../types/index.js';
import { VITE_PLUS_OVERRIDE_PACKAGES, VITEST_VERSION } from '../../utils/constants.js';
import { createMigrationReport } from '../report.js';

// Mock VITE_PLUS_VERSION to a stable value for snapshot tests.
// When tests run via `vp test`, the env var is injected with the actual version,
// which would cause snapshot mismatches.
vi.mock('../../utils/constants.js', async (importOriginal) => {
  const mod = await importOriginal<typeof import('../../utils/constants.js')>();
  return { ...mod, VITE_PLUS_VERSION: 'latest' };
});

const {
  rewritePackageJson,
  rewriteStandaloneProject,
  rewriteMonorepo,
  rewriteMonorepoProject,
  detectPendingCoreMigration,
  detectVitePlusBootstrapPending,
  ensureVitePlusBootstrap,
  finalizeCoreMigrationForExistingVitePlus,
  parseNvmrcVersion,
  detectNodeVersionManagerFile,
  migrateNodeVersionManagerFile,
  detectFramework,
  hasFrameworkShim,
  addFrameworkShim,
  injectCreateDefaultTemplate,
  injectFmtDefaults,
  injectLintTypeCheckDefaults,
  rewriteEslintPackageJson,
  detectIncompatibleEslintIntegration,
  preflightGitHooksSetup,
  detectLegacyGitHooksMigrationCandidate,
  setPackageManager,
} = await import('../migrator.js');

describe('rewritePackageJson', () => {
  it('should rewrite package.json scripts and extract staged config', async () => {
    const pkg = {
      scripts: {
        test: 'vitest',
        test_run: 'vitest run && vitest --ui',
        lint: 'oxlint',
        lint_config: 'oxlint --config .oxlint.json',
        lint_type_aware: 'oxlint --type-aware',
        fmt: 'oxfmt',
        fmt_config: 'oxfmt --config .oxfmt.json',
        pack: 'tsdown',
        pack_watch: 'tsdown --watch',
        preview: 'vite preview',
        optimize: 'vite optimize',
        build: 'pnpm install && vite build -r && vite run build --watch && tsdown && tsc || exit 1',
        dev: 'vite',
        dev_cjs: 'VITE_CJS_IGNORE_WARNING=true vite',
        dev_cjs_cross_env: 'cross-env VITE_CJS_IGNORE_WARNING=true vite',
        version: 'vite --version',
        version_short: 'vite -v',
        dev_help: 'vite --help && vite -h',
        dev_port: 'vite --port 3000',
        dev_host: 'vite --host 0.0.0.0',
        dev_open: 'vite --open',
        dev_verbose: 'vite --verbose',
        dev_debug: 'vite --debug',
        dev_trace: 'vite --trace',
        dev_profile: 'vite --profile',
        dev_stats: 'vite --stats',
        dev_analyze: 'vite --analyze',
        ready: 'oxlint --fix --type-aware && vitest run && tsdown && oxfmt --fix',
        ready_env:
          'NODE_ENV=test FOO=bar oxlint --fix --type-aware && NODE_ENV=test FOO=bar vitest run && NODE_ENV=test FOO=bar tsdown && NODE_ENV=test FOO=bar oxfmt --fix',
        ready_new:
          'vite install && vite fmt && vite lint --type-aware && vite test -r && vite build -r',
      },
      'lint-staged': {
        '*.js': ['oxlint --fix --type-aware', 'oxfmt --fix'],
        '*.ts': 'oxfmt --fix',
      },
    };
    const extractedStagedConfig = rewritePackageJson(pkg, PackageManager.npm);
    // lint-staged and vite-staged keys should be removed from pkg
    expect(pkg).toMatchSnapshot();
    // Extracted config should have rewritten commands
    expect(extractedStagedConfig).toMatchSnapshot();
  });

  it('should rewrite devDependencies and dependencies on standalone project', async () => {
    const pkg = {
      devDependencies: {
        oxlint: '1.0.0',
        oxfmt: '1.0.0',
      },
      dependencies: {
        foo: '1.0.0',
        tsdown: '1.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.pnpm);
    expect(pkg).toMatchSnapshot();
  });

  it('should rewrite devDependencies and dependencies on pnpm monorepo project', async () => {
    const pkg = {
      devDependencies: {
        oxlint: '1.0.0',
        oxfmt: '1.0.0',
      },
      dependencies: {
        foo: '1.0.0',
        tsdown: '1.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.pnpm, true);
    expect(pkg).toMatchSnapshot();
  });

  it('should rewrite devDependencies and dependencies on npm monorepo project', async () => {
    const pkg = {
      devDependencies: {
        oxlint: '1.0.0',
        oxfmt: '1.0.0',
      },
      dependencies: {
        foo: '1.0.0',
        tsdown: '1.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.npm, true);
    expect(pkg).toMatchSnapshot();
  });

  it('should rewrite devDependencies and dependencies on yarn monorepo project', async () => {
    const pkg = {
      devDependencies: {
        oxlint: '1.0.0',
        oxfmt: '1.0.0',
      },
      dependencies: {
        foo: '1.0.0',
        tsdown: '1.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.yarn, true);
    expect(pkg).toMatchSnapshot();
  });

  it('preserves named catalog dependency specs in monorepo projects', async () => {
    const pkg = {
      devDependencies: {
        vite: 'catalog:vite7',
        vitest: 'catalog:',
      },
      dependencies: {
        vitest: 'catalog:test',
      },
    };

    rewritePackageJson(pkg, PackageManager.pnpm, true);

    expect(pkg.devDependencies.vite).toBe('catalog:vite7');
    expect(pkg.devDependencies.vitest).toBe('catalog:');
    expect(pkg.dependencies.vitest).toBe('catalog:test');
    expect((pkg.devDependencies as Record<string, string>)['vite-plus']).toBe('catalog:');
  });

  it('normalizes a pre-existing pinned vite-plus to `catalog:` in catalog-supporting monorepos', async () => {
    const pkg = {
      devDependencies: {
        'vite-plus': '^0.1.20',
      },
    };

    rewritePackageJson(pkg, PackageManager.pnpm, true);

    expect(pkg.devDependencies['vite-plus']).toBe('catalog:');
  });

  it('leaves a pre-existing pinned vite-plus alone on npm monorepo projects', async () => {
    const pkg = {
      devDependencies: {
        'vite-plus': '^0.1.20',
      },
    };

    rewritePackageJson(pkg, PackageManager.npm, true);

    expect(pkg.devDependencies['vite-plus']).toBe('^0.1.20');
  });

  it('normalizes a pre-existing pinned vite-plus on yarn/bun monorepo projects', async () => {
    for (const pm of [PackageManager.yarn, PackageManager.bun]) {
      const pkg = { devDependencies: { 'vite-plus': '^0.1.20' } };
      rewritePackageJson(pkg, pm, true);
      expect(pkg.devDependencies['vite-plus']).toBe('catalog:');
    }
  });

  // Under pnpm, a package that depends on vite-plus needs a direct `vite` so
  // vitest's required `vite` peer binds to the override (@voidzero-dev/vite-plus-core);
  // otherwise pnpm's autoInstallPeers installs a second upstream vite and splits
  // vite-plus / vite / vitest into duplicate instances.
  describe('pnpm direct-vite dedupe (#1932)', () => {
    it('adds a direct `vite` devDep when a package depends on vite-plus under pnpm', () => {
      // monorepo sub-package -> catalog: (catalog.vite is written by rewriteCatalog)
      const sub: { devDependencies: Record<string, string> } = {
        devDependencies: { 'vite-plus': 'catalog:' },
      };
      rewritePackageJson(sub, PackageManager.pnpm, true);
      expect(sub.devDependencies.vite).toBe('catalog:');
      // inserted in sorted position (oxfmt sorts package.json), not appended
      expect(Object.keys(sub.devDependencies)).toEqual(['vite', 'vite-plus']);

      // standalone (no catalog) -> mirror the override target directly
      const standalone: { devDependencies: Record<string, string> } = {
        devDependencies: { 'vite-plus': 'latest' },
      };
      rewritePackageJson(standalone, PackageManager.pnpm);
      expect(standalone.devDependencies.vite).toBe(VITE_PLUS_OVERRIDE_PACKAGES.vite);
    });

    it('does not add a direct `vite` for npm/yarn/bun (they dedupe via overrides/resolutions)', () => {
      for (const pm of [PackageManager.npm, PackageManager.yarn, PackageManager.bun]) {
        const pkg: { devDependencies: Record<string, string> } = {
          devDependencies: { 'vite-plus': pm === PackageManager.npm ? '^0.1.20' : 'catalog:' },
        };
        rewritePackageJson(pkg, pm, true);
        expect(pkg.devDependencies.vite).toBeUndefined();
      }
    });

    it('does not add `vite` for a pnpm package that does not depend on vite-plus', () => {
      const pkg: { devDependencies: Record<string, string> } = {
        devDependencies: { typescript: '^5' },
      };
      rewritePackageJson(pkg, PackageManager.pnpm, true);
      expect(pkg.devDependencies.vite).toBeUndefined();
    });

    it('keeps an existing direct `vite` instead of overwriting it under pnpm', () => {
      const pkg: { devDependencies: Record<string, string> } = {
        devDependencies: { 'vite-plus': 'catalog:', vite: 'catalog:vite7' },
      };
      rewritePackageJson(pkg, PackageManager.pnpm, true);
      expect(pkg.devDependencies.vite).toBe('catalog:vite7');
    });

    it('does not inject a direct vite when vite is only a peerDependency under pnpm', () => {
      // A vite plugin that pins `vite` as a peer must keep its own contract;
      // injecting vite-plus-core as a concrete devDep would conflict with it.
      const pkg: {
        devDependencies: Record<string, string>;
        peerDependencies: Record<string, string>;
      } = {
        devDependencies: { 'vite-plus': 'catalog:' },
        peerDependencies: { vite: '^6.0.0' },
      };
      rewritePackageJson(pkg, PackageManager.pnpm, true);
      expect(pkg.devDependencies.vite).toBeUndefined();
      expect(pkg.peerDependencies.vite).toBe('^6.0.0');
    });

    it('does not add a second vite when an empty-string vite spec is already declared under pnpm', () => {
      const pkg: {
        dependencies: Record<string, string>;
        devDependencies: Record<string, string>;
      } = {
        dependencies: { vite: '' },
        devDependencies: { 'vite-plus': 'catalog:' },
      };
      rewritePackageJson(pkg, PackageManager.pnpm, true);
      expect(pkg.devDependencies.vite).toBeUndefined();
    });
  });

  it('preserves protocol-prefixed vite-plus specs (catalog:named, workspace:, link:, github:) in catalog-supporting monorepos', async () => {
    for (const existing of [
      'catalog:next',
      'workspace:*',
      'link:../vite-plus',
      'github:fork/vite-plus',
      'npm:@scope/vite-plus@^1.0.0',
    ]) {
      const pkg = { devDependencies: { 'vite-plus': existing } };
      rewritePackageJson(pkg, PackageManager.pnpm, true);
      expect(pkg.devDependencies['vite-plus']).toBe(existing);
    }
  });

  it('adds a direct vitest for a vitest-adjacent dep even when vite-plus is already present', async () => {
    // `vitest-browser-svelte` declares a NON-optional `vitest` peer. Even though
    // `vite-plus` is already here (bundling vitest transitively), a strict pnpm /
    // Yarn PnP layout won't expose that transitive vitest to the package root, so
    // the peer can't resolve. The migrator must pin a direct `vitest` regardless of
    // whether `vite-plus` is already present.
    const pkg = {
      devDependencies: {
        'vite-plus': '^0.1.20',
        'vitest-browser-svelte': '^1.0.0',
      },
    };

    rewritePackageJson(pkg, PackageManager.pnpm, true);

    expect(pkg.devDependencies['vite-plus']).toBe('catalog:');
    expect((pkg.devDependencies as Record<string, string>).vitest).toBe('catalog:');
  });

  it('does not auto-add vitest on a genuine normalize pass (no browser mode, no vitest-adjacent dep)', async () => {
    // vite-plus present, nothing vitest-adjacent, no browser mode -> nothing to
    // pin. needDirectVitest stays false and the package is left untouched beyond
    // the vite-plus spec normalization.
    const pkg = {
      devDependencies: {
        'vite-plus': '^0.1.20',
        '@scope/some-plugin': '^1.0.0',
      },
    };

    rewritePackageJson(pkg, PackageManager.pnpm, true);

    expect(pkg.devDependencies['vite-plus']).toBe('catalog:');
    expect((pkg.devDependencies as Record<string, string>).vitest).toBeUndefined();
  });

  it('uses default catalog specs for non-catalog dependency specs in monorepo projects', async () => {
    const pkg = {
      devDependencies: {
        vite: '^7.0.0',
      },
      dependencies: {
        vitest: '^4.0.0',
      },
    };

    rewritePackageJson(pkg, PackageManager.yarn, true);

    expect(pkg.devDependencies.vite).toBe('catalog:');
    // vitest is a managed override key — non-catalog specs are rewritten to
    // `catalog:` so the override is resolved through the catalog.
    expect(pkg.dependencies.vitest).toBe('catalog:');
    expect((pkg.devDependencies as Record<string, string>)['vite-plus']).toBe('catalog:');
  });

  it('uses override specs for yarn optional dependencies in monorepo projects', async () => {
    const pkg = {
      devDependencies: {
        vite: '^7.0.0',
      },
      optionalDependencies: {
        vite: '^7.0.0',
        vitest: 'catalog:test',
      },
    };

    rewritePackageJson(pkg, PackageManager.yarn, true);

    expect(pkg.devDependencies.vite).toBe('catalog:');
    expect(pkg.optionalDependencies.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    // vitest is now a managed override key — yarn optional deps receive the
    // literal override version so the resolution doesn't depend on catalog
    // lookup at the optionalDependency site.
    expect(pkg.optionalDependencies.vitest).toBe('4.1.9');
    expect((pkg.devDependencies as Record<string, string>)['vite-plus']).toBe('catalog:');
  });

  it('rewrites peer and optional dependency catalog specs in monorepo projects', async () => {
    const pkg = {
      peerDependencies: {
        vite: 'catalog:vite7',
        tsdown: 'catalog:build',
      },
      optionalDependencies: {
        vitest: 'catalog:test',
        oxlint: 'catalog:build',
      },
    };

    rewritePackageJson(pkg, PackageManager.pnpm, true);

    expect(pkg.peerDependencies.vite).toBe('*');
    expect(pkg.peerDependencies).not.toHaveProperty('tsdown');
    expect(pkg.optionalDependencies.vitest).toBe('catalog:test');
    expect(pkg.optionalDependencies).not.toHaveProperty('oxlint');
    expect(
      (pkg as { devDependencies?: Record<string, string> }).devDependencies?.['vite-plus'],
    ).toBe('catalog:');
  });

  it('preserves peer dependency ranges', async () => {
    const pkg = {
      peerDependencies: {
        vite: '^7.0.0',
        vitest: '^4.0.0',
      },
      optionalDependencies: {
        vite: '^7.0.0',
      },
    };

    rewritePackageJson(pkg, PackageManager.pnpm, true);

    expect(pkg.peerDependencies.vite).toBe('^7.0.0');
    expect(pkg.peerDependencies.vitest).toBe('^4.0.0');
    expect(pkg.optionalDependencies.vite).toBe('catalog:');
    expect(
      (pkg as { devDependencies?: Record<string, string> }).devDependencies?.['vite-plus'],
    ).toBe('catalog:');

    const npmPkg = {
      peerDependencies: {
        vite: '^7.0.0',
      },
      optionalDependencies: {
        vite: '^7.0.0',
      },
    };

    rewritePackageJson(npmPkg, PackageManager.npm);

    expect(npmPkg.peerDependencies.vite).toBe('^7.0.0');
    expect(npmPkg.optionalDependencies.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
  });

  it('adds local vitest when only a peer vitest exists for vitest-adjacent packages', async () => {
    const pkg = {
      dependencies: {
        'vitest-browser-svelte': '^1.0.0',
      },
      peerDependencies: {
        vitest: '^4.0.0',
      },
    };

    rewritePackageJson(pkg, PackageManager.pnpm, true);

    expect(pkg.peerDependencies.vitest).toBe('^4.0.0');
    expect((pkg as { devDependencies?: Record<string, string> }).devDependencies?.vitest).toBe(
      'catalog:',
    );
    expect(
      (pkg as { devDependencies?: Record<string, string> }).devDependencies?.['vite-plus'],
    ).toBe('catalog:');
  });

  it('keeps and normalizes @vitest/browser-playwright and ensures the playwright peer', async () => {
    // Playwright is opt-in: vite-plus no longer bundles the provider at runtime
    // (its `playwright` peer is non-optional), so the migration KEEPS the user's
    // declared `@vitest/browser-playwright` (version-normalized to the bundled
    // vitest version) and ensures its runtime framework peer `playwright`.
    // `@vitest/browser` stays in REMOVE_PACKAGES and is still stripped.
    const pkg = {
      devDependencies: {
        '@vitest/browser': '^4.0.0',
        '@vitest/browser-playwright': '^4.0.0',
        vitest: '^4.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.pnpm);
    // Standalone (supportCatalog=false) → concrete pinned spec.
    expect(pkg.devDependencies).toHaveProperty('@vitest/browser-playwright', VITEST_VERSION);
    expect(pkg.devDependencies).toHaveProperty('playwright', '*');
    expect(pkg.devDependencies).not.toHaveProperty('@vitest/browser');
  });

  it('injects a direct vite devDependency for an npm project that uses an opt-in browser provider', async () => {
    // npm's flat node_modules cannot dedupe the provider's own
    // `@vitest/browser → @vitest/mocker` subtree against the one vite-plus
    // bundles, leaving several nested `@vitest/mocker` copies. The `vite`
    // override only lands inside the `vitest` subtree, so the nested mockers
    // can't resolve their (optional) `vite` peer and `@vitest/mocker/dist/node.js`
    // throws `ERR_MODULE_NOT_FOUND: Cannot find package 'vite'` at config load.
    // A direct `vite` devDep (= the override target) forces npm to hoist a
    // single top-level `node_modules/vite` every nested mocker resolves.
    const pkg = {
      devDependencies: {
        '@vitest/browser-playwright': '^4.0.0',
        playwright: '^1.60.0',
        vitest: '^4.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.npm);
    expect(pkg.devDependencies).toHaveProperty('vite', VITE_PLUS_OVERRIDE_PACKAGES.vite);
  });

  it('does not inject a direct vite devDependency for npm projects without a browser provider', async () => {
    // Node-mode projects dedupe cleanly (a single hoisted `@vitest/mocker`
    // next to a top-level `vite`), so the migration must not add a direct
    // `vite` dep — leaving non-browser consumers untouched.
    const pkg = {
      devDependencies: {
        vitest: '^4.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.npm);
    expect(pkg.devDependencies).not.toHaveProperty('vite');
  });

  it('injects a direct vite devDependency for pnpm projects depending on vite-plus, but not yarn/bun', async () => {
    // pnpm needs a direct `vite` so vitest's `vite` peer binds to the override
    // instead of pnpm auto-installing a separate upstream vite. yarn/bun redirect
    // the transitive/peer vite via resolutions/overrides, so they do not get a
    // direct `vite` here (the bun workspace root is handled separately).
    for (const pm of [PackageManager.pnpm, PackageManager.yarn, PackageManager.bun]) {
      const pkg: { devDependencies: Record<string, string> } = {
        devDependencies: {
          '@vitest/browser-playwright': '^4.0.0',
          playwright: '^1.60.0',
          vitest: '^4.0.0',
        },
      };
      rewritePackageJson(pkg, pm);
      if (pm === PackageManager.pnpm) {
        expect(pkg.devDependencies).toHaveProperty('vite', VITE_PLUS_OVERRIDE_PACKAGES.vite);
      } else {
        expect(pkg.devDependencies).not.toHaveProperty('vite');
      }
    }
  });

  it('normalizes a pre-existing direct vite dep to the override target for an npm provider project', async () => {
    // A pre-existing direct `vite` is already normalized to the override target
    // by the `VITE_PLUS_OVERRIDE_PACKAGES` loop (vite-plus replaces `vite` with
    // its bundled core). The provider workaround must not duplicate or clobber
    // it — the single direct `vite` stays pointed at the override target.
    const pkg = {
      devDependencies: {
        '@vitest/browser-playwright': '^4.0.0',
        playwright: '^1.60.0',
        vite: '^7.0.0',
        vitest: '^4.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.npm);
    expect(pkg.devDependencies).toHaveProperty('vite', VITE_PLUS_OVERRIDE_PACKAGES.vite);
  });

  it('keeps and normalizes @vitest/browser-webdriverio and ensures the webdriverio peer', async () => {
    // Webdriverio is opt-in: vite-plus no longer bundles the provider, so the
    // migration KEEPS the user's declared `@vitest/browser-webdriverio`
    // (version-normalized to the bundled vitest version) and ensures its
    // runtime framework peer `webdriverio`. `@vitest/browser` stays in
    // REMOVE_PACKAGES and is still stripped.
    const pkg = {
      devDependencies: {
        '@vitest/browser': '^4.0.0',
        '@vitest/browser-webdriverio': '^4.0.0',
        vitest: '^4.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.pnpm);
    // Standalone (supportCatalog=false) → concrete pinned spec.
    expect(pkg.devDependencies).toHaveProperty('@vitest/browser-webdriverio', VITEST_VERSION);
    expect(pkg.devDependencies).toHaveProperty('webdriverio', '*');
    expect(pkg.devDependencies).not.toHaveProperty('@vitest/browser');
  });

  it('should not overwrite playwright if already in devDependencies', async () => {
    const pkg = {
      devDependencies: {
        '@vitest/browser-playwright': '^4.0.0',
        playwright: '^1.40.0',
        vitest: '^4.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.pnpm);
    expect(pkg.devDependencies).toHaveProperty('playwright', '^1.40.0');
  });

  it('should not add playwright if already in dependencies', async () => {
    const pkg = {
      dependencies: {
        playwright: '^1.40.0',
      },
      devDependencies: {
        '@vitest/browser-playwright': '^4.0.0',
        vitest: '^4.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.pnpm);
    expect(pkg.dependencies).toHaveProperty('playwright', '^1.40.0');
    expect(pkg.devDependencies).not.toHaveProperty('playwright');
  });

  it('adds a direct vitest devDependency when the package uses browser mode', async () => {
    // A package that drives vitest browser mode but has no direct vitest dep
    // (e.g. it only imports `vite-plus/test/browser-playwright`). `@vitest/browser`
    // needs `vitest` resolvable from the package root, so the migration must
    // pin it as a direct devDependency.
    const pkg = {
      devDependencies: {
        playwright: '^1.58.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.pnpm, true, undefined, undefined, true);
    expect(pkg.devDependencies).toHaveProperty('vitest', 'catalog:');
    expect(pkg.devDependencies).toHaveProperty('vite-plus', 'catalog:');
  });

  it('uses a concrete vitest version for browser mode in non-catalog package managers', async () => {
    const pkg = {
      devDependencies: {
        playwright: '^1.58.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.npm, false, undefined, undefined, true);
    expect((pkg as { devDependencies?: Record<string, string> }).devDependencies?.vitest).toBe(
      VITEST_VERSION,
    );
  });

  it('does not overwrite an existing direct vitest dep in browser mode', async () => {
    const pkg = {
      devDependencies: {
        vitest: '^4.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.pnpm, true, undefined, undefined, true);
    // existing direct dep is normalized through the override path, not replaced
    expect(pkg.devDependencies.vitest).toBe('catalog:');
  });

  it('does not add vitest when browser mode is not detected', async () => {
    const pkg = {
      devDependencies: {
        vite: '^7.0.0',
      },
    };
    rewritePackageJson(pkg, PackageManager.pnpm, true, undefined, undefined, false);
    expect(pkg.devDependencies).not.toHaveProperty('vitest');
  });

  it('adds a direct vitest dep when a browser provider is declared but not source-imported', async () => {
    // Config-only browser mode: vitest is enabled via `vite.config.ts`
    // (e.g. `test.browser.provider: 'playwright'`) and the provider package is
    // declared in devDependencies, but no source file `import`s it. The
    // source-scan signal (`usesVitestBrowserMode`) is therefore false; the
    // dep declaration in the original package.json must still drive the
    // direct-`vitest` injection so the browser optimizer can resolve `vitest`
    // from the package root under pnpm strict / Yarn PnP.
    const pkg = {
      devDependencies: {
        '@vitest/browser': '^4.1.7',
        '@vitest/browser-playwright': '^4.1.7',
      },
    };
    rewritePackageJson(pkg, PackageManager.pnpm, true, undefined, undefined, false);
    expect(pkg.devDependencies).toHaveProperty('vitest', 'catalog:');
    expect(pkg.devDependencies).toHaveProperty('vite-plus', 'catalog:');
    // The base `@vitest/browser` is still stripped (bundled by vite-plus).
    expect(pkg.devDependencies).not.toHaveProperty('@vitest/browser');
    // Playwright is opt-in: vite-plus keeps it in the user's deps, normalized to
    // the bundled vitest version, so the rewritten import resolves.
    expect(pkg.devDependencies).toHaveProperty('@vitest/browser-playwright', VITEST_VERSION);
    // The provider's runtime peer dep is preserved.
    expect(pkg.devDependencies).toHaveProperty('playwright', '*');
  });
});

describe('rewriteEslintPackageJson', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-eslint-cleanup-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  function writePkg(pkg: object): string {
    const pkgPath = path.join(tmpDir, 'package.json');
    fs.writeFileSync(pkgPath, JSON.stringify(pkg));
    return pkgPath;
  }

  it('removes eslint, eslint-plugin-*, eslint-config-*, typescript-eslint, @typescript-eslint/*', () => {
    const pkgPath = writePkg({
      devDependencies: {
        eslint: '^9.0.0',
        'eslint-plugin-vue': '^10.0.0',
        'eslint-plugin-react': '^7.0.0',
        'eslint-config-airbnb': '^19.0.0',
        'typescript-eslint': '^8.0.0',
        '@typescript-eslint/parser': '^8.0.0',
        '@typescript-eslint/eslint-plugin': '^8.0.0',
        vite: '^7.0.0',
      },
      dependencies: {
        'eslint-plugin-import': '^2.0.0',
        vue: '^3.5.0',
      },
    });
    rewriteEslintPackageJson(pkgPath);
    const pkg = readJson(pkgPath);
    expect(pkg.devDependencies).toEqual({ vite: '^7.0.0' });
    expect(pkg.dependencies).toEqual({ vue: '^3.5.0' });
  });

  it('removes scoped ESLint plugin/config packages (e.g. @vue/eslint-config-typescript)', () => {
    const pkgPath = writePkg({
      devDependencies: {
        '@vue/eslint-config-typescript': '^13.0.0',
        '@nuxt/eslint-config': '^0.5.0',
        '@stylistic/eslint-plugin': '^2.0.0',
        '@stylistic/eslint-plugin-ts': '^2.0.0',
        '@vitest/eslint-plugin': '^1.0.0',
        keepme: '^1.0.0',
      },
    });
    rewriteEslintPackageJson(pkgPath);
    const pkg = readJson(pkgPath);
    expect(pkg.devDependencies).toEqual({ keepme: '^1.0.0' });
  });

  it('removes @eslint/*, @eslint-community/*, and @angular-eslint/* scope packages', () => {
    const pkgPath = writePkg({
      devDependencies: {
        eslint: '^9.0.0',
        '@eslint/js': '^9.0.0',
        '@eslint/eslintrc': '^3.0.0',
        '@eslint/compat': '^1.0.0',
        '@eslint-community/eslint-utils': '^4.0.0',
        '@eslint-community/regexpp': '^4.0.0',
        '@angular-eslint/template-parser': '^18.0.0',
        '@angular-eslint/builder': '^18.0.0',
        keepme: '^1.0.0',
      },
    });
    rewriteEslintPackageJson(pkgPath);
    const pkg = readJson(pkgPath);
    expect(pkg.devDependencies).toEqual({ keepme: '^1.0.0' });
  });

  it('removes ESLint formatter and helper packages', () => {
    const pkgPath = writePkg({
      devDependencies: {
        eslint: '^9.0.0',
        'eslint-formatter-pretty': '^6.0.0',
        'eslint-formatter-gitlab': '^5.0.0',
        eslintrc: '^2.0.0',
        'eslint-utils': '^3.0.0',
        'eslint-visitor-keys': '^4.0.0',
        'eslint-scope': '^8.0.0',
        'eslint-define-config': '^2.0.0',
        'eslint-doc-generator': '^2.0.0',
        keepme: '^1.0.0',
      },
    });
    rewriteEslintPackageJson(pkgPath);
    const pkg = readJson(pkgPath);
    expect(pkg.devDependencies).toEqual({ keepme: '^1.0.0' });
  });

  it('does NOT remove framework-ESLint integrations (e.g. @nuxt/eslint) — those short-circuit migration upstream', () => {
    // The skip path in `bin.ts` prevents `rewriteEslintPackageJson` from
    // being called when `@nuxt/eslint` is present, so this function
    // doesn't need to (and shouldn't) know about it.
    const pkgPath = writePkg({
      devDependencies: {
        eslint: '^9.0.0',
        '@nuxt/eslint': '^1.0.0',
      },
    });
    rewriteEslintPackageJson(pkgPath);
    const pkg = readJson(pkgPath);
    expect(pkg.devDependencies).toEqual({ '@nuxt/eslint': '^1.0.0' });
  });

  it('preserves reusable @typescript-eslint/* AST libraries (utils, typescript-estree, etc.)', () => {
    const pkgPath = writePkg({
      devDependencies: {
        eslint: '^9.0.0',
        '@typescript-eslint/parser': '^8.0.0',
        '@typescript-eslint/eslint-plugin': '^8.0.0',
        '@typescript-eslint/rule-tester': '^8.0.0',
        '@typescript-eslint/utils': '^8.0.0',
        '@typescript-eslint/typescript-estree': '^8.0.0',
        '@typescript-eslint/scope-manager': '^8.0.0',
        '@typescript-eslint/types': '^8.0.0',
        vite: '^7.0.0',
      },
    });
    rewriteEslintPackageJson(pkgPath);
    const pkg = readJson(pkgPath);
    expect(pkg.devDependencies).toEqual({
      '@typescript-eslint/utils': '^8.0.0',
      '@typescript-eslint/typescript-estree': '^8.0.0',
      '@typescript-eslint/scope-manager': '^8.0.0',
      '@typescript-eslint/types': '^8.0.0',
      vite: '^7.0.0',
    });
  });

  it('removes @types/<X> packages symmetrically with their runtime counterparts', () => {
    const pkgPath = writePkg({
      devDependencies: {
        eslint: '^9.0.0',
        '@types/eslint': '^9.0.0',
        '@types/eslint-plugin-foo': '^1.0.0',
        '@types/eslint-config-bar': '^1.0.0',
        // Type-only counterpart of an ESLint plugin should also go.
        '@types/eslint-scope': '^3.0.0',
        // Unrelated @types should stay.
        '@types/node': '^22.0.0',
      },
    });
    rewriteEslintPackageJson(pkgPath);
    const pkg = readJson(pkgPath);
    expect(pkg.devDependencies).toEqual({ '@types/node': '^22.0.0' });
  });

  it('scrubs peerDependencies and optionalDependencies', () => {
    const pkgPath = writePkg({
      peerDependencies: {
        eslint: '>=9',
        'eslint-plugin-vue': '^10.0.0',
      },
      optionalDependencies: {
        '@typescript-eslint/parser': '^8.0.0',
      },
      devDependencies: { vite: '^7.0.0' },
    });
    rewriteEslintPackageJson(pkgPath);
    const pkg = readJson(pkgPath);
    expect(pkg.peerDependencies).toBeUndefined();
    expect(pkg.optionalDependencies).toBeUndefined();
    expect(pkg.devDependencies).toEqual({ vite: '^7.0.0' });
  });

  it('deletes the dependency field entirely when our cleanup emptied it', () => {
    const pkgPath = writePkg({
      devDependencies: {
        eslint: '^9.0.0',
        'eslint-plugin-import': '^2.0.0',
      },
      dependencies: { 'eslint-config-airbnb': '^19.0.0' },
    });
    rewriteEslintPackageJson(pkgPath);
    const pkg = readJson(pkgPath);
    expect(pkg.devDependencies).toBeUndefined();
    expect(pkg.dependencies).toBeUndefined();
  });

  it('preserves unrelated dependencies (e.g. @vitejs/plugin-vue, vue, vite, @nuxt/kit)', () => {
    const pkgPath = writePkg({
      devDependencies: {
        eslint: '^9.0.0',
        '@vitejs/plugin-vue': '^6.0.0',
        '@vue/runtime-core': '^3.5.0',
        '@nuxt/kit': '^3.13.0',
        vite: '^7.0.0',
      },
    });
    rewriteEslintPackageJson(pkgPath);
    const pkg = readJson(pkgPath);
    expect(pkg.devDependencies).toEqual({
      '@vitejs/plugin-vue': '^6.0.0',
      '@vue/runtime-core': '^3.5.0',
      '@nuxt/kit': '^3.13.0',
      vite: '^7.0.0',
    });
  });

  it('no-ops when package.json has no eslint-ecosystem deps', () => {
    const pkgPath = writePkg({
      devDependencies: { vite: '^7.0.0' },
    });
    const before = fs.readFileSync(pkgPath, 'utf8');
    rewriteEslintPackageJson(pkgPath);
    const after = fs.readFileSync(pkgPath, 'utf8');
    expect(after).toBe(before);
  });

  it('preserves packages referenced in lint.jsPlugins (so the generated config still loads)', () => {
    // When @oxlint/migrate translates a real ESLint plugin into a
    // lint.jsPlugins reference, Oxlint will `import()` the package at
    // lint time. If we strip it from package.json the lint config we
    // just generated is invalidated. The preserveJsPlugins set guards
    // against that.
    const pkgPath = writePkg({
      devDependencies: {
        eslint: '^9.0.0',
        'eslint-plugin-vue': '^10.0.0',
        'eslint-plugin-import-x': '^4.0.0',
        'eslint-plugin-react': '^7.37.0',
        '@stylistic/eslint-plugin': '^2.0.0',
        '@typescript-eslint/parser': '^8.0.0',
        vite: '^7.0.0',
      },
    });
    rewriteEslintPackageJson(
      pkgPath,
      new Set(['eslint-plugin-vue', 'eslint-plugin-import-x', '@stylistic/eslint-plugin']),
    );
    const pkg = readJson(pkgPath);
    expect(pkg.devDependencies).toEqual({
      // Preserved (in jsPlugins set, so Oxlint will load them):
      'eslint-plugin-vue': '^10.0.0',
      'eslint-plugin-import-x': '^4.0.0',
      '@stylistic/eslint-plugin': '^2.0.0',
      // Removed (no jsPlugins reference, normal cleanup):
      // 'eslint': stripped
      // 'eslint-plugin-react': stripped
      // '@typescript-eslint/parser': stripped
      vite: '^7.0.0',
    });
  });

  it('preserveJsPlugins overrides every cleanup pattern (named, prefix, scope, regex)', () => {
    // Stress-test each branch of isEslintEcosystemDep against the
    // preserve set so a future contributor adding a new cleanup branch
    // can't accidentally bypass the carve-out.
    const pkgPath = writePkg({
      devDependencies: {
        eslint: '^9.0.0', // named match in ESLINT_ECOSYSTEM_NAMES
        'eslint-plugin-foo': '^1.0.0', // prefix match
        '@eslint/js': '^9.0.0', // scope match
        '@scope/eslint-plugin-bar': '^1.0.0', // scoped regex match
        keepme: '^1.0.0',
      },
    });
    rewriteEslintPackageJson(
      pkgPath,
      new Set(['eslint', 'eslint-plugin-foo', '@eslint/js', '@scope/eslint-plugin-bar']),
    );
    const pkg = readJson(pkgPath);
    expect(pkg.devDependencies).toEqual({
      eslint: '^9.0.0',
      'eslint-plugin-foo': '^1.0.0',
      '@eslint/js': '^9.0.0',
      '@scope/eslint-plugin-bar': '^1.0.0',
      keepme: '^1.0.0',
    });
  });

  it('does not invent preserveJsPlugins entries — only what the caller asked for', () => {
    // Sanity: an empty preserve set behaves identically to the default
    // (no carve-out), so the new parameter can't accidentally weaken
    // the cleanup for existing callers.
    const pkgPath = writePkg({
      devDependencies: {
        eslint: '^9.0.0',
        'eslint-plugin-foo': '^1.0.0',
        vite: '^7.0.0',
      },
    });
    rewriteEslintPackageJson(pkgPath, new Set());
    const pkg = readJson(pkgPath);
    expect(pkg.devDependencies).toEqual({ vite: '^7.0.0' });
  });
});

function writePkgAt(dir: string, pkg: object): void {
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, 'package.json'), JSON.stringify(pkg));
}

describe('detectIncompatibleEslintIntegration', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-incompat-eslint-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('returns "@nuxt/eslint" when listed in devDependencies', () => {
    writePkgAt(tmpDir, { devDependencies: { '@nuxt/eslint': '^1.0.0' } });
    expect(detectIncompatibleEslintIntegration(tmpDir)).toBe('@nuxt/eslint');
  });

  it('returns "@nuxt/eslint" when listed in dependencies', () => {
    writePkgAt(tmpDir, { dependencies: { '@nuxt/eslint': '^1.0.0' } });
    expect(detectIncompatibleEslintIntegration(tmpDir)).toBe('@nuxt/eslint');
  });

  it('detects when @nuxt/eslint lives in a workspace package, not the root', () => {
    writePkgAt(tmpDir, { name: 'root' });
    writePkgAt(path.join(tmpDir, 'packages/app'), {
      name: 'app',
      devDependencies: { '@nuxt/eslint': '^1.0.0' },
    });
    expect(
      detectIncompatibleEslintIntegration(tmpDir, [{ name: 'app', path: 'packages/app' }]),
    ).toBe('@nuxt/eslint');
  });

  it('returns undefined when @nuxt/eslint is absent', () => {
    writePkgAt(tmpDir, {
      devDependencies: { eslint: '^9.0.0', '@nuxt/kit': '^3.0.0' },
    });
    expect(detectIncompatibleEslintIntegration(tmpDir)).toBeUndefined();
  });

  it('returns undefined when package.json is missing', () => {
    expect(detectIncompatibleEslintIntegration(tmpDir)).toBeUndefined();
  });
});

describe('parseNvmrcVersion', () => {
  it('strips v prefix', () => {
    expect(parseNvmrcVersion('v20.5.0')).toBe('20.5.0');
  });

  it('passes through version without prefix', () => {
    expect(parseNvmrcVersion('20.5.0')).toBe('20.5.0');
    expect(parseNvmrcVersion('20')).toBe('20');
  });

  it('passes through lts aliases', () => {
    expect(parseNvmrcVersion('lts/*')).toBe('lts/*');
    expect(parseNvmrcVersion('lts/iron')).toBe('lts/iron');
    expect(parseNvmrcVersion('lts/-1')).toBe('lts/-1');
  });

  it('converts node/stable aliases to lts/*', () => {
    expect(parseNvmrcVersion('node')).toBe('lts/*');
    expect(parseNvmrcVersion('stable')).toBe('lts/*');
  });

  it('returns null for untranslatable aliases', () => {
    expect(parseNvmrcVersion('iojs')).toBeNull();
    expect(parseNvmrcVersion('system')).toBeNull();
    expect(parseNvmrcVersion('default')).toBeNull();
    expect(parseNvmrcVersion('')).toBeNull();
  });

  it('returns null for invalid version strings', () => {
    expect(parseNvmrcVersion('v')).toBeNull();
    expect(parseNvmrcVersion('laetst')).toBeNull();
    expect(parseNvmrcVersion('20.5.0.1')).toBeNull();
  });
});

describe('setPackageManager', () => {
  let tmpDir: string;

  const downloadResult = {
    name: 'pnpm',
    installDir: '/tmp/install',
    binPrefix: '/tmp/install/bin',
    packageName: 'pnpm',
    version: '11.5.1',
  };

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  const readPkg = () =>
    JSON.parse(fs.readFileSync(path.join(tmpDir, 'package.json'), 'utf8')) as Record<
      string,
      unknown
    >;

  it('writes devEngines.packageManager when neither field exists', () => {
    fs.writeFileSync(path.join(tmpDir, 'package.json'), JSON.stringify({ name: 'x' }, null, 2));
    setPackageManager(tmpDir, downloadResult);
    expect(readPkg().devEngines).toEqual({
      packageManager: { name: 'pnpm', version: '11.5.1', onFail: 'download' },
    });
  });

  it('keeps an existing packageManager field untouched', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'x', packageManager: 'npm@10.5.0' }, null, 2),
    );
    setPackageManager(tmpDir, downloadResult);
    const pkg = readPkg();
    expect(pkg.packageManager).toBe('npm@10.5.0');
    expect(pkg.devEngines).toBeUndefined();
  });

  it('preserves an existing devEngines.runtime entry', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify(
        { name: 'x', devEngines: { runtime: { name: 'node', version: '^24.0.0' } } },
        null,
        2,
      ),
    );
    setPackageManager(tmpDir, downloadResult);
    expect(readPkg().devEngines).toEqual({
      runtime: { name: 'node', version: '^24.0.0' },
      packageManager: { name: 'pnpm', version: '11.5.1', onFail: 'download' },
    });
  });

  it('replaces a malformed devEngines value instead of spreading it', () => {
    // spreading a string would corrupt the field with numeric index keys
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'x', devEngines: 'oops' }, null, 2),
    );
    setPackageManager(tmpDir, downloadResult);
    expect(readPkg().devEngines).toEqual({
      packageManager: { name: 'pnpm', version: '11.5.1', onFail: 'download' },
    });
  });
});

describe('detectNodeVersionManagerFile', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('returns undefined when no version files found', () => {
    expect(detectNodeVersionManagerFile(tmpDir)).toBeUndefined();
  });

  it('returns undefined when .node-version already exists', () => {
    fs.writeFileSync(path.join(tmpDir, '.node-version'), '20.5.0\n');
    fs.writeFileSync(path.join(tmpDir, '.nvmrc'), 'v20.5.0\n');
    expect(detectNodeVersionManagerFile(tmpDir)).toBeUndefined();
  });

  it('detects .nvmrc', () => {
    fs.writeFileSync(path.join(tmpDir, '.nvmrc'), 'v20.5.0\n');
    expect(detectNodeVersionManagerFile(tmpDir)).toEqual({ file: '.nvmrc' });
  });

  it('detects volta node in package.json', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ volta: { node: '20.5.0' } }),
    );
    expect(detectNodeVersionManagerFile(tmpDir)).toEqual({
      file: 'package.json',
      voltaNodeVersion: '20.5.0',
    });
  });

  it('prefers .nvmrc over volta when both are present and sets voltaPresent', () => {
    fs.writeFileSync(path.join(tmpDir, '.nvmrc'), 'v20.5.0\n');
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ volta: { node: '18.0.0' } }),
    );
    expect(detectNodeVersionManagerFile(tmpDir)).toEqual({ file: '.nvmrc', voltaPresent: true });
  });

  it('returns undefined when .node-version already exists even with volta', () => {
    fs.writeFileSync(path.join(tmpDir, '.node-version'), '20.5.0\n');
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ volta: { node: '20.5.0' } }),
    );
    expect(detectNodeVersionManagerFile(tmpDir)).toBeUndefined();
  });
});

describe('migrateNodeVersionManagerFile', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('adds volta manual step when voltaPresent is set', () => {
    fs.writeFileSync(path.join(tmpDir, '.nvmrc'), 'v20.5.0\n');
    const report = createMigrationReport();
    migrateNodeVersionManagerFile(tmpDir, { file: '.nvmrc', voltaPresent: true }, report);
    expect(report.manualSteps).toContain('Remove the "volta" field from package.json');
  });

  it('migrates .nvmrc to .node-version and removes .nvmrc', () => {
    fs.writeFileSync(path.join(tmpDir, '.nvmrc'), 'v20.5.0\n');
    const ok = migrateNodeVersionManagerFile(tmpDir, { file: '.nvmrc' });
    expect(ok).toBe(true);
    expect(fs.readFileSync(path.join(tmpDir, '.node-version'), 'utf8')).toBe('20.5.0\n');
    expect(fs.existsSync(path.join(tmpDir, '.nvmrc'))).toBe(false);
  });

  it('returns false and warns for unsupported alias', () => {
    fs.writeFileSync(path.join(tmpDir, '.nvmrc'), 'system\n');
    const report = createMigrationReport();
    const ok = migrateNodeVersionManagerFile(tmpDir, { file: '.nvmrc' }, report);
    expect(ok).toBe(false);
    expect(report.warnings.length).toBe(1);
    expect(fs.existsSync(path.join(tmpDir, '.node-version'))).toBe(false);
  });

  it('migrates volta node version to .node-version', () => {
    const ok = migrateNodeVersionManagerFile(tmpDir, {
      file: 'package.json',
      voltaNodeVersion: '20.5.0',
    });
    expect(ok).toBe(true);
    expect(fs.readFileSync(path.join(tmpDir, '.node-version'), 'utf8')).toBe('20.5.0\n');
  });

  it('sets nodeVersionFileMigrated and manualSteps in report for volta migration', () => {
    const report = createMigrationReport();
    migrateNodeVersionManagerFile(
      tmpDir,
      { file: 'package.json', voltaNodeVersion: '20.5.0' },
      report,
    );
    expect(report.nodeVersionFileMigrated).toBe(true);
    expect(report.manualSteps).toContain('Remove the "volta" field from package.json');
  });

  it('normalizes volta.node "lts" to "lts/*"', () => {
    const ok = migrateNodeVersionManagerFile(tmpDir, {
      file: 'package.json',
      voltaNodeVersion: 'lts',
    });
    expect(ok).toBe(true);
    expect(fs.readFileSync(path.join(tmpDir, '.node-version'), 'utf8')).toBe('lts/*\n');
  });

  it('returns false and warns when volta.node is a partial version', () => {
    const report = createMigrationReport();
    const ok = migrateNodeVersionManagerFile(
      tmpDir,
      { file: 'package.json', voltaNodeVersion: '20' },
      report,
    );
    expect(ok).toBe(false);
    expect(report.warnings.length).toBe(1);
    expect(fs.existsSync(path.join(tmpDir, '.node-version'))).toBe(false);
  });
});

function makeWorkspaceInfo(
  rootDir: string,
  packageManager: PackageManager,
): import('../../types/index.js').WorkspaceInfo {
  return {
    rootDir,
    isMonorepo: false,
    monorepoScope: '',
    workspacePatterns: [],
    parentDirs: [],
    packageManager,
    packageManagerVersion: '10.33.0',
    downloadPackageManager: {
      name: packageManager,
      installDir: '/tmp',
      binPrefix: '/tmp/bin',
      packageName: packageManager,
      version: '10.33.0',
    },
    packages: [],
  };
}

function readJson(filePath: string): Record<string, unknown> {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function readYaml(filePath: string): string {
  return fs.readFileSync(filePath, 'utf8');
}

function readYamlObject(filePath: string): Record<string, unknown> {
  return parseYaml(readYaml(filePath)) as Record<string, unknown>;
}

describe('ensureVitePlusBootstrap', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-bootstrap-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('adds missing npm overrides and package manager pin for existing Vite+ projects', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'test', devDependencies: { 'vite-plus': 'latest' } }),
    );

    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.npm)).toBe(true);

    const report = createMigrationReport();
    const result = ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, PackageManager.npm), report);

    expect(result.changed).toBe(true);
    expect(report.packageManagerBootstrapConfigured).toBe(true);
    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.npm)).toBe(false);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      overrides: Record<string, string>;
      devEngines: { packageManager: { name: string } };
    };
    expect(pkg.overrides.vite).toContain('@voidzero-dev/vite-plus-core');
    // Common case (no @vitest/* dep, no vitest source): `vitest` is NOT managed —
    // it arrives transitively through vite-plus, so no override is written.
    expect(pkg.overrides.vitest).toBeUndefined();
    expect(pkg.devEngines.packageManager.name).toBe(PackageManager.npm);
  });

  it('removes the stale vitest wrapper override for a non-vitest npm project', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { 'vite-plus': 'latest' },
        overrides: {
          vite: 'npm:@voidzero-dev/vite-plus-core@0.1.0',
          vitest: 'npm:@voidzero-dev/vite-plus-test@0.1.0',
        },
        devEngines: {
          packageManager: { name: 'npm', version: '10.33.0', onFail: 'download' },
        },
      }),
    );
    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.npm)).toBe(true);
    const result = ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, PackageManager.npm));

    // The `vite` alias still points at the live `@voidzero-dev/vite-plus-core`
    // package, so it satisfies the migration and is left untouched. The project
    // does NOT use vitest directly (no @vitest/* dep, no vitest source), so the
    // stale `vitest` wrapper override (the DELETED `@voidzero-dev/vite-plus-test`)
    // is REMOVED entirely — vitest arrives transitively through vite-plus.
    expect(result.changed).toBe(true);
    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      overrides: Record<string, string>;
    };
    expect(pkg.overrides.vite).toBe('npm:@voidzero-dev/vite-plus-core@0.1.0');
    expect(pkg.overrides.vitest).toBeUndefined();
    expect(pkg.overrides['@vitest/expect']).toBeUndefined();
    expect(pkg.overrides['@vitest/coverage-v8']).toBeUndefined();
    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.npm)).toBe(false);
  });

  it('rewrites direct npm Vite dependencies before adding overrides', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { 'vite-plus': 'latest', vite: '^7.0.0' },
        dependencies: { vitest: '^3.0.0' },
        overrides: {
          vite: 'npm:@voidzero-dev/vite-plus-core@latest',
          vitest: 'npm:@voidzero-dev/vite-plus-test@latest',
        },
        devEngines: {
          packageManager: { name: 'npm', version: '10.33.0', onFail: 'download' },
        },
      }),
    );

    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.npm)).toBe(true);
    const result = ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, PackageManager.npm));

    expect(result.changed).toBe(true);
    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.npm)).toBe(false);
    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      devDependencies: Record<string, string>;
      dependencies: Record<string, string>;
    };
    expect(pkg.devDependencies.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    // Common case (no @vitest/* dep, no vitest source): the direct `vitest` dep
    // is removed — it arrives transitively through vite-plus.
    expect(pkg.dependencies.vitest).toBeUndefined();
  });

  it('normalizes catalog vite-plus pins for npm projects', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { 'vite-plus': 'catalog:' },
        overrides: {
          vite: 'npm:@voidzero-dev/vite-plus-core@latest',
          vitest: 'npm:@voidzero-dev/vite-plus-test@latest',
        },
        devEngines: {
          packageManager: { name: 'npm', version: '10.33.0', onFail: 'download' },
        },
      }),
    );

    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.npm)).toBe(true);
    const result = ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, PackageManager.npm));

    expect(result.changed).toBe(true);
    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.npm)).toBe(false);
    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      devDependencies: Record<string, string>;
    };
    expect(pkg.devDependencies['vite-plus']).toBe('latest');
  });

  it('adds missing pnpm workspace overrides without writing optional setup files', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'test', devDependencies: { 'vite-plus': 'catalog:' } }),
    );

    const result = ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, PackageManager.pnpm));

    expect(result.changed).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, 'pnpm-workspace.yaml'))).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, 'AGENTS.md'))).toBe(false);
    expect(fs.existsSync(path.join(tmpDir, '.vite-hooks'))).toBe(false);
    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm)).toBe(false);
  });

  it('detects missing pnpm workspace catalog entry for vite-plus', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { 'vite-plus': 'catalog:' },
        devEngines: {
          packageManager: { name: 'pnpm', version: '10.33.0', onFail: 'download' },
        },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      [
        'catalog:',
        '  vite: npm:@voidzero-dev/vite-plus-core@latest',
        '  vitest: npm:@voidzero-dev/vite-plus-test@latest',
        'overrides:',
        "  vite: 'catalog:'",
        "  vitest: 'catalog:'",
        'peerDependencyRules:',
        '  allowAny:',
        '    - vite',
        '    - vitest',
        '  allowedVersions:',
        "    vite: '*'",
        "    vitest: '*'",
      ].join('\n'),
    );

    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm)).toBe(true);
    const result = ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, PackageManager.pnpm));

    expect(result.changed).toBe(true);
    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm)).toBe(false);
    const workspace = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      catalog: Record<string, string>;
    };
    expect(workspace.catalog['vite-plus']).toBe('latest');
  });

  it('reconciles stale pnpm-workspace.yaml overrides when package.json has an empty pnpm field (urllib shape)', () => {
    // urllib 0.1.x shape: an empty `pnpm: {}` in package.json AND a committed
    // pnpm-workspace.yaml whose overrides pin vite/vitest to the deleted
    // @voidzero-dev/vite-plus-test wrapper. The empty `pnpm: {}` is truthy, so the
    // bootstrap used to take the package.json path and IGNORE the workspace.yaml,
    // leaving the dead wrapper override in place (and a second, conflicting
    // override source in package.json). Because a pnpm-workspace.yaml exists, the
    // workspace.yaml is the real config location and must be reconciled.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'urllib',
        devDependencies: {
          '@vitest/coverage-v8': '^4.1.8',
          vite: 'npm:@voidzero-dev/vite-plus-core@^0.1.24',
          'vite-plus': '^0.1.24',
          vitest: 'npm:@voidzero-dev/vite-plus-test@^0.1.24',
        },
        pnpm: {},
        devEngines: {
          packageManager: { name: 'pnpm', version: '11.7.0', onFail: 'download' },
        },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      [
        'overrides:',
        "  vite: 'npm:@voidzero-dev/vite-plus-core@^0.1.24'",
        "  vitest: 'npm:@voidzero-dev/vite-plus-test@^0.1.24'",
        'peerDependencyRules:',
        '  allowAny:',
        '    - vite',
        '    - vitest',
      ].join('\n'),
    );

    ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, PackageManager.pnpm));

    // The deleted wrapper alias must no longer survive in the workspace.yaml.
    const workspaceRaw = fs.readFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), 'utf-8');
    expect(workspaceRaw).not.toContain('@voidzero-dev/vite-plus-test');
    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      devDependencies: Record<string, string>;
    };
    expect(pkg.devDependencies.vitest).toBe('catalog:');
    expect(JSON.stringify(pkg)).not.toContain('@voidzero-dev/vite-plus-test');

    // And the project must not be left pending (no stale wrapper override anywhere).
    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm)).toBe(false);
  });

  it('aligns coverage providers to the bundled vitest version (urllib coverage-v8 symptom)', () => {
    // A coverage provider is a project-installed peer that Vitest pins to an
    // exact runner version; a skewed copy makes Vitest run mixed versions. The
    // upgrade must bump it to the bundled vitest version, not leave it behind.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: {
          'vite-plus': 'latest',
          '@vitest/coverage-v8': '^4.1.8',
        },
        overrides: {
          vite: 'npm:@voidzero-dev/vite-plus-core@latest',
          vitest: 'npm:@voidzero-dev/vite-plus-test@latest',
        },
        devEngines: {
          packageManager: { name: 'npm', version: '10.33.0', onFail: 'download' },
        },
      }),
    );

    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.npm)).toBe(true);
    ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, PackageManager.npm));

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      devDependencies: Record<string, string>;
    };
    expect(pkg.devDependencies['@vitest/coverage-v8']).toBe(VITEST_VERSION);
    expect(pkg.devDependencies.vitest).toBe(VITEST_VERSION);
    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.npm)).toBe(false);
  });

  it('aligns the full @vitest/* ecosystem (ui, web-worker) but leaves @vitest/eslint-plugin alone', () => {
    // Every official @vitest/* package carries an exact `vitest` peer, so each
    // must match the bundled vitest. @vitest/eslint-plugin versions on its own
    // line (`vitest: *` peer) and must NOT be pinned to the vitest version.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: {
          'vite-plus': 'latest',
          '@vitest/ui': '^4.1.0',
          '@vitest/web-worker': '^4.1.0',
          '@vitest/eslint-plugin': '^1.0.0',
        },
        overrides: {
          vite: 'npm:@voidzero-dev/vite-plus-core@latest',
        },
        devEngines: {
          packageManager: { name: 'npm', version: '10.33.0', onFail: 'download' },
        },
      }),
    );

    ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, PackageManager.npm));

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      devDependencies: Record<string, string>;
    };
    expect(pkg.devDependencies['@vitest/ui']).toBe(VITEST_VERSION);
    expect(pkg.devDependencies['@vitest/web-worker']).toBe(VITEST_VERSION);
    expect(pkg.devDependencies['@vitest/eslint-plugin']).toBe('^1.0.0');
    expect(pkg.devDependencies.vitest).toBe(VITEST_VERSION);
  });

  it('does not treat @vitest/eslint-plugin as runner usage', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: {
          'vite-plus': 'latest',
          '@vitest/eslint-plugin': '^1.6.0',
          '@vitest/utils': '^4.1.8',
          vitest: '4.1.8',
        },
        overrides: {
          vite: 'npm:@voidzero-dev/vite-plus-core@latest',
          vitest: '4.1.8',
        },
        devEngines: {
          packageManager: { name: 'npm', version: '10.33.0', onFail: 'download' },
        },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'eslint.config.js'),
      "import vitest from '@vitest/eslint-plugin';\nimport { diff } from '@vitest/utils';\nexport default [vitest.configs.recommended, diff];\n",
    );

    ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, PackageManager.npm));

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      devDependencies: Record<string, string>;
      overrides: Record<string, string>;
    };
    expect(pkg.devDependencies['@vitest/eslint-plugin']).toBe('^1.6.0');
    expect(pkg.devDependencies['@vitest/utils']).toBe(VITEST_VERSION);
    expect(pkg.devDependencies.vitest).toBeUndefined();
    expect(pkg.overrides.vitest).toBeUndefined();
  });

  it('reconciles vitest and vite-plus in the workspace package that needs them', () => {
    const appDir = path.join(tmpDir, 'packages/app');
    fs.mkdirSync(appDir, { recursive: true });
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'root',
        private: true,
        devDependencies: { 'vite-plus': 'catalog:' },
        devEngines: {
          packageManager: { name: 'pnpm', version: '10.33.0', onFail: 'download' },
        },
      }),
    );
    fs.writeFileSync(
      path.join(appDir, 'package.json'),
      JSON.stringify({
        name: 'app',
        devDependencies: {
          'vite-plus': '^0.1.24',
          vitest: 'npm:@voidzero-dev/vite-plus-test@^0.1.24',
          '@vitest/ui': '^4.1.8',
        },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      [
        'packages:',
        '  - packages/*',
        'catalog:',
        '  vite-plus: latest',
        '  vite: npm:@voidzero-dev/vite-plus-core@latest',
        'overrides:',
        "  vite: 'catalog:'",
        'peerDependencyRules:',
        '  allowAny: [vite]',
        '  allowedVersions:',
        "    vite: '*'",
        '',
      ].join('\n'),
    );
    const workspaceInfo = {
      ...makeWorkspaceInfo(tmpDir, PackageManager.pnpm),
      isMonorepo: true,
      workspacePatterns: ['packages/*'],
      packages: [{ name: 'app', path: 'packages/app' }],
    };

    expect(
      detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm, workspaceInfo.packages),
    ).toBe(true);
    ensureVitePlusBootstrap(workspaceInfo);

    const rootPkg = readJson(path.join(tmpDir, 'package.json')) as {
      devDependencies: Record<string, string>;
    };
    const appPkg = readJson(path.join(appDir, 'package.json')) as {
      devDependencies: Record<string, string>;
    };
    expect(rootPkg.devDependencies.vitest).toBeUndefined();
    expect(appPkg.devDependencies['vite-plus']).toBe('catalog:');
    expect(appPkg.devDependencies['@vitest/ui']).toBe(VITEST_VERSION);
    expect(appPkg.devDependencies.vitest).toBe('catalog:');
    expect(JSON.stringify(appPkg)).not.toContain('@voidzero-dev/vite-plus-test');
    expect(
      detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm, workspaceInfo.packages),
    ).toBe(false);
  });

  it('restores an opt-in browser provider used only through a Vite+ shim', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'browser-app',
        devDependencies: { 'vite-plus': 'catalog:' },
        devEngines: {
          packageManager: { name: 'pnpm', version: '10.33.0', onFail: 'download' },
        },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'vite.config.ts'),
      [
        "import { defineConfig } from 'vite-plus';",
        "import { playwright } from 'vite-plus/test/browser-playwright';",
        'export default defineConfig({ test: { browser: { enabled: true, provider: playwright() } } });',
      ].join('\n'),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      [
        'catalog:',
        '  vite-plus: latest',
        '  vite: npm:@voidzero-dev/vite-plus-core@latest',
        'overrides:',
        "  vite: 'catalog:'",
        'peerDependencyRules:',
        '  allowAny: [vite]',
        '  allowedVersions:',
        "    vite: '*'",
        '',
      ].join('\n'),
    );

    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm)).toBe(true);
    ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, PackageManager.pnpm));

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      devDependencies: Record<string, string>;
    };
    expect(pkg.devDependencies['@vitest/browser-playwright']).toBe(VITEST_VERSION);
    expect(pkg.devDependencies.playwright).toBe('*');
    expect(pkg.devDependencies.vitest).toBe('catalog:');
    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm)).toBe(false);
  });

  it('removes a stale vitest wrapper override for a common-case npm project (no @vitest/* dep, no vitest source)', () => {
    // v0.2.1 spec: vite-plus consumes upstream vitest directly, so a project that
    // does NOT use vitest directly must NOT carry a managed `vitest` override —
    // it arrives transitively through vite-plus. A pre-existing stale wrapper
    // override (`npm:@voidzero-dev/vite-plus-test@*`) is REMOVED entirely while
    // the `vite` alias stays. The bootstrap is idempotent: a second detect is
    // false.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { 'vite-plus': 'latest' },
        overrides: {
          vite: 'npm:@voidzero-dev/vite-plus-core@latest',
          vitest: 'npm:@voidzero-dev/vite-plus-test@latest',
        },
        devEngines: {
          packageManager: { name: 'npm', version: '10.33.0', onFail: 'download' },
        },
      }),
    );

    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.npm)).toBe(true);
    ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, PackageManager.npm));

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      overrides: Record<string, string>;
    };
    expect(pkg.overrides.vitest).toBeUndefined();
    expect(pkg.overrides.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.npm)).toBe(false);
  });

  it('keeps vitest managed for a direct-usage npm project (@vitest/coverage-v8) and aligns coverage', () => {
    // The project lists `@vitest/coverage-v8`, so it USES vitest directly: the
    // managed `vitest` override is kept (re-pinned to the bundled vitest version,
    // off the stale wrapper) AND the coverage provider is aligned to that version.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: {
          'vite-plus': 'latest',
          '@vitest/coverage-v8': '^4.1.8',
        },
        overrides: {
          vite: 'npm:@voidzero-dev/vite-plus-core@latest',
          vitest: 'npm:@voidzero-dev/vite-plus-test@latest',
        },
        devEngines: {
          packageManager: { name: 'npm', version: '10.33.0', onFail: 'download' },
        },
      }),
    );

    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.npm)).toBe(true);
    ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, PackageManager.npm));

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      devDependencies: Record<string, string>;
      overrides: Record<string, string>;
    };
    // vitest stays managed (the stale wrapper is re-pinned to the bundled version).
    expect(pkg.overrides.vitest).toBe(VITEST_VERSION);
    // Coverage provider aligned to the same bundled vitest version.
    expect(pkg.devDependencies['@vitest/coverage-v8']).toBe(VITEST_VERSION);
    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.npm)).toBe(false);
  });

  it('removes managed vitest catalog/override/peer entries from pnpm-workspace.yaml in the common case', () => {
    // pnpm-workspace.yaml common-case removal: a project with no @vitest/* dep
    // and no vitest source must have every managed `vitest` entry (catalog,
    // override, peer rule) stripped from the workspace file so vitest resolves
    // transitively through vite-plus.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { 'vite-plus': 'catalog:' },
        devEngines: {
          packageManager: { name: 'pnpm', version: '10.33.0', onFail: 'download' },
        },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      [
        'catalog:',
        '  vite: npm:@voidzero-dev/vite-plus-core@latest',
        '  vitest: npm:@voidzero-dev/vite-plus-test@latest',
        '  vite-plus: latest',
        'overrides:',
        "  vite: 'catalog:'",
        "  vitest: 'catalog:'",
        'peerDependencyRules:',
        '  allowAny:',
        '    - vite',
        '    - vitest',
        '  allowedVersions:',
        "    vite: '*'",
        "    vitest: '*'",
        '',
      ].join('\n'),
    );

    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm)).toBe(true);
    ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, PackageManager.pnpm));

    const workspace = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      catalog: Record<string, string>;
      overrides: Record<string, string>;
      peerDependencyRules: { allowAny: string[]; allowedVersions: Record<string, string> };
    };
    // Managed `vitest` is gone from every sink; `vite` stays managed.
    expect(workspace.catalog.vitest).toBeUndefined();
    expect(workspace.catalog.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    expect(workspace.overrides.vitest).toBeUndefined();
    expect(workspace.overrides.vite).toBe('catalog:');
    expect(workspace.peerDependencyRules.allowAny).toEqual(['vite']);
    expect(workspace.peerDependencyRules.allowedVersions).toEqual({ vite: '*' });
    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm)).toBe(false);
  });

  it('re-pins a behind vite-plus spec so the upgrade moves off the old version (urllib)', () => {
    // urllib pinned vite-plus to a concrete 0.1.x range. A spec that stays at
    // ^0.1.24 keeps the lockfile on the old resolution; the upgrade must re-pin
    // it to the migrating toolchain target (here the mocked VITE_PLUS_VERSION
    // 'latest', materialized as `catalog:` in a pnpm-workspace.yaml project) so
    // the reinstall resolves the new version.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'urllib',
        devDependencies: {
          'vite-plus': '^0.1.24',
          vite: 'npm:@voidzero-dev/vite-plus-core@^0.1.24',
          vitest: 'npm:@voidzero-dev/vite-plus-test@^0.1.24',
        },
        pnpm: {},
        devEngines: {
          packageManager: { name: 'pnpm', version: '11.7.0', onFail: 'download' },
        },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      [
        'overrides:',
        "  vite: 'npm:@voidzero-dev/vite-plus-core@^0.1.24'",
        "  vitest: 'npm:@voidzero-dev/vite-plus-test@^0.1.24'",
      ].join('\n'),
    );

    ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, PackageManager.pnpm));

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      devDependencies: Record<string, string>;
    };
    // vite-plus must no longer be pinned to the old 0.1.x range.
    expect(pkg.devDependencies['vite-plus']).not.toContain('0.1.24');
    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm)).toBe(false);
  });

  it('uses a concrete vite-plus version when pnpm config stays in package.json', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        dependencies: { 'vite-plus': 'latest' },
        pnpm: {},
      }),
    );

    const result = ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, PackageManager.pnpm));

    expect(result.changed).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, 'pnpm-workspace.yaml'))).toBe(false);
    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      devDependencies: Record<string, string>;
      pnpm: { overrides: Record<string, string> };
    };
    expect(pkg.devDependencies['vite-plus']).toBe('latest');
    expect(pkg.pnpm.overrides.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
  });

  it('normalizes an existing catalog vite-plus pin when pnpm config stays in package.json', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { 'vite-plus': 'catalog:' },
        devEngines: {
          packageManager: { name: 'pnpm', version: '10.33.0', onFail: 'download' },
        },
        pnpm: {
          overrides: {
            vite: 'npm:@voidzero-dev/vite-plus-core@latest',
            vitest: 'npm:@voidzero-dev/vite-plus-test@latest',
          },
          peerDependencyRules: {
            allowAny: ['vite', 'vitest'],
            allowedVersions: { vite: '*', vitest: '*' },
          },
        },
      }),
    );

    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm)).toBe(true);
    const result = ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, PackageManager.pnpm));

    expect(result.changed).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, 'pnpm-workspace.yaml'))).toBe(false);
    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      devDependencies: Record<string, string>;
    };
    expect(pkg.devDependencies['vite-plus']).toBe('latest');
    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm)).toBe(false);
  });

  it('normalizes catalog vite-plus pins outside devDependencies when pnpm config stays in package.json', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        dependencies: { 'vite-plus': 'catalog:' },
        optionalDependencies: { 'vite-plus': 'catalog:' },
        devEngines: {
          packageManager: { name: 'pnpm', version: '10.33.0', onFail: 'download' },
        },
        pnpm: {
          overrides: {
            vite: 'npm:@voidzero-dev/vite-plus-core@latest',
            vitest: 'npm:@voidzero-dev/vite-plus-test@latest',
          },
          peerDependencyRules: {
            allowAny: ['vite', 'vitest'],
            allowedVersions: { vite: '*', vitest: '*' },
          },
        },
      }),
    );

    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm)).toBe(true);
    const result = ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, PackageManager.pnpm));

    expect(result.changed).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, 'pnpm-workspace.yaml'))).toBe(false);
    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      devDependencies: Record<string, string>;
      dependencies: Record<string, string>;
      optionalDependencies: Record<string, string>;
    };
    expect(pkg.devDependencies['vite-plus']).toBe('latest');
    expect(pkg.dependencies['vite-plus']).toBe('latest');
    expect(pkg.optionalDependencies['vite-plus']).toBe('latest');
    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm)).toBe(false);
  });

  it('uses a concrete vite-plus version for pnpm monorepos that keep pnpm config in package.json', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        dependencies: { 'vite-plus': 'latest' },
        pnpm: {},
      }),
    );

    const result = ensureVitePlusBootstrap({
      ...makeWorkspaceInfo(tmpDir, PackageManager.pnpm),
      isMonorepo: true,
      workspacePatterns: ['packages/*'],
    });

    expect(result.changed).toBe(true);
    expect(fs.existsSync(path.join(tmpDir, 'pnpm-workspace.yaml'))).toBe(false);
    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      devDependencies: Record<string, string>;
    };
    expect(pkg.devDependencies['vite-plus']).toBe('latest');
  });

  it('normalizes yarn monorepo dependency specs through the shared catalog', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { 'vite-plus': 'latest', vite: '^7.0.0' },
        devEngines: {
          packageManager: { name: 'yarn', version: '4.0.0', onFail: 'download' },
        },
      }),
    );

    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.yarn)).toBe(true);
    const result = ensureVitePlusBootstrap({
      ...makeWorkspaceInfo(tmpDir, PackageManager.yarn),
      isMonorepo: true,
      workspacePatterns: ['packages/*'],
    });

    expect(result.changed).toBe(true);
    expect(result.packageManagerConfig).toBe(true);
    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.yarn)).toBe(false);
    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      devDependencies: Record<string, string>;
      resolutions: Record<string, string>;
    };
    expect(pkg.devDependencies.vite).toBe('catalog:');
    expect(pkg.devDependencies['vite-plus']).toBe('catalog:');
    expect(pkg.resolutions.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    const yarnrc = readYamlObject(path.join(tmpDir, '.yarnrc.yml')) as {
      nodeLinker: string;
      catalog: Record<string, string>;
    };
    expect(yarnrc.nodeLinker).toBe('node-modules');
    expect(yarnrc.catalog.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    // Common case (no @vitest/* dep, no vitest source): `vitest` is not managed,
    // so no catalog entry is written for it.
    expect(yarnrc.catalog.vitest).toBeUndefined();
    expect(yarnrc.catalog['vite-plus']).toBe('latest');
  });

  it('completes missing pnpm workspace peer dependency rules', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { 'vite-plus': 'catalog:' },
        devEngines: {
          packageManager: { name: 'pnpm', version: '10.33.0', onFail: 'download' },
        },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      [
        'catalog:',
        '  vite: npm:@voidzero-dev/vite-plus-core@latest',
        '  vitest: npm:@voidzero-dev/vite-plus-test@latest',
        '  vite-plus: latest',
        'overrides:',
        "  vite: 'catalog:'",
        "  vitest: 'catalog:'",
        '',
      ].join('\n'),
    );

    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm)).toBe(true);
    const result = ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, PackageManager.pnpm));

    expect(result.changed).toBe(true);
    expect(detectVitePlusBootstrapPending(tmpDir, PackageManager.pnpm)).toBe(false);
    // Common case (no @vitest/* dep, no vitest source): the pre-existing managed
    // `vitest` catalog/override/peer entries are REMOVED — only `vite` stays
    // managed. vitest arrives transitively through vite-plus.
    const workspace = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      catalog: Record<string, string>;
      overrides: Record<string, string>;
      peerDependencyRules: { allowAny: string[]; allowedVersions: Record<string, string> };
    };
    expect(workspace.catalog.vitest).toBeUndefined();
    expect(workspace.overrides.vitest).toBeUndefined();
    expect(workspace.peerDependencyRules.allowAny).toEqual(['vite']);
    expect(workspace.peerDependencyRules.allowedVersions).toEqual({
      vite: '*',
    });
  });

  it('exempts the vitest family from a pnpm minimumReleaseAge gate, preserving existing entries', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { 'vite-plus': 'catalog:' },
        devEngines: {
          packageManager: { name: 'pnpm', version: '10.33.0', onFail: 'download' },
        },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      [
        'catalog:',
        '  vite: npm:@voidzero-dev/vite-plus-core@latest',
        '  vitest: npm:@voidzero-dev/vite-plus-test@latest',
        'minimumReleaseAge: 1440',
        'minimumReleaseAgeExclude:',
        '  - my-internal-pkg',
        '',
      ].join('\n'),
    );

    ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, PackageManager.pnpm));

    const workspace = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      minimumReleaseAgeExclude: string[];
    };
    // Vite+ pins a possibly-fresh vitest; its family must bypass the gate.
    expect(workspace.minimumReleaseAgeExclude).toContain('vitest');
    expect(workspace.minimumReleaseAgeExclude).toContain('@vitest/*');
    // The user's own entry and the vite-plus/ox families are preserved.
    expect(workspace.minimumReleaseAgeExclude).toContain('my-internal-pkg');
    expect(workspace.minimumReleaseAgeExclude).toContain('vite-plus');
  });

  it('does not add minimumReleaseAgeExclude when the pnpm age gate is absent', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { 'vite-plus': 'catalog:' },
        devEngines: {
          packageManager: { name: 'pnpm', version: '10.33.0', onFail: 'download' },
        },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      [
        'catalog:',
        '  vite: npm:@voidzero-dev/vite-plus-core@latest',
        '  vitest: npm:@voidzero-dev/vite-plus-test@latest',
        '',
      ].join('\n'),
    );

    ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, PackageManager.pnpm));

    const workspace = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      minimumReleaseAgeExclude?: string[];
    };
    // No gate present → we never introduce the exclude list.
    expect(workspace.minimumReleaseAgeExclude).toBeUndefined();
  });

  it('merges the vitest family into an existing yarn npmPreapprovedPackages list', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { 'vite-plus': 'latest' },
        devEngines: {
          packageManager: { name: 'yarn', version: '4.0.0', onFail: 'download' },
        },
      }),
    );
    // A project that already preapproves private packages must keep them.
    fs.writeFileSync(
      path.join(tmpDir, '.yarnrc.yml'),
      ['npmPreapprovedPackages:', '  - "@my-org/*"', ''].join('\n'),
    );

    ensureVitePlusBootstrap(makeWorkspaceInfo(tmpDir, PackageManager.yarn));

    const yarnrc = readYamlObject(path.join(tmpDir, '.yarnrc.yml')) as {
      npmPreapprovedPackages: string[];
    };
    expect(yarnrc.npmPreapprovedPackages).toContain('@my-org/*');
    expect(yarnrc.npmPreapprovedPackages).toContain('vitest');
    expect(yarnrc.npmPreapprovedPackages).toContain('@vitest/*');
  });

  it('preserves package.json workspace patterns when creating pnpm-workspace.yaml', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        workspaces: ['packages/*'],
        devDependencies: { 'vite-plus': 'catalog:' },
      }),
    );

    const result = ensureVitePlusBootstrap({
      ...makeWorkspaceInfo(tmpDir, PackageManager.pnpm),
      isMonorepo: true,
      workspacePatterns: ['packages/*'],
    });

    expect(result.changed).toBe(true);
    const workspace = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      packages: string[];
    };
    expect(workspace.packages).toEqual(['packages/*']);
  });
});

describe('rewriteStandaloneProject pnpm workspace yaml', () => {
  let tmpDir: string;
  const savedEnv: Record<string, string | undefined> = {};
  // Env keys the Yarn-hoisting resolver consults at HIGHEST precedence. Clear them in
  // setup so an ambient `YARN_NODE_LINKER=pnp` (etc.) in the runner's environment can't
  // override the fixture `.yarnrc.yml` values and make these tests non-hermetic; the
  // env-precedence tests set them explicitly. `HOME`/`USERPROFILE` both matter because
  // `os.homedir()` reads `HOME` on POSIX but `USERPROFILE` on Windows — set both so the
  // home-rc lookup is redirected on every platform.
  const ISOLATED_ENV = [
    'HOME',
    'USERPROFILE',
    'YARN_NODE_LINKER',
    'YARN_NM_HOISTING_LIMITS',
  ] as const;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-pnpm-'));
    for (const key of ISOLATED_ENV) {
      savedEnv[key] = process.env[key];
      delete process.env[key];
    }
    // Point Yarn's home `.yarnrc.yml` (the lowest-precedence config source the resolver
    // consults) at a clean, empty dir so these tests can't read a contributor's real
    // ~/.yarnrc.yml. Tests that need a home rc set the home env vars themselves.
    const cleanHome = path.join(tmpDir, '.home');
    fs.mkdirSync(cleanHome, { recursive: true });
    process.env.HOME = cleanHome;
    process.env.USERPROFILE = cleanHome;
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
    for (const key of ISOLATED_ENV) {
      if (savedEnv[key] === undefined) {
        delete process.env[key];
      } else {
        process.env[key] = savedEnv[key];
      }
    }
  });

  it('creates pnpm-workspace.yaml when no existing pnpm config in package.json', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'test', devDependencies: { vite: '^7.0.0' } }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    // pnpm-workspace.yaml should be created
    expect(fs.existsSync(path.join(tmpDir, 'pnpm-workspace.yaml'))).toBe(true);
    const yaml = readYaml(path.join(tmpDir, 'pnpm-workspace.yaml'));
    expect(yaml).toContain('overrides:');
    expect(yaml).toContain('peerDependencyRules:');
    expect(yaml).toContain('catalog:');

    // package.json should not have pnpm section
    const pkg = readJson(path.join(tmpDir, 'package.json'));
    expect(pkg.pnpm).toBeUndefined();

    // devDependencies should use catalog:
    const devDeps = pkg.devDependencies as Record<string, string>;
    expect(devDeps.vite).toBe('catalog:');
    expect(devDeps['vite-plus']).toBe('catalog:');
  });

  it('keeps pnpm config in package.json when existing pnpm field present', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0' },
        pnpm: {
          overrides: { 'some-pkg': '1.0.0' },
          onlyBuiltDependencies: ['esbuild'],
        },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    // pnpm-workspace.yaml should NOT be created
    expect(fs.existsSync(path.join(tmpDir, 'pnpm-workspace.yaml'))).toBe(false);

    // package.json should have pnpm.overrides with both existing and vite overrides
    const pkg = readJson(path.join(tmpDir, 'package.json'));
    const pnpm = pkg.pnpm as Record<string, unknown>;
    expect(pnpm).toBeDefined();
    const overrides = pnpm.overrides as Record<string, string>;
    expect(overrides['some-pkg']).toBe('1.0.0');
    expect(overrides.vite).toBeDefined();
    // Common case (no @vitest/* dep, no vitest source): `vitest` is not managed,
    // so no override is written — it arrives transitively through vite-plus.
    expect(overrides.vitest).toBeUndefined();

    // peerDependencyRules should be present
    expect(pnpm.peerDependencyRules).toBeDefined();
    // onlyBuiltDependencies should be preserved
    expect(pnpm.onlyBuiltDependencies).toEqual(['esbuild']);
  });

  it('preserves custom peerDependencyRules when migrating to pnpm-workspace.yaml', () => {
    // Project has peerDependencyRules but no pnpm.overrides -- pnpm field is present
    // so it should keep using package.json
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0' },
        pnpm: {
          peerDependencyRules: {
            allowAny: ['react', 'vite'],
            allowedVersions: { react: '*', vite: '*' },
            ignoreMissing: ['@types/node'],
          },
        },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json'));
    const pnpm = pkg.pnpm as Record<string, unknown>;
    const rules = pnpm.peerDependencyRules as Record<string, unknown>;
    // Custom entries preserved, Vite entries merged (vitest is no longer
    // injected as it's not a managed override key anymore).
    expect(rules.allowAny).toEqual(expect.arrayContaining(['react', 'vite']));
    // ignoreMissing preserved
    expect(rules.ignoreMissing).toEqual(['@types/node']);
  });

  it('writes vite overrides with catalog references to pnpm-workspace.yaml', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'test', devDependencies: { vite: '^7.0.0' } }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const yaml = readYaml(path.join(tmpDir, 'pnpm-workspace.yaml'));
    expect(yaml).toContain("vite: 'catalog:'");
    // Common case (no @vitest/* dep, no vitest source): `vitest` is not managed,
    // so no `vitest` override is written — it arrives transitively through
    // vite-plus.
    expect(yaml).not.toContain('vitest');
  });

  it('rewrites named catalogs in pnpm-workspace.yaml without adding new entries', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: 'catalog:vite7' },
        peerDependencies: {
          vite: 'catalog:vite7',
          vitest: 'catalog:',
          tsdown: 'catalog:test',
        },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      [
        'overrides:',
        '  vite: catalog:vite7',
        'catalog:',
        '  vitest: ^4.0.0',
        'catalogs:',
        '  vite7:',
        '    react: ^18.0.0',
        '    vite: ^7.0.0',
        '    vite-plus: ^0.0.0',
        '  test:',
        '    vitest: ^4.0.0',
        '    tsdown: ^0.1.0',
        '',
      ].join('\n'),
    );

    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      catalog: Record<string, string>;
      overrides: Record<string, string>;
      catalogs: Record<string, Record<string, string>>;
    };
    expect(yaml.overrides.vite).toBe('catalog:vite7');
    // Common case (no @vitest/* dep, no vitest source): `vitest` is not managed,
    // so no override is added and the pre-existing managed `vitest` catalog
    // entries (default + named) are REMOVED — it arrives transitively through
    // vite-plus.
    expect(yaml.overrides.vitest).toBeUndefined();
    expect(yaml.catalog?.vitest).toBeUndefined();
    expect(yaml.catalogs.vite7.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    expect(yaml.catalogs.vite7.react).toBe('^18.0.0');
    expect(yaml.catalogs.vite7['vite-plus']).toBe('latest');
    expect(yaml.catalogs.test.vitest).toBeUndefined();
    expect(yaml.catalogs.test.tsdown).toBeUndefined();
    expect(yaml.catalogs.test['vite-plus']).toBeUndefined();

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      devDependencies: Record<string, string>;
      peerDependencies: Record<string, string>;
    };
    expect(pkg.devDependencies.vite).toBe('catalog:vite7');
    expect(pkg.devDependencies['vite-plus']).toBe('catalog:');
    expect(pkg.peerDependencies.vite).toBe('^7.0.0');
    // `vitest` is no longer a managed override key, so the peer entry is left as
    // the user wrote it (untouched).
    expect(pkg.peerDependencies.vitest).toBe('catalog:');
    expect(pkg.peerDependencies).not.toHaveProperty('tsdown');
  });

  it('drops only global/vite-plus-parent selector-shaped REMOVE_PACKAGES overrides from package.json pnpm.overrides', () => {
    // Project keeps its pnpm config in package.json (`pkg.pnpm.overrides`).
    // A selector-shaped provider key is stripped only when it would re-pin
    // vite-plus's OWN provider dep — a versioned global pin or a `vite-plus`
    // parent. A provider selector scoped under a SPECIFIC non-vite-plus parent
    // (`some-app>@vitest/browser-playwright`) only constrains that parent's
    // subtree and is preserved.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0' },
        pnpm: {
          overrides: {
            'vite-plus>@vitest/browser-playwright': '^4.0.0',
            'some-app>@vitest/browser-playwright': '^4.0.0',
            'a>vite-plus>@vitest/browser-playwright': '^4.0.0',
            '@vitest/browser-playwright@4': '4.1.7',
            // `@vitest/browser-preview` stays in REMOVE_PACKAGES, so it remains a
            // vite-plus-OWNED provider ancestor: an un-anchored chain through it
            // still constrains vite-plus's own `@vitest/browser` dep — dropped.
            '@vitest/browser-preview>@vitest/browser': '4.0.0',
            'vite-plus>@vitest/browser-preview>@vitest/browser': '4.0.0',
            'some-app>@vitest/browser-preview>@vitest/browser': '4.0.0',
            // Playwright is now opt-in (NOT owned by vite-plus), so an un-anchored
            // chain PARENTED by playwright constrains the user's own provider
            // subtree, not vite-plus's — PRESERVED.
            '@vitest/browser-playwright>@vitest/browser': '4.0.0',
            'vite-plus>@vitest/browser-playwright>@vitest/browser': '4.0.0',
            'other>foo': '1.0.0',
          },
        },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      pnpm?: { overrides?: Record<string, string> };
    };
    const overrides = pkg.pnpm?.overrides ?? {};
    // Playwright-as-TARGET: vite-plus parent and versioned global pin reach
    // vite-plus's own (now direct-dep) provider — dropped.
    expect(overrides).not.toHaveProperty('vite-plus>@vitest/browser-playwright');
    expect(overrides).not.toHaveProperty('@vitest/browser-playwright@4');
    // An OWNED-provider ancestor (`@vitest/browser-preview`) still constrains
    // vite-plus's provider subtree (selectors are not root-anchored), with or
    // without an explicit vite-plus prefix — dropped.
    expect(overrides).not.toHaveProperty('@vitest/browser-preview>@vitest/browser');
    expect(overrides).not.toHaveProperty('vite-plus>@vitest/browser-preview>@vitest/browser');
    // Provider-as-TARGET selector scoped under a SPECIFIC non-vite-plus parent —
    // PRESERVED.
    expect(overrides['some-app>@vitest/browser-playwright']).toBe('^4.0.0');
    expect(overrides['some-app>@vitest/browser-preview>@vitest/browser']).toBe('4.0.0');
    // Playwright is opt-in (not a vite-plus-owned ancestor), so a chain PARENTED
    // by it constrains the user's own subtree — PRESERVED even with a vite-plus
    // prefix above it.
    expect(overrides['@vitest/browser-playwright>@vitest/browser']).toBe('4.0.0');
    expect(overrides['vite-plus>@vitest/browser-playwright>@vitest/browser']).toBe('4.0.0');
    // A chain with an outer non-vite-plus ancestor (`a>vite-plus>…`) requires
    // vite-plus to sit UNDER `a`, so it never matches the root vite-plus edge —
    // PRESERVED.
    expect(overrides['a>vite-plus>@vitest/browser-playwright']).toBe('^4.0.0');
    // Unrelated selector keys must survive.
    expect(overrides['other>foo']).toBe('1.0.0');
  });

  it('drops a vite-plus-scoped provider pin and prunes the emptied vite-plus parent', () => {
    // A provider pin nested under a `vite-plus` parent forces vite-plus's own
    // (now direct-dep) provider, so it must be dropped. Removing the sole pin
    // empties the `vite-plus` parent, which is then pruned.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0' },
        overrides: {
          'vite-plus': { '@vitest/browser-playwright': '4.0.0' },
          // An OWNED-provider parent (`@vitest/browser-preview`, still bundled by
          // vite-plus) reaches vite-plus's provider subtree even without an
          // explicit vite-plus level — its child pin is dropped and the emptied
          // parent pruned with it.
          '@vitest/browser-preview': { '@vitest/browser': '4.0.0' },
        },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.npm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      overrides?: Record<string, unknown>;
    };
    const overrides = pkg.overrides ?? {};
    // `vite-plus` parent (with playwright-as-target child) is dropped and pruned.
    expect(overrides).not.toHaveProperty('vite-plus');
    // Owned-provider parent emptied by dropping its child pin is pruned too.
    expect(overrides).not.toHaveProperty('@vitest/browser-preview');
  });

  it('preserves a provider override scoped under an unrelated parent', () => {
    // npm/bun nested overrides are SCOPED: a provider pin under `some-pkg`
    // forces the provider only within some-pkg's subtree, NOT vite-plus's own
    // provider dep. Deleting it would be silent loss of the user's unrelated
    // override, so it (and its parent) must survive untouched.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0' },
        overrides: {
          'some-pkg': { '@vitest/browser-playwright': '4.0.0' },
        },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.npm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      overrides?: Record<string, Record<string, string>>;
    };
    const overrides = pkg.overrides ?? {};
    expect(overrides).toHaveProperty('some-pkg');
    expect(overrides['some-pkg']['@vitest/browser-playwright']).toBe('4.0.0');
  });

  it('aligns already-declared coverage providers without adding provider overrides', () => {
    // Coverage providers have an exact vitest peer and must match the runner.
    // Align their dependency specs directly; no provider override is needed.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: {
          vite: '^7.0.0',
          vitest: '^4.0.0',
          '@vitest/coverage-v8': '^4.0.0',
          '@vitest/coverage-istanbul': '^4.0.0',
        },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.npm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      devDependencies: Record<string, string>;
      overrides?: Record<string, unknown>;
    };
    expect(pkg.devDependencies['@vitest/coverage-v8']).toBe(VITEST_VERSION);
    expect(pkg.devDependencies['@vitest/coverage-istanbul']).toBe(VITEST_VERSION);
    // vitest itself is still pinned to the bundled version.
    expect(pkg.devDependencies.vitest).toBe(VITEST_VERSION);
    // …and coverage is never written into the override sink.
    const overrides = pkg.overrides ?? {};
    expect(overrides['@vitest/coverage-v8']).toBeUndefined();
    expect(overrides['@vitest/coverage-istanbul']).toBeUndefined();
  });

  it('removes direct vitest in the same pass that rewrites ordinary vitest imports', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0', vitest: '^4.0.0' },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'example.spec.ts'),
      "import { expect, it } from 'vitest';\nit('works', () => expect(true).toBe(true));\n",
    );

    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.npm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      devDependencies: Record<string, string>;
      overrides: Record<string, string>;
    };
    expect(pkg.devDependencies.vitest).toBeUndefined();
    expect(pkg.overrides.vitest).toBeUndefined();
    expect(fs.readFileSync(path.join(tmpDir, 'example.spec.ts'), 'utf8')).toContain(
      "from 'vite-plus/test'",
    );
  });

  it('does not add a coverage provider the project never declared', () => {
    // A project that uses vitest WITHOUT a coverage provider must not have one
    // injected by the migration — the user installs it only if they need it.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0', vitest: '^4.0.0' },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.npm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      devDependencies: Record<string, string>;
    };
    expect(pkg.devDependencies['@vitest/coverage-v8']).toBeUndefined();
    expect(pkg.devDependencies['@vitest/coverage-istanbul']).toBeUndefined();
  });

  it('drops a vite-plus-scoped provider pin while keeping non-provider siblings', () => {
    // Inside a `vite-plus` subtree only the provider pin is dropped; an
    // unrelated sibling (`lodash`) keeps the `vite-plus` parent alive.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0' },
        overrides: {
          'vite-plus': { '@vitest/browser-playwright': '4.0.0', lodash: '4.17.0' },
        },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.npm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      overrides?: Record<string, Record<string, string>>;
    };
    const overrides = pkg.overrides ?? {};
    expect(overrides).toHaveProperty('vite-plus');
    expect(overrides['vite-plus']).not.toHaveProperty('@vitest/browser-playwright');
    expect(overrides['vite-plus'].lodash).toBe('4.17.0');
  });

  it('drops a top-level global provider pin', () => {
    // A TOP-LEVEL provider pin is a global override that reaches vite-plus's
    // bundled copy, so it must be dropped (regression guard).
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0' },
        overrides: {
          '@vitest/browser-playwright': '4.0.0',
        },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.npm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      overrides?: Record<string, unknown>;
    };
    const overrides = pkg.overrides ?? {};
    expect(overrides).not.toHaveProperty('@vitest/browser-playwright');
  });

  it('drops a long-form top-level provider self-pin but keeps unrelated children', () => {
    // A long-form provider override pins the provider's own version via the `.`
    // self-key; that pin is dropped (it reaches vite-plus's bundled copy) while
    // unrelated scoped children (`bar`) are preserved.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0' },
        overrides: {
          '@vitest/browser-playwright': { '.': '4.0.0', bar: '1.0.0' },
        },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.npm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      overrides?: Record<string, Record<string, string>>;
    };
    const overrides = pkg.overrides ?? {};
    expect(overrides).toHaveProperty('@vitest/browser-playwright');
    const provider = overrides['@vitest/browser-playwright'];
    // The provider's own version pin (`.`) is dropped; the `.` self-key must
    // be asserted via `in` (Jest's `toHaveProperty('.')` treats `.` as a path
    // separator and would not match the literal key).
    expect('.' in provider).toBe(false);
    expect(provider.bar).toBe('1.0.0');
  });

  it('preserves a provider pin nested under an outer non-vite-plus ancestor', () => {
    // `{ a: { vite-plus: { provider } } }` forces the provider only for the
    // vite-plus instance that sits UNDER `a` — NOT the project's own (root)
    // vite-plus direct dep. It is the npm/bun nested equivalent of the flat pnpm
    // `a>vite-plus>provider` chain, so (like that chain) it must be PRESERVED.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0' },
        overrides: {
          a: { 'vite-plus': { '@vitest/browser-playwright': '4.0.0' } },
        },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.npm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      overrides?: Record<string, Record<string, Record<string, string>>>;
    };
    const overrides = pkg.overrides ?? {};
    expect(overrides).toHaveProperty('a');
    expect(overrides.a['vite-plus']['@vitest/browser-playwright']).toBe('4.0.0');
  });

  it('drops a root vite-plus provider pin nested one level deep and prunes the parent', () => {
    // `{ vite-plus: { provider } }` forces the provider as a direct dep of the
    // root vite-plus, so it IS dropped; the emptied `vite-plus` parent is pruned.
    // Contrast the outer-ancestor case above — only a single-segment `vite-plus`
    // chain reaches the protected edge.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0' },
        overrides: {
          'vite-plus': { '@vitest/browser-playwright': { '.': '4.0.0' } },
        },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.npm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      overrides?: Record<string, unknown>;
    };
    const overrides = pkg.overrides ?? {};
    expect(overrides).not.toHaveProperty('vite-plus');
  });

  it('preserves a deep provider override under unrelated parents', () => {
    // No `vite-plus` parent anywhere on the path: the provider pin is the
    // user's scoped override (`a > b > provider`) and must survive fully.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0' },
        overrides: {
          a: { b: { '@vitest/browser-playwright': '4.0.0' } },
        },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.npm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      overrides?: Record<string, Record<string, Record<string, string>>>;
    };
    const overrides = pkg.overrides ?? {};
    expect(overrides).toHaveProperty('a');
    expect(overrides.a).toHaveProperty('b');
    expect(overrides.a.b['@vitest/browser-playwright']).toBe('4.0.0');
  });

  it('does not over-delete a non-provider override scoped under vite-plus', () => {
    // A non-provider pin (`lodash`) under `vite-plus` is a legitimate user
    // override; descending into the `vite-plus` subtree must leave it untouched.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0' },
        overrides: {
          'vite-plus': { lodash: '4.17.0' },
        },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.npm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      overrides?: Record<string, Record<string, string>>;
    };
    const overrides = pkg.overrides ?? {};
    expect(overrides).toHaveProperty('vite-plus');
    expect(overrides['vite-plus'].lodash).toBe('4.17.0');
  });

  it('leaves a user-authored pre-existing empty override object untouched', () => {
    // We only prune parents WE empty by dropping provider pins. A parent the
    // user authored as already-empty must be preserved as-is even when an
    // unrelated top-level provider key is dropped in the same pass.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0' },
        overrides: {
          'some-pkg': {},
          '@vitest/browser-playwright': '4.0.0',
        },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.npm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      overrides?: Record<string, unknown>;
    };
    const overrides = pkg.overrides ?? {};
    expect(overrides).not.toHaveProperty('@vitest/browser-playwright');
    expect(overrides).toHaveProperty('some-pkg');
    expect(overrides['some-pkg']).toEqual({});
  });

  it('does not crash on a nested object value under a managed bun catalog override key', () => {
    // Bun monorepo: a nested object value under a MANAGED override key (e.g.
    // `vitest`) is a user override scoped under that key, not a version pin.
    // The bun catalog rewrite must not pass it to getCatalogDependencySpec
    // (which calls `.startsWith` and would crash / clobber it to a string).
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'bun-monorepo',
        workspaces: ['packages/*'],
        devDependencies: { vite: '^7.0.0' },
        overrides: {
          vitest: { '@vitest/runner': '4.0.0' },
        },
      }),
    );

    expect(() =>
      rewriteMonorepo(makeWorkspaceInfo(tmpDir, PackageManager.bun), true),
    ).not.toThrow();

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      overrides?: Record<string, unknown>;
    };
    const overrides = pkg.overrides ?? {};
    // The nested override object is left intact, not clobbered to a string.
    expect(overrides.vitest).toEqual({ '@vitest/runner': '4.0.0' });
  });

  it('drops stale @vitest/browser* overrides from pnpm-workspace.yaml', () => {
    // The migration moves provider packages out of project manifests and adds
    // them as direct vite-plus deps. A pre-existing workspace override pinning
    // an old provider version would then force vite-plus's own provider dep to
    // an incompatible version against the bundled vitest.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'test', devDependencies: { vite: '^7.0.0' } }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      [
        'packages:',
        '  - packages/*',
        'overrides:',
        "  '@vitest/browser-playwright': 4.0.0",
        "  '@vitest/browser-webdriverio': 4.0.0",
        "  '@vitest/browser-playwright@4': 4.0.0",
        "  'vite-plus>@vitest/browser-playwright': 4.0.0",
        "  'some-app>@vitest/browser-playwright': 4.0.0",
        '  some-other-pkg: 1.0.0',
        "  'unrelated>some-other-pkg': 1.0.0",
        '',
      ].join('\n'),
    );

    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      overrides: Record<string, string>;
    };
    // Playwright is opt-in: vite-plus keeps it in the user's deps pinned to the
    // bundled vitest version, but a stale override pinning an old version (as a
    // TARGET — bare/versioned/global pin, or a `vite-plus`-parented selector)
    // would win over that direct dep and misalign the provider against bundled
    // vitest — so the stale override is dropped (the dep stays installed, the pin
    // does not).
    expect(yaml.overrides).not.toHaveProperty('@vitest/browser-playwright');
    expect(yaml.overrides).not.toHaveProperty('@vitest/browser-playwright@4');
    expect(yaml.overrides).not.toHaveProperty('vite-plus>@vitest/browser-playwright');
    // A provider-as-TARGET selector scoped under a SPECIFIC non-vite-plus parent
    // only constrains that parent's subtree, so it is PRESERVED.
    expect(yaml.overrides['some-app>@vitest/browser-playwright']).toBe('4.0.0');
    // Webdriverio is opt-in: vite-plus keeps it in the user's deps pinned to the
    // bundled vitest version, but a stale override pinning an old version would
    // win over that direct dep and misalign the provider against bundled vitest —
    // so the stale override is dropped too (the dep stays installed, the pin
    // does not).
    expect(yaml.overrides).not.toHaveProperty('@vitest/browser-webdriverio');
    expect(yaml.overrides['some-other-pkg']).toBe('1.0.0');
    expect(yaml.overrides['unrelated>some-other-pkg']).toBe('1.0.0');
  });

  it('adds a direct vitest dep when a vite config enables browser mode', () => {
    // A package whose vite config imports a browser provider but has no direct
    // vitest dep — `@vitest/browser` needs `vitest` resolvable from the package
    // root, so the migration must pin it. Mirrors the vibe-dashboard regression.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'test', devDependencies: { playwright: '^1.58.0' } }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'vite.config.ts'),
      [
        "import { playwright } from 'vite-plus/test/browser-playwright';",
        "import { defineConfig } from 'vite-plus';",
        'export default defineConfig({',
        '  test: { browser: { enabled: true, provider: playwright() } },',
        '});',
        '',
      ].join('\n'),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json'));
    const devDeps = pkg.devDependencies as Record<string, string>;
    expect(devDeps.vitest).toBe('catalog:');
    expect(devDeps['vite-plus']).toBe('catalog:');
  });

  it('detects browser mode from a test file when the config has no hint', () => {
    // Browser config can live in a workspace-referenced config under any name;
    // the source scan also catches `@vitest/browser*` imports in test files.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'test', devDependencies: { vite: '^7.0.0' } }),
    );
    fs.mkdirSync(path.join(tmpDir, 'src', '__tests__'), { recursive: true });
    fs.writeFileSync(
      path.join(tmpDir, 'src', '__tests__', 'app.test.ts'),
      "import { page } from '@vitest/browser/context';\n",
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const devDeps = readJson(path.join(tmpDir, 'package.json')).devDependencies as Record<
      string,
      string
    >;
    expect(devDeps.vitest).toBe('catalog:');
  });

  // Published browser surfaces whose specifier carries NO `vite-plus/test/browser`
  // substring must still be detected as browser mode, so migration pins the
  // direct `vitest` the optimizer needs resolvable from the package root under
  // pnpm strict / Yarn PnP. Two families:
  //   - the bare browser shims `vite-plus/test/{client,context,locators,matchers,
  //     utils}` (build.ts createBareBrowserShims; the rewriter flattens four of
  //     them, `context` is the published bare export), and
  //   - the generated plugin shims `vite-plus/test/plugins/browser*` (build.ts
  //     PLUGIN_SHIM_ENTRIES) sitting under a `/plugins/` segment, and
  //   - the published internal shim `vite-plus/test/internal/browser`
  //     (re-exports `vitest/internal/browser`).
  // Each is a browser surface yet a package importing only one of them with no
  // `@vitest/browser*` dep must get a direct `vitest` (and must NOT gain an
  // injected `@vitest/browser`).
  for (const subpath of [
    'client',
    'context',
    'locators',
    'matchers',
    'utils',
    'plugins/browser',
    'plugins/browser-context',
    'plugins/browser-playwright',
    'internal/browser',
  ] as const) {
    it(`detects browser mode from the published \`vite-plus/test/${subpath}\` shim`, () => {
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({ name: 'test', devDependencies: { vite: '^7.0.0' } }),
      );
      fs.mkdirSync(path.join(tmpDir, 'src', '__tests__'), { recursive: true });
      fs.writeFileSync(
        path.join(tmpDir, 'src', '__tests__', 'app.test.ts'),
        `import { thing } from 'vite-plus/test/${subpath}';\n`,
      );
      rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

      const devDeps = readJson(path.join(tmpDir, 'package.json')).devDependencies as Record<
        string,
        string
      >;
      // Browser mode pins a direct `vitest`…
      expect(devDeps.vitest).toBe('catalog:');
      // …but must NOT inject any browser/provider dep the source never asked for.
      expect(devDeps).not.toHaveProperty('@vitest/browser');
    });
  }

  it('injects the webdriverio provider + peer from a source-only vitest config and allows driver builds', () => {
    // Opt-in provider: vite-plus no longer bundles `@vitest/browser-webdriverio`.
    // A project that imports it in source with NO declared dep must have the
    // provider injected into its own deps (pinned to the bundled vitest version)
    // plus the `webdriverio` framework peer, and the edgedriver/geckodriver
    // postinstalls allowed.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'test', devDependencies: { vite: '^7.0.0' } }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'vitest.config.ts'),
      [
        "import { webdriverio } from '@vitest/browser-webdriverio';",
        "import { defineConfig } from 'vite-plus';",
        'export default defineConfig({',
        '  test: { browser: { enabled: true, provider: webdriverio() } },',
        '});',
        '',
      ].join('\n'),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const devDeps = readJson(path.join(tmpDir, 'package.json')).devDependencies as Record<
      string,
      string
    >;
    // Opt-in provider is pinned to a CONCRETE bundled vitest version in the
    // user's own deps — it is deliberately NOT in VITE_PLUS_OVERRIDE_PACKAGES, so
    // no catalog entry is written for it and it must self-resolve.
    expect(devDeps).toHaveProperty('@vitest/browser-webdriverio', VITEST_VERSION);
    expect(devDeps.webdriverio).toBe('*');

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      allowBuilds: Record<string, boolean>;
    };
    expect(yaml.allowBuilds.edgedriver).toBe(true);
    expect(yaml.allowBuilds.geckodriver).toBe(true);
  });

  it('injects the playwright provider + peer from a source-only vitest config', () => {
    // Opt-in provider: vite-plus no longer bundles `@vitest/browser-playwright`
    // at runtime (its `playwright` peer is non-optional). A project that imports
    // it in source with NO declared dep must have the provider injected into its
    // own deps (pinned to the bundled vitest version) plus the `playwright`
    // framework peer. (Playwright has no edgedriver/geckodriver postinstall, so
    // — unlike webdriverio — it does not touch allowBuilds.)
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'test', devDependencies: { vite: '^7.0.0' } }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'vitest.config.ts'),
      [
        "import { playwright } from '@vitest/browser-playwright';",
        "import { defineConfig } from 'vite-plus';",
        'export default defineConfig({',
        '  test: { browser: { enabled: true, provider: playwright() } },',
        '});',
        '',
      ].join('\n'),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const devDeps = readJson(path.join(tmpDir, 'package.json')).devDependencies as Record<
      string,
      string
    >;
    // Opt-in provider pinned to a CONCRETE bundled vitest version in the user's
    // own deps — deliberately NOT in VITE_PLUS_OVERRIDE_PACKAGES, so no catalog
    // entry is written for it and it must self-resolve.
    expect(devDeps).toHaveProperty('@vitest/browser-playwright', VITEST_VERSION);
    expect(devDeps.playwright).toBe('*');
  });

  it('injects the playwright provider on a re-run from the migrated provider-subpath import', () => {
    // Re-running migration on an ALREADY-migrated project: the import rewriter
    // maps `@vitest/browser-playwright/provider` to
    // `vite-plus/test/browser/providers/playwright`, so an already-migrated
    // source can contain that subpath form. The playwright source scan must
    // recognize it, or the re-run would skip injecting the (no-longer-bundled)
    // provider and the import would break under pnpm strict / Yarn PnP.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'test', devDependencies: { vite: '^7.0.0' } }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'vitest.config.ts'),
      [
        "import { playwright } from 'vite-plus/test/browser/providers/playwright';",
        "import { defineConfig } from 'vite-plus';",
        'export default defineConfig({',
        '  test: { browser: { enabled: true, provider: playwright() } },',
        '});',
        '',
      ].join('\n'),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const devDeps = readJson(path.join(tmpDir, 'package.json')).devDependencies as Record<
      string,
      string
    >;
    expect(devDeps).toHaveProperty('@vitest/browser-playwright', VITEST_VERSION);
    expect(devDeps.playwright).toBe('*');
  });

  it('injects the webdriverio provider on a re-run from the migrated provider-subpath import', () => {
    // Re-running migration on an ALREADY-migrated project: the import rewriter
    // maps `@vitest/browser-webdriverio/provider` to
    // `vite-plus/test/browser/providers/webdriverio`, so an already-migrated
    // source can contain that subpath form (not just `vite-plus/test/browser-
    // webdriverio`). The webdriverio source scan must recognize it, or the re-run
    // would skip injecting the (no-longer-bundled) provider and the import would
    // break under pnpm strict / Yarn PnP.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'test', devDependencies: { vite: '^7.0.0' } }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'vitest.config.ts'),
      [
        "import { webdriverio } from 'vite-plus/test/browser/providers/webdriverio';",
        "import { defineConfig } from 'vite-plus';",
        'export default defineConfig({',
        '  test: { browser: { enabled: true, provider: webdriverio() } },',
        '});',
        '',
      ].join('\n'),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const devDeps = readJson(path.join(tmpDir, 'package.json')).devDependencies as Record<
      string,
      string
    >;
    expect(devDeps).toHaveProperty('@vitest/browser-webdriverio', VITEST_VERSION);
    expect(devDeps.webdriverio).toBe('*');
    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      allowBuilds: Record<string, boolean>;
    };
    expect(yaml.allowBuilds.edgedriver).toBe(true);
    expect(yaml.allowBuilds.geckodriver).toBe(true);
  });

  it('injects the webdriverio provider from a source-only import of the plugin shim', () => {
    // `vite-plus/test/plugins/browser-webdriverio` is a generated shim that
    // re-exports `@vitest/browser-webdriverio` wholesale, so importing it uses
    // the (now opt-in, no-longer-bundled) provider. A source-only import of it
    // must still trigger provider+peer injection and driver-build allowance.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'test', devDependencies: { vite: '^7.0.0' } }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'vitest.config.ts'),
      [
        "import { webdriverio } from 'vite-plus/test/plugins/browser-webdriverio';",
        "import { defineConfig } from 'vite-plus';",
        'export default defineConfig({',
        '  test: { browser: { enabled: true, provider: webdriverio() } },',
        '});',
        '',
      ].join('\n'),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const devDeps = readJson(path.join(tmpDir, 'package.json')).devDependencies as Record<
      string,
      string
    >;
    expect(devDeps).toHaveProperty('@vitest/browser-webdriverio', VITEST_VERSION);
    expect(devDeps.webdriverio).toBe('*');
    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      allowBuilds: Record<string, boolean>;
    };
    expect(yaml.allowBuilds.edgedriver).toBe(true);
    expect(yaml.allowBuilds.geckodriver).toBe(true);
  });

  it('keeps a peer-only catalog webdriverio provider resolvable (no dangling catalog reference)', () => {
    // A package declares the provider ONLY as a `peerDependencies` `catalog:`
    // entry. The migration installs the provider into the user's own deps so the
    // rewritten import resolves, but it must NOT delete the catalog entry the
    // surviving peer still references — deleting it would dangle the `catalog:`
    // spec and break the next install. (Catalog deletion uses REMOVE_PACKAGES,
    // not the override-drop set, precisely so webdriverio entries are preserved.)
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0' },
        peerDependencies: { '@vitest/browser-webdriverio': 'catalog:' },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      [
        'packages:',
        '  - packages/*',
        'catalog:',
        "  '@vitest/browser-webdriverio': 4.0.0",
        '',
      ].join('\n'),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json'));
    const devDeps = pkg.devDependencies as Record<string, string>;
    // Provider installed in the user's own deps at the bundled vitest version.
    expect(devDeps).toHaveProperty('@vitest/browser-webdriverio', VITEST_VERSION);
    expect(devDeps.webdriverio).toBe('*');
    // Peer-only declaration is left intact and its `catalog:` reference still
    // resolves because the catalog entry is preserved (NOT deleted).
    expect((pkg.peerDependencies as Record<string, string>)['@vitest/browser-webdriverio']).toBe(
      'catalog:',
    );
    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      catalog: Record<string, string>;
      allowBuilds: Record<string, boolean>;
    };
    expect(yaml.catalog['@vitest/browser-webdriverio']).toBe('4.0.0');
    expect(yaml.allowBuilds.edgedriver).toBe(true);
    expect(yaml.allowBuilds.geckodriver).toBe(true);
  });

  it('drops a stale npm @vitest/browser-webdriverio override that would conflict with the injected provider', () => {
    // npm hard-fails with EOVERRIDE when an override pins the provider to a
    // version different from the migrated direct dep. Because webdriverio is now
    // KEPT/injected as a direct dep (not stripped), the migration must prune the
    // stale `overrides` entry before injecting `@vitest/browser-webdriverio@4.1.9`.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0', '@vitest/browser-webdriverio': '^4.0.0' },
        overrides: { '@vitest/browser-webdriverio': '4.0.0', 'some-other-pkg': '1.0.0' },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.npm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json'));
    const overrides = pkg.overrides as Record<string, string>;
    // Stale provider override dropped (it would EOVERRIDE-conflict with the dep).
    expect(overrides).not.toHaveProperty('@vitest/browser-webdriverio');
    // Unrelated overrides preserved.
    expect(overrides['some-other-pkg']).toBe('1.0.0');
    // Provider normalized to the bundled vitest version, peer ensured.
    const devDeps = pkg.devDependencies as Record<string, string>;
    expect(devDeps['@vitest/browser-webdriverio']).toBe(VITEST_VERSION);
    expect(devDeps.webdriverio).toBe('*');
  });

  it('drops a stale npm @vitest/browser-playwright override that would conflict with the kept provider', () => {
    // Same hazard as webdriverio: playwright is now opt-in and KEPT as a direct
    // dep (not stripped), so a stale `overrides` pin to a different version would
    // EOVERRIDE-conflict with the migrated `@vitest/browser-playwright@4.1.9`. The
    // migration must prune it before normalizing the provider dep.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0', '@vitest/browser-playwright': '^4.0.0' },
        overrides: { '@vitest/browser-playwright': '4.0.0', 'some-other-pkg': '1.0.0' },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.npm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json'));
    const overrides = pkg.overrides as Record<string, string>;
    expect(overrides).not.toHaveProperty('@vitest/browser-playwright');
    expect(overrides['some-other-pkg']).toBe('1.0.0');
    const devDeps = pkg.devDependencies as Record<string, string>;
    expect(devDeps['@vitest/browser-playwright']).toBe(VITEST_VERSION);
    expect(devDeps.playwright).toBe('*');
  });

  it('drops a stale @vitest/browser-webdriverio override pinned with a COMPARATOR range', () => {
    // A `name@range` override key may use a semver comparator (`@>=4`, `@>4`,
    // `@<5`). The `>` MUST NOT be mistaken for a pnpm `parent>child` selector
    // (pnpm's own delimiter rule excludes a `>` preceded by `@`), or the key's
    // target is parsed incorrectly and the stale pin survives, forcing the
    // provider off the migrated 4.1.9 dep. A comparator-range key for an
    // unrelated package must still be preserved.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0', '@vitest/browser-webdriverio': '^4.0.0' },
        overrides: {
          '@vitest/browser-webdriverio@>=4': '4.0.0',
          'some-other-pkg@>=1': '1.0.0',
        },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.npm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json'));
    const overrides = pkg.overrides as Record<string, string>;
    expect(overrides).not.toHaveProperty('@vitest/browser-webdriverio@>=4');
    // Unrelated comparator-range override preserved.
    expect(overrides['some-other-pkg@>=1']).toBe('1.0.0');
    const devDeps = pkg.devDependencies as Record<string, string>;
    expect(devDeps['@vitest/browser-webdriverio']).toBe(VITEST_VERSION);
    expect(devDeps.webdriverio).toBe('*');
  });

  it('drops a stale yarn @vitest/browser-webdriverio resolution that would force the wrong provider version', () => {
    // Same hazard as npm, via yarn `resolutions`: a leftover pin would force the
    // stale provider over the migrated, bundled-vitest-aligned 4.1.9 dep.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0', '@vitest/browser-webdriverio': '^4.0.0' },
        resolutions: { '@vitest/browser-webdriverio': '4.0.0', 'some-other-pkg': '1.0.0' },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.yarn), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json'));
    const resolutions = pkg.resolutions as Record<string, string>;
    expect(resolutions).not.toHaveProperty('@vitest/browser-webdriverio');
    expect(resolutions['some-other-pkg']).toBe('1.0.0');
    const devDeps = pkg.devDependencies as Record<string, string>;
    expect(devDeps['@vitest/browser-webdriverio']).toBe(VITEST_VERSION);
    expect(devDeps.webdriverio).toBe('*');
  });

  it('drops only global/glob/vite-plus-parent yarn SELECTOR-shaped @vitest/browser-webdriverio resolutions', () => {
    // Yarn resolutions commonly use selector shapes (glob `**/pkg`, nested
    // `parent/pkg`). A pin is pruned only when it would reach vite-plus's OWN
    // direct provider dep — i.e. a versioned global pin, a NAME glob that matches
    // vite-plus (`**`, `vite-*`), or a parent that is literally `vite-plus`. A
    // selector scoped under a SPECIFIC non-vite-plus parent — including a
    // wildcard RANGE on that parent (`parent@*`, `parent@workspace:*`) or a name
    // glob that does NOT match vite-plus (`react-*`) — only constrains that
    // parent's subtree and is preserved (over-reaching would silently change
    // that parent's resolved transitive provider).
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0', '@vitest/browser-webdriverio': '^4.0.0' },
        resolutions: {
          '**/@vitest/browser-webdriverio': '4.0.0',
          'vite-*/@vitest/browser-webdriverio': '4.0.0',
          'vite-plus/@vitest/browser-webdriverio': '4.0.0',
          '**/vite-plus/@vitest/browser-webdriverio': '4.0.0',
          'some-parent/@vitest/browser-webdriverio': '4.0.0',
          'react-*/@vitest/browser-webdriverio': '4.0.0',
          'parent@*/@vitest/browser-webdriverio': '4.0.0',
          'parent@workspace:*/@vitest/browser-webdriverio': '4.0.0',
          'some-parent/**/@vitest/browser-webdriverio': '4.0.0',
          'some-parent/vite-*/@vitest/browser-webdriverio': '4.0.0',
          '@vitest/browser-webdriverio@4': '4.0.0',
          '**/some-other-pkg': '1.0.0',
        },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.yarn), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json'));
    const resolutions = pkg.resolutions as Record<string, string>;
    // Glob parent matches all parents (incl. vite-plus) — dropped.
    expect(resolutions).not.toHaveProperty('**/@vitest/browser-webdriverio');
    // Name glob that matches vite-plus — dropped.
    expect(resolutions).not.toHaveProperty('vite-*/@vitest/browser-webdriverio');
    // Parent is literally vite-plus — dropped.
    expect(resolutions).not.toHaveProperty('vite-plus/@vitest/browser-webdriverio');
    // `**`-padded vite-plus reaches the root vite-plus edge — dropped.
    expect(resolutions).not.toHaveProperty('**/vite-plus/@vitest/browser-webdriverio');
    // Versioned global pin — dropped.
    expect(resolutions).not.toHaveProperty('@vitest/browser-webdriverio@4');
    // Scoped under a SPECIFIC non-vite-plus parent — PRESERVED (does not affect
    // vite-plus's own provider dep).
    expect(resolutions['some-parent/@vitest/browser-webdriverio']).toBe('4.0.0');
    // A name glob that does NOT match vite-plus — PRESERVED.
    expect(resolutions['react-*/@vitest/browser-webdriverio']).toBe('4.0.0');
    // A wildcard RANGE on a specific parent is not a glob parent — PRESERVED.
    expect(resolutions['parent@*/@vitest/browser-webdriverio']).toBe('4.0.0');
    expect(resolutions['parent@workspace:*/@vitest/browser-webdriverio']).toBe('4.0.0');
    // A nested glob gated by a SPECIFIC non-vite-plus ancestor only constrains
    // that ancestor's subtree, NOT the root vite-plus edge — PRESERVED.
    expect(resolutions['some-parent/**/@vitest/browser-webdriverio']).toBe('4.0.0');
    expect(resolutions['some-parent/vite-*/@vitest/browser-webdriverio']).toBe('4.0.0');
    // Unrelated selector resolutions survive.
    expect(resolutions['**/some-other-pkg']).toBe('1.0.0');
    const devDeps = pkg.devDependencies as Record<string, string>;
    expect(devDeps['@vitest/browser-webdriverio']).toBe(VITEST_VERSION);
    expect(devDeps.webdriverio).toBe('*');
  });

  it('preserves yarn from/target resolutions that do NOT target the provider (yarn-grammar faithful)', () => {
    // A yarn `from/target` resolution key forces the TRAILING descriptor, not
    // the parent. Verified against @yarnpkg/parsers parseResolution:
    //   `@vitest/browser-webdriverio@4/some-transitive-dep`
    //       -> from=@vitest/browser-webdriverio@4, descriptor=some-transitive-dep
    //   `@vitest/browser-webdriverio@npm:@other/fork@1.2.3`
    //       -> from=@vitest/browser-webdriverio@npm:@other, descriptor=fork@1.2.3
    // Neither targets the provider, so neither may be pruned — dropping them
    // would silently delete an unrelated user resolution. (Yarn rejects keys
    // whose range embeds a `/`, e.g. `pkg@patch:…/…` or git/URL ranges, so those
    // can never appear as valid keys.) Only keys whose TARGET is the provider
    // are dropped — see the SELECTOR-shaped test above.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0', '@vitest/browser-webdriverio': '^4.0.0' },
        resolutions: {
          '@vitest/browser-webdriverio@4/some-transitive-dep': '1.0.0',
          '@vitest/browser-webdriverio@npm:@other/fork@1.2.3': '2.0.0',
          '@vitest/browser-webdriverio': '4.0.0',
        },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.yarn), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json'));
    const resolutions = pkg.resolutions as Record<string, string>;
    // Parent-through-provider key targets some-transitive-dep — preserved.
    expect(resolutions['@vitest/browser-webdriverio@4/some-transitive-dep']).toBe('1.0.0');
    // npm-alias key targets `fork` (the aliased descriptor), not the provider — preserved.
    expect(resolutions['@vitest/browser-webdriverio@npm:@other/fork@1.2.3']).toBe('2.0.0');
    // The bare key DOES target the provider — pruned so it can't force the
    // stale provider over the migrated 4.1.9 dep.
    expect(resolutions).not.toHaveProperty('@vitest/browser-webdriverio');
    const devDeps = pkg.devDependencies as Record<string, string>;
    expect(devDeps['@vitest/browser-webdriverio']).toBe(VITEST_VERSION);
    expect(devDeps.webdriverio).toBe('*');
  });

  it('does not add vitest for a package without browser mode', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'test', devDependencies: { vite: '^7.0.0' } }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'vite.config.ts'),
      "import { defineConfig } from 'vite';\nexport default defineConfig({});\n",
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const devDeps = readJson(path.join(tmpDir, 'package.json')).devDependencies as Record<
      string,
      string
    >;
    expect(devDeps).not.toHaveProperty('vitest');
  });

  it('detects browser mode from a declared provider dep with no source imports', () => {
    // Config-only browser mode: `vite.config.ts` enables the browser runner by
    // provider name (resolved by vitest at runtime) and the user lists
    // `@vitest/browser-playwright` in devDependencies — but no source or config
    // file imports `@vitest/browser*`. The source-scan signal is therefore
    // false; the dep declaration is what tells us the package drives browser
    // mode. After migration, `vitest` must still be pinned as a direct dep so
    // the browser optimizer can resolve it under pnpm strict / Yarn PnP.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { '@vitest/browser-playwright': '^4.1.7' },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'vite.config.ts'),
      [
        "import { defineConfig } from 'vite';",
        "export default defineConfig({ test: { browser: { provider: 'playwright' } } });",
        '',
      ].join('\n'),
    );

    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const devDeps = readJson(path.join(tmpDir, 'package.json')).devDependencies as Record<
      string,
      string
    >;
    expect(devDeps.vitest).toBe('catalog:');
    expect(devDeps['vite-plus']).toBe('catalog:');
    // Playwright is opt-in: vite-plus keeps the provider in the user's deps,
    // normalized to the bundled vitest version.
    expect(devDeps['@vitest/browser-playwright']).toBe(VITEST_VERSION);
    // Provider's runtime peer dep is preserved.
    expect(devDeps.playwright).toBe('*');
  });

  it('preserves named pnpm overrides when moving root overrides to pnpm-workspace.yaml', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'pnpm-monorepo',
        workspaces: ['packages/*'],
        devDependencies: { vite: 'catalog:vite7' },
        pnpm: {
          overrides: {
            vite: 'catalog:vite7',
            react: '^18.0.0',
          },
        },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      ['packages:', '  - packages/*', 'catalogs:', '  vite7:', '    vite: ^7.0.0', ''].join('\n'),
    );

    rewriteMonorepo(makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true);

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      overrides: Record<string, string>;
      catalogs: Record<string, Record<string, string>>;
    };
    expect(yaml.overrides.vite).toBe('catalog:vite7');
    // Common case (no @vitest/* dep, no vitest source): `vitest` is not managed,
    // so no `vitest` override is injected.
    expect(yaml.overrides.vitest).toBeUndefined();
    expect(yaml.overrides.react).toBe('^18.0.0');
    expect(yaml.catalogs.vite7.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      pnpm?: unknown;
    };
    expect(pkg.pnpm).toBeUndefined();
  });

  it('preserves default pnpm catalog overrides over stale workspace named overrides', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'pnpm-monorepo',
        workspaces: ['packages/*'],
        devDependencies: { vite: 'catalog:' },
        pnpm: {
          overrides: {
            vite: 'catalog:',
          },
        },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      [
        'packages:',
        '  - packages/*',
        'overrides:',
        '  vite: catalog:vite7',
        'catalogs:',
        '  vite7:',
        '    vite: ^7.0.0',
        '',
      ].join('\n'),
    );

    rewriteMonorepo(makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true);

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      overrides: Record<string, string>;
    };
    expect(yaml.overrides.vite).toBe('catalog:');
    // Common case (no @vitest/* dep, no vitest source): `vitest` is not managed,
    // so no `vitest` override is added.
    expect(yaml.overrides.vitest).toBeUndefined();
  });

  it('does not resolve peer dependency catalog specs to migrated aliases', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        peerDependencies: {
          vite: 'catalog:vite7',
          vitest: 'catalog:',
        },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      [
        'catalog:',
        '  vitest: npm:@voidzero-dev/vite-plus-test@latest',
        'catalogs:',
        '  vite7:',
        '    vite: npm:@voidzero-dev/vite-plus-core@latest',
        '',
      ].join('\n'),
    );

    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      peerDependencies: Record<string, string>;
    };
    expect(pkg.peerDependencies.vite).toBe('*');
    // `vitest` is no longer a managed override key (common case: no @vitest/*
    // dep, no vitest source), so its peer entry is left as the user wrote it.
    expect(pkg.peerDependencies.vitest).toBe('catalog:');
  });

  it('adds vitest only to the monorepo package that uses browser mode', () => {
    // Root has no browser config; only `apps/dashboard` does. The browser-mode
    // scan must stop at the nested package.json boundary so the root package
    // does not inherit the sub-package's signal.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'root', devDependencies: {} }),
    );
    fs.writeFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), 'packages:\n  - apps/*\n');
    const appDir = path.join(tmpDir, 'apps', 'dashboard');
    fs.mkdirSync(appDir, { recursive: true });
    fs.writeFileSync(
      path.join(appDir, 'package.json'),
      JSON.stringify({ name: '@vibe/dashboard', devDependencies: { playwright: '^1.58.0' } }),
    );
    fs.writeFileSync(
      path.join(appDir, 'vite.config.ts'),
      [
        "import { playwright } from 'vite-plus/test/browser-playwright';",
        "import { defineConfig } from 'vite-plus';",
        'export default defineConfig({ test: { browser: { provider: playwright() } } });',
        '',
      ].join('\n'),
    );

    const workspaceInfo = makeWorkspaceInfo(tmpDir, PackageManager.pnpm);
    workspaceInfo.isMonorepo = true;
    workspaceInfo.packages = [{ name: '@vibe/dashboard', path: 'apps/dashboard' }];
    rewriteMonorepo(workspaceInfo, true);

    const rootDeps = (readJson(path.join(tmpDir, 'package.json')).devDependencies ?? {}) as Record<
      string,
      string
    >;
    expect(rootDeps).not.toHaveProperty('vitest');

    const appDeps = readJson(path.join(appDir, 'package.json')).devDependencies as Record<
      string,
      string
    >;
    expect(appDeps.vitest).toBe('catalog:');
  });

  it('opts vite-plus workspaces out of yarn nmHoistingLimits so the bundled vitest dedupes', () => {
    // Yarn `node-modules` + `nmHoistingLimits: workspaces` would give every
    // workspace that gets `vite-plus` (which depends on the bundled `vitest`) its
    // own physical `vitest`/`@vitest/runner` copy, splitting `vp test`'s runner
    // across two instances -> `TypeError: ...reading 'config'`. The migration must
    // set `installConfig.hoistingLimits: none` on each vite-plus-receiving
    // workspace so its vitest hoists to the single shared root copy, WITHOUT
    // touching the root `.yarnrc.yml` isolation (which unrelated workspaces such as
    // a React Native `example` may rely on).
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'root',
        private: true,
        workspaces: ['packages/*'],
        devDependencies: {},
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, '.yarnrc.yml'),
      'nodeLinker: node-modules\nnmHoistingLimits: workspaces\n',
    );
    // A workspace WITH vitest -> gets vite-plus -> must be opted out.
    const libDir = path.join(tmpDir, 'packages', 'lib');
    fs.mkdirSync(libDir, { recursive: true });
    fs.writeFileSync(
      path.join(libDir, 'package.json'),
      JSON.stringify({ name: '@scope/lib', devDependencies: { vitest: '^4.0.0' } }),
    );
    // A workspace WITHOUT any vite-plus signal -> must stay untouched.
    const isoDir = path.join(tmpDir, 'packages', 'iso');
    fs.mkdirSync(isoDir, { recursive: true });
    fs.writeFileSync(
      path.join(isoDir, 'package.json'),
      JSON.stringify({ name: '@scope/iso', dependencies: { 'left-pad': '^1.0.0' } }),
    );

    const workspaceInfo = makeWorkspaceInfo(tmpDir, PackageManager.yarn);
    workspaceInfo.isMonorepo = true;
    workspaceInfo.packages = [
      { name: '@scope/lib', path: 'packages/lib' },
      { name: '@scope/iso', path: 'packages/iso' },
    ];
    const report = createMigrationReport();
    rewriteMonorepo(workspaceInfo, true, false, report);

    const libPkg = readJson(path.join(libDir, 'package.json')) as {
      devDependencies?: Record<string, string>;
      installConfig?: { hoistingLimits?: string };
    };
    expect(libPkg.devDependencies).toHaveProperty('vite-plus');
    expect(libPkg.installConfig?.hoistingLimits).toBe('none');

    // No vite-plus added -> no installConfig opt-out.
    const isoPkg = readJson(path.join(isoDir, 'package.json')) as {
      devDependencies?: Record<string, string>;
      installConfig?: unknown;
    };
    expect(isoPkg.devDependencies ?? {}).not.toHaveProperty('vite-plus');
    expect(isoPkg.installConfig).toBeUndefined();

    // The root .yarnrc.yml isolation is preserved (not silently removed) and the
    // root package.json is not given a redundant per-workspace opt-out.
    expect(readYamlObject(path.join(tmpDir, '.yarnrc.yml')).nmHoistingLimits).toBe('workspaces');
    expect(
      (readJson(path.join(tmpDir, 'package.json')) as { installConfig?: unknown }).installConfig,
    ).toBeUndefined();

    // Auto-fix is silent: a deduped workspace needs no manual-step warning.
    expect(report.warnings.some((w) => w.includes('isolates dependency hoisting'))).toBe(false);
  });

  it('leaves yarn workspaces alone when nmHoistingLimits does not isolate them', () => {
    // Default hoisting (no nmHoistingLimits) already dedupes vitest to root, so the
    // migration must NOT add a spurious installConfig opt-out.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'root',
        private: true,
        workspaces: ['packages/*'],
        devDependencies: {},
      }),
    );
    fs.writeFileSync(path.join(tmpDir, '.yarnrc.yml'), 'nodeLinker: node-modules\n');
    const libDir = path.join(tmpDir, 'packages', 'lib');
    fs.mkdirSync(libDir, { recursive: true });
    fs.writeFileSync(
      path.join(libDir, 'package.json'),
      JSON.stringify({ name: '@scope/lib', devDependencies: { vitest: '^4.0.0' } }),
    );

    const workspaceInfo = makeWorkspaceInfo(tmpDir, PackageManager.yarn);
    workspaceInfo.isMonorepo = true;
    workspaceInfo.packages = [{ name: '@scope/lib', path: 'packages/lib' }];
    const report = createMigrationReport();
    rewriteMonorepo(workspaceInfo, true, false, report);

    const libPkg = readJson(path.join(libDir, 'package.json')) as {
      devDependencies?: Record<string, string>;
      installConfig?: unknown;
    };
    expect(libPkg.devDependencies).toHaveProperty('vite-plus');
    expect(libPkg.installConfig).toBeUndefined();
    expect(report.warnings.some((w) => w.includes('isolates dependency hoisting'))).toBe(false);
  });

  it('does not auto-fix yarn `dependencies` hoisting (the opt-out cannot dedupe it)', () => {
    // The stricter `nmHoistingLimits: dependencies` keeps a dep BELOW each
    // dependent package even when the workspace opts out to `none` (verified with
    // Yarn 4.17: two workspaces sharing a dep still produced two physical copies),
    // so writing the opt-out would be false remediation — a package.json that looks
    // fixed but keeps the crash layout. The migration must leave installConfig
    // untouched for this mode.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'root',
        private: true,
        workspaces: ['packages/*'],
        devDependencies: {},
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, '.yarnrc.yml'),
      'nodeLinker: node-modules\nnmHoistingLimits: dependencies\n',
    );
    const libDir = path.join(tmpDir, 'packages', 'lib');
    fs.mkdirSync(libDir, { recursive: true });
    fs.writeFileSync(
      path.join(libDir, 'package.json'),
      JSON.stringify({ name: '@scope/lib', devDependencies: { vitest: '^4.0.0' } }),
    );

    const workspaceInfo = makeWorkspaceInfo(tmpDir, PackageManager.yarn);
    workspaceInfo.isMonorepo = true;
    workspaceInfo.packages = [{ name: '@scope/lib', path: 'packages/lib' }];
    const report = createMigrationReport();
    rewriteMonorepo(workspaceInfo, true, false, report);

    const libPkg = readJson(path.join(libDir, 'package.json')) as {
      devDependencies?: Record<string, string>;
      installConfig?: unknown;
    };
    expect(libPkg.devDependencies).toHaveProperty('vite-plus');
    expect(libPkg.installConfig).toBeUndefined();
    // Not silently broken: warn that vp test can crash for this isolated workspace.
    expect(report.warnings.some((w) => w.includes('isolates dependency hoisting'))).toBe(true);
  });

  it('preserves an explicit workspace installConfig.hoistingLimits instead of clobbering it', () => {
    // A workspace that deliberately set its OWN hoisting limit (e.g. to isolate its
    // whole tree for Metro) and also uses Vite+ must keep that explicit invariant —
    // `installConfig.hoistingLimits` governs the ENTIRE workspace tree, not just the
    // vitest family. The opt-out only relaxes the INHERITED root limit (unset field).
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'root',
        private: true,
        workspaces: ['packages/*'],
        devDependencies: {},
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, '.yarnrc.yml'),
      'nodeLinker: node-modules\nnmHoistingLimits: workspaces\n',
    );
    const libDir = path.join(tmpDir, 'packages', 'lib');
    fs.mkdirSync(libDir, { recursive: true });
    fs.writeFileSync(
      path.join(libDir, 'package.json'),
      JSON.stringify({
        name: '@scope/lib',
        devDependencies: { vitest: '^4.0.0' },
        installConfig: { hoistingLimits: 'workspaces' },
      }),
    );

    const workspaceInfo = makeWorkspaceInfo(tmpDir, PackageManager.yarn);
    workspaceInfo.isMonorepo = true;
    workspaceInfo.packages = [{ name: '@scope/lib', path: 'packages/lib' }];
    const report = createMigrationReport();
    rewriteMonorepo(workspaceInfo, true, false, report);

    const libPkg = readJson(path.join(libDir, 'package.json')) as {
      devDependencies?: Record<string, string>;
      installConfig?: { hoistingLimits?: string };
    };
    expect(libPkg.devDependencies).toHaveProperty('vite-plus');
    // Explicit value preserved, NOT overwritten to 'none'.
    expect(libPkg.installConfig?.hoistingLimits).toBe('workspaces');
    // The preserved isolation still splits vp test, so it must be flagged.
    expect(report.warnings.some((w) => w.includes('isolates dependency hoisting'))).toBe(true);
  });

  it('warns on workspace-level hoisting isolation even when the root nmHoistingLimits is unset', () => {
    // A workspace can isolate its OWN tree via `installConfig.hoistingLimits`
    // regardless of the root limit. With the root unset, that workspace still keeps
    // its own vitest copy, so the migration must preserve the explicit value AND
    // warn — the per-workspace check cannot be gated on the root limit.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'root',
        private: true,
        workspaces: ['packages/*'],
        devDependencies: {},
      }),
    );
    fs.writeFileSync(path.join(tmpDir, '.yarnrc.yml'), 'nodeLinker: node-modules\n');
    const libDir = path.join(tmpDir, 'packages', 'lib');
    fs.mkdirSync(libDir, { recursive: true });
    fs.writeFileSync(
      path.join(libDir, 'package.json'),
      JSON.stringify({
        name: '@scope/lib',
        devDependencies: { vitest: '^4.0.0' },
        installConfig: { hoistingLimits: 'workspaces' },
      }),
    );

    const workspaceInfo = makeWorkspaceInfo(tmpDir, PackageManager.yarn);
    workspaceInfo.isMonorepo = true;
    workspaceInfo.packages = [{ name: '@scope/lib', path: 'packages/lib' }];
    const report = createMigrationReport();
    rewriteMonorepo(workspaceInfo, true, false, report);

    const libPkg = readJson(path.join(libDir, 'package.json')) as {
      devDependencies?: Record<string, string>;
      installConfig?: { hoistingLimits?: string };
    };
    expect(libPkg.devDependencies).toHaveProperty('vite-plus');
    expect(libPkg.installConfig?.hoistingLimits).toBe('workspaces');
    expect(report.warnings.some((w) => w.includes('isolates dependency hoisting'))).toBe(true);
  });

  it('warns on workspace-level hoisting isolation even when the root nmHoistingLimits is none', () => {
    // Root explicitly `none` (default deduping) but the workspace pins its own
    // `dependencies` isolation -> it still keeps its own vitest copy -> the
    // migration must preserve the explicit value AND warn.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'root',
        private: true,
        workspaces: ['packages/*'],
        devDependencies: {},
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, '.yarnrc.yml'),
      'nodeLinker: node-modules\nnmHoistingLimits: none\n',
    );
    const libDir = path.join(tmpDir, 'packages', 'lib');
    fs.mkdirSync(libDir, { recursive: true });
    fs.writeFileSync(
      path.join(libDir, 'package.json'),
      JSON.stringify({
        name: '@scope/lib',
        devDependencies: { vitest: '^4.0.0' },
        installConfig: { hoistingLimits: 'dependencies' },
      }),
    );

    const workspaceInfo = makeWorkspaceInfo(tmpDir, PackageManager.yarn);
    workspaceInfo.isMonorepo = true;
    workspaceInfo.packages = [{ name: '@scope/lib', path: 'packages/lib' }];
    const report = createMigrationReport();
    rewriteMonorepo(workspaceInfo, true, false, report);

    const libPkg = readJson(path.join(libDir, 'package.json')) as {
      devDependencies?: Record<string, string>;
      installConfig?: { hoistingLimits?: string };
    };
    expect(libPkg.devDependencies).toHaveProperty('vite-plus');
    expect(libPkg.installConfig?.hoistingLimits).toBe('dependencies');
    expect(report.warnings.some((w) => w.includes('isolates dependency hoisting'))).toBe(true);
  });

  it('auto-fixes a direct rewriteMonorepoProject call by deriving the root .yarnrc.yml limit', () => {
    // Callers other than rewriteMonorepo (e.g. `vp create` integrating a package
    // into an existing monorepo) call rewriteMonorepoProject directly with no
    // workspace context and no root-limit argument. The root
    // `nmHoistingLimits: workspaces` must still be discovered by walking up from
    // the package directory, so the workspace is deduped — not silently left split.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'root', private: true, workspaces: ['packages/*'] }),
    );
    fs.writeFileSync(
      path.join(tmpDir, '.yarnrc.yml'),
      'nodeLinker: node-modules\nnmHoistingLimits: workspaces\n',
    );
    const libDir = path.join(tmpDir, 'packages', 'lib');
    fs.mkdirSync(libDir, { recursive: true });
    fs.writeFileSync(
      path.join(libDir, 'package.json'),
      JSON.stringify({ name: '@scope/lib', devDependencies: { vitest: '^4.0.0' } }),
    );

    // Direct call: no workspaceContext, no root-limit arg.
    rewriteMonorepoProject(libDir, PackageManager.yarn, true, true);

    const libPkg = readJson(path.join(libDir, 'package.json')) as {
      devDependencies?: Record<string, string>;
      installConfig?: { hoistingLimits?: string };
    };
    expect(libPkg.devDependencies).toHaveProperty('vite-plus');
    expect(libPkg.installConfig?.hoistingLimits).toBe('none');
  });

  it('finds the real monorepo root limit even when the workspace has its own .yarnrc.yml', () => {
    // `vp create` (and install) can write a package-local `.yarnrc.yml` under a
    // workspace before it is rewritten. The hoisting lookup must NOT treat that
    // child rc as the project root — it must find the actual workspace root (the
    // package.json with `workspaces`) and read ITS nmHoistingLimits, so the
    // workspace is still deduped rather than silently left split.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'root', private: true, workspaces: ['packages/*'] }),
    );
    fs.writeFileSync(
      path.join(tmpDir, '.yarnrc.yml'),
      'nodeLinker: node-modules\nnmHoistingLimits: workspaces\n',
    );
    const libDir = path.join(tmpDir, 'packages', 'lib');
    fs.mkdirSync(libDir, { recursive: true });
    fs.writeFileSync(
      path.join(libDir, 'package.json'),
      JSON.stringify({ name: '@scope/lib', devDependencies: { vitest: '^4.0.0' } }),
    );
    // A package-local `.yarnrc.yml` (no nmHoistingLimits) must NOT shadow the root.
    fs.writeFileSync(path.join(libDir, '.yarnrc.yml'), 'nodeLinker: node-modules\n');

    // Direct call (no workspaceContext): the lookup must walk past the child rc to
    // the workspace root.
    rewriteMonorepoProject(libDir, PackageManager.yarn, true, true);

    const libPkg = readJson(path.join(libDir, 'package.json')) as {
      devDependencies?: Record<string, string>;
      installConfig?: { hoistingLimits?: string };
    };
    expect(libPkg.devDependencies).toHaveProperty('vite-plus');
    expect(libPkg.installConfig?.hoistingLimits).toBe('none');
  });

  it('honours nmHoistingLimits set in an ANCESTOR .yarnrc.yml above the monorepo root', () => {
    // Yarn merges `.yarnrc.yml` across the project root AND its ancestor directories
    // (verified with Yarn 4.17: a key set only in an ancestor rc is in effect). So a
    // monorepo root whose own `.yarnrc.yml` omits `nmHoistingLimits` can still inherit
    // `workspaces` isolation from a parent rc. The lookup must resolve the effective
    // limit across the chain, not read only the root dir's rc, or the workspace is
    // silently left split.
    const ancestorDir = tmpDir;
    const rootDir = path.join(ancestorDir, 'monorepo');
    fs.mkdirSync(rootDir, { recursive: true });
    // Ancestor rc (ABOVE the monorepo root) sets the isolating limit.
    fs.writeFileSync(
      path.join(ancestorDir, '.yarnrc.yml'),
      'nodeLinker: node-modules\nnmHoistingLimits: workspaces\n',
    );
    // The monorepo root is the package.json with `workspaces`; its own rc omits the key.
    fs.writeFileSync(
      path.join(rootDir, 'package.json'),
      JSON.stringify({ name: 'root', private: true, workspaces: ['packages/*'] }),
    );
    fs.writeFileSync(path.join(rootDir, '.yarnrc.yml'), 'nodeLinker: node-modules\n');
    const libDir = path.join(rootDir, 'packages', 'lib');
    fs.mkdirSync(libDir, { recursive: true });
    fs.writeFileSync(
      path.join(libDir, 'package.json'),
      JSON.stringify({ name: '@scope/lib', devDependencies: { vitest: '^4.0.0' } }),
    );

    // Direct call (no workspaceContext): the lookup must walk past the root rc to the
    // ancestor rc to find the effective `workspaces` limit.
    rewriteMonorepoProject(libDir, PackageManager.yarn, true, true);

    const libPkg = readJson(path.join(libDir, 'package.json')) as {
      devDependencies?: Record<string, string>;
      installConfig?: { hoistingLimits?: string };
    };
    expect(libPkg.devDependencies).toHaveProperty('vite-plus');
    expect(libPkg.installConfig?.hoistingLimits).toBe('none');
  });

  it('lets a monorepo-root .yarnrc.yml override an ancestor nmHoistingLimits (closer wins)', () => {
    // Yarn's config merge gives the closest `.yarnrc.yml` precedence: a root rc that
    // sets `nmHoistingLimits: none` neutralizes an ancestor's `workspaces`, so the
    // layout already dedupes and the migration must NOT add a spurious opt-out.
    const ancestorDir = tmpDir;
    const rootDir = path.join(ancestorDir, 'monorepo');
    fs.mkdirSync(rootDir, { recursive: true });
    fs.writeFileSync(
      path.join(ancestorDir, '.yarnrc.yml'),
      'nodeLinker: node-modules\nnmHoistingLimits: workspaces\n',
    );
    fs.writeFileSync(
      path.join(rootDir, 'package.json'),
      JSON.stringify({ name: 'root', private: true, workspaces: ['packages/*'] }),
    );
    // Root rc explicitly opts back into full hoisting — overrides the ancestor.
    fs.writeFileSync(
      path.join(rootDir, '.yarnrc.yml'),
      'nodeLinker: node-modules\nnmHoistingLimits: none\n',
    );
    const libDir = path.join(rootDir, 'packages', 'lib');
    fs.mkdirSync(libDir, { recursive: true });
    fs.writeFileSync(
      path.join(libDir, 'package.json'),
      JSON.stringify({ name: '@scope/lib', devDependencies: { vitest: '^4.0.0' } }),
    );

    rewriteMonorepoProject(libDir, PackageManager.yarn, true, true);

    const libPkg = readJson(path.join(libDir, 'package.json')) as {
      devDependencies?: Record<string, string>;
      installConfig?: { hoistingLimits?: string };
    };
    expect(libPkg.devDependencies).toHaveProperty('vite-plus');
    // Closer rc wins (none) -> no isolation -> no opt-out added.
    expect(libPkg.installConfig?.hoistingLimits).toBeUndefined();
  });

  it('skips the hoisting opt-out for a PnP Yarn project (nmHoistingLimits is inert without node-modules)', () => {
    // Yarn DEFAULTS to Plug'n'Play; `nmHoistingLimits` only splits physical copies
    // under the `node-modules` linker. With `nodeLinker` unset the effective linker is
    // `pnp`, resolution is virtual, and there is no dual-`@vitest/runner` — so writing
    // `installConfig.hoistingLimits: none` would be a spurious mutation. Skip it.
    const savedLinker = process.env.YARN_NODE_LINKER;
    delete process.env.YARN_NODE_LINKER;
    try {
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({ name: 'root', private: true, workspaces: ['packages/*'] }),
      );
      // nmHoistingLimits set, but nodeLinker unset -> effective linker is pnp.
      fs.writeFileSync(path.join(tmpDir, '.yarnrc.yml'), 'nmHoistingLimits: workspaces\n');
      const libDir = path.join(tmpDir, 'packages', 'lib');
      fs.mkdirSync(libDir, { recursive: true });
      fs.writeFileSync(
        path.join(libDir, 'package.json'),
        JSON.stringify({ name: '@scope/lib', devDependencies: { vitest: '^4.0.0' } }),
      );

      rewriteMonorepoProject(libDir, PackageManager.yarn, true, true);

      const libPkg = readJson(path.join(libDir, 'package.json')) as {
        devDependencies?: Record<string, string>;
        installConfig?: unknown;
      };
      expect(libPkg.devDependencies).toHaveProperty('vite-plus');
      expect(libPkg.installConfig).toBeUndefined();
    } finally {
      if (savedLinker === undefined) {
        delete process.env.YARN_NODE_LINKER;
      } else {
        process.env.YARN_NODE_LINKER = savedLinker;
      }
    }
  });

  it('honours YARN_NM_HOISTING_LIMITS=workspaces from the environment (highest precedence)', () => {
    // Yarn lets `YARN_<KEY>` env vars override `.yarnrc.yml` (verified with Yarn 4.17).
    // A repo whose rc omits `nmHoistingLimits` but runs under
    // `YARN_NM_HOISTING_LIMITS=workspaces` is still isolated, so the workspace must be
    // auto-fixed.
    const savedLimit = process.env.YARN_NM_HOISTING_LIMITS;
    process.env.YARN_NM_HOISTING_LIMITS = 'workspaces';
    try {
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({ name: 'root', private: true, workspaces: ['packages/*'] }),
      );
      // node-modules linker, but no rc nmHoistingLimits — the env supplies it.
      fs.writeFileSync(path.join(tmpDir, '.yarnrc.yml'), 'nodeLinker: node-modules\n');
      const libDir = path.join(tmpDir, 'packages', 'lib');
      fs.mkdirSync(libDir, { recursive: true });
      fs.writeFileSync(
        path.join(libDir, 'package.json'),
        JSON.stringify({ name: '@scope/lib', devDependencies: { vitest: '^4.0.0' } }),
      );

      rewriteMonorepoProject(libDir, PackageManager.yarn, true, true);

      const libPkg = readJson(path.join(libDir, 'package.json')) as {
        installConfig?: { hoistingLimits?: string };
      };
      expect(libPkg.installConfig?.hoistingLimits).toBe('none');
    } finally {
      if (savedLimit === undefined) {
        delete process.env.YARN_NM_HOISTING_LIMITS;
      } else {
        process.env.YARN_NM_HOISTING_LIMITS = savedLimit;
      }
    }
  });

  it('lets YARN_NM_HOISTING_LIMITS=none override an in-tree workspaces limit (env wins)', () => {
    // Env precedence cuts both ways: an env override to `none` makes an in-tree
    // `workspaces` value non-effective, so the layout already dedupes and the
    // migration must NOT add a spurious opt-out.
    const savedLimit = process.env.YARN_NM_HOISTING_LIMITS;
    process.env.YARN_NM_HOISTING_LIMITS = 'none';
    try {
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({ name: 'root', private: true, workspaces: ['packages/*'] }),
      );
      fs.writeFileSync(
        path.join(tmpDir, '.yarnrc.yml'),
        'nodeLinker: node-modules\nnmHoistingLimits: workspaces\n',
      );
      const libDir = path.join(tmpDir, 'packages', 'lib');
      fs.mkdirSync(libDir, { recursive: true });
      fs.writeFileSync(
        path.join(libDir, 'package.json'),
        JSON.stringify({ name: '@scope/lib', devDependencies: { vitest: '^4.0.0' } }),
      );

      rewriteMonorepoProject(libDir, PackageManager.yarn, true, true);

      const libPkg = readJson(path.join(libDir, 'package.json')) as {
        installConfig?: unknown;
      };
      expect(libPkg.installConfig).toBeUndefined();
    } finally {
      if (savedLimit === undefined) {
        delete process.env.YARN_NM_HOISTING_LIMITS;
      } else {
        process.env.YARN_NM_HOISTING_LIMITS = savedLimit;
      }
    }
  });

  it('honours nodeLinker from the home ~/.yarnrc.yml for a repo OUTSIDE $HOME (devcontainer layout)', () => {
    // Yarn reads the home `.yarnrc.yml` even when the project is not under $HOME
    // (verified with Yarn 4.17). Devcontainers/Codespaces mount the repo under
    // /workspaces while $HOME is /home/<user>, so the node-modules linker can come
    // from the home rc while the project rc carries `nmHoistingLimits: workspaces`.
    // The gate must still fire — resolving `nodeLinker` from the home rc — or the
    // split is silently left behind.
    const homeDir = path.join(tmpDir, 'home');
    const projectRoot = path.join(tmpDir, 'workspaces', 'repo');
    fs.mkdirSync(homeDir, { recursive: true });
    fs.mkdirSync(projectRoot, { recursive: true });
    // nodeLinker lives ONLY in the home rc; the project is a sibling of $HOME.
    fs.writeFileSync(path.join(homeDir, '.yarnrc.yml'), 'nodeLinker: node-modules\n');
    fs.writeFileSync(
      path.join(projectRoot, 'package.json'),
      JSON.stringify({ name: 'root', private: true, workspaces: ['packages/*'] }),
    );
    fs.writeFileSync(path.join(projectRoot, '.yarnrc.yml'), 'nmHoistingLimits: workspaces\n');
    const libDir = path.join(projectRoot, 'packages', 'lib');
    fs.mkdirSync(libDir, { recursive: true });
    fs.writeFileSync(
      path.join(libDir, 'package.json'),
      JSON.stringify({ name: '@scope/lib', devDependencies: { vitest: '^4.0.0' } }),
    );

    // Redirect the home dir to our temp home. `os.homedir()` reads `HOME` on POSIX and
    // `USERPROFILE` on Windows, so set both; the describe-level afterEach restores them.
    process.env.HOME = homeDir;
    process.env.USERPROFILE = homeDir;
    rewriteMonorepoProject(libDir, PackageManager.yarn, true, true);

    const libPkg = readJson(path.join(libDir, 'package.json')) as {
      devDependencies?: Record<string, string>;
      installConfig?: { hoistingLimits?: string };
    };
    expect(libPkg.devDependencies).toHaveProperty('vite-plus');
    // node-modules (home rc) + workspaces (project rc) -> the split is real -> opt-out.
    expect(libPkg.installConfig?.hoistingLimits).toBe('none');
  });

  it('does not write an edgedriver/geckodriver default-deny in pnpm-workspace.yaml when webdriverio is unused (pnpm v10)', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'test', devDependencies: { vite: '^7.0.0' } }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    // edgedriver/geckodriver only reach the tree via the opt-in webdriverio provider (an
    // OPTIONAL peer of both vite-plus and vitest, so pnpm never auto-installs it). A
    // non-webdriverio project never installs them, so there is nothing to manage and
    // vite-plus writes no allowBuilds block at all.
    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      allowBuilds?: Record<string, boolean>;
    };
    expect(yaml.allowBuilds).toBeUndefined();
  });

  it('allows edgedriver/geckodriver builds when webdriverio is in devDependencies (pnpm v10)', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0', webdriverio: '^9.0.0' },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      allowBuilds: Record<string, boolean>;
    };
    expect(yaml.allowBuilds.edgedriver).toBe(true);
    expect(yaml.allowBuilds.geckodriver).toBe(true);
  });

  it('allows edgedriver/geckodriver builds when only @vitest/browser-webdriverio is declared (pnpm v10)', () => {
    // The migrator keeps `@vitest/browser-webdriverio` (opt-in provider) and
    // ensures `webdriverio: '*'` as its runtime peer, so the post-migration
    // deps will need the driver postinstalls even though the pre-migration
    // package.json never lists `webdriverio` directly.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: {
          vite: '^7.0.0',
          vitest: '^4.0.0',
          '@vitest/browser-webdriverio': '^4.0.0',
        },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      allowBuilds: Record<string, boolean>;
    };
    expect(yaml.allowBuilds.edgedriver).toBe(true);
    expect(yaml.allowBuilds.geckodriver).toBe(true);
  });

  it('allows edgedriver/geckodriver builds when @vitest/browser-webdriverio is declared only in peerDependencies (pnpm v10)', () => {
    // Same rationale as the devDependencies case: the migrator keeps the
    // opt-in `@vitest/browser-webdriverio` provider and ensures `webdriverio: '*'`,
    // so the post-migration deps still need the driver postinstalls. The
    // allow-signal scan must therefore also look at peerDependencies.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: {
          vite: '^7.0.0',
          vitest: '^4.0.0',
        },
        peerDependencies: {
          '@vitest/browser-webdriverio': '^4.0.0',
        },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      allowBuilds: Record<string, boolean>;
    };
    expect(yaml.allowBuilds.edgedriver).toBe(true);
    expect(yaml.allowBuilds.geckodriver).toBe(true);
  });

  it('preserves explicit allowBuilds entries and adds nothing else on second run (idempotent)', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'test', devDependencies: { vite: '^7.0.0' } }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      ['allowBuilds:', '  edgedriver: true', ''].join('\n'),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const firstPass = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      allowBuilds: Record<string, boolean>;
    };
    // explicit user choice survives; with no webdriverio the missing geckodriver entry is
    // left absent (vite-plus no longer writes a default deny).
    expect(firstPass.allowBuilds.edgedriver).toBe(true);
    expect('geckodriver' in firstPass.allowBuilds).toBe(false);

    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);
    const secondPass = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      allowBuilds: Record<string, boolean>;
    };
    expect(secondPass.allowBuilds).toEqual(firstPass.allowBuilds);
  });

  it('flips a stale edgedriver/geckodriver denial to true when webdriverio is added on a re-migration (pnpm v10)', () => {
    // A prior WebdriverIO-less migration wrote `allowBuilds.<driver>: false`. The user
    // then adds webdriverio and re-runs migration: the drivers are now needed, so the
    // stale `false` MUST be overwritten with `true` — otherwise pnpm keeps the driver
    // postinstall blocked and `vp test` browser mode breaks.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0', webdriverio: '^9.0.0' },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      ['allowBuilds:', '  edgedriver: false', '  geckodriver: false', ''].join('\n'),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      allowBuilds: Record<string, boolean>;
    };
    expect(yaml.allowBuilds.edgedriver).toBe(true);
    expect(yaml.allowBuilds.geckodriver).toBe(true);
  });

  it('leaves a user-authored driver denial untouched on a re-migration (pnpm v10, no webdriverio)', () => {
    // The user directly depends on `edgedriver` and has denied its build
    // (`allowBuilds.edgedriver: false`, e.g. their own Selenium setup, no webdriverio).
    // vite-plus does not manage these postinstalls when webdriverio is unused, so it must
    // leave the user's denial — and the unrelated geckodriver entry — exactly as-is
    // rather than deleting a trust decision it never made.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0', edgedriver: '^6.0.0' },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      ['allowBuilds:', '  edgedriver: false', '  geckodriver: false', ''].join('\n'),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      allowBuilds: Record<string, boolean>;
    };
    expect(yaml.allowBuilds.edgedriver).toBe(false);
    expect(yaml.allowBuilds.geckodriver).toBe(false);
  });

  it('preserves a user-approved (true) direct-driver dep on a re-migration (pnpm-workspace.yaml v10)', () => {
    // The user depends on `edgedriver` directly AND has already approved its build
    // (`allowBuilds.edgedriver: true`, e.g. via `pnpm approve-builds`). Re-running
    // migration (no webdriverio) must PRESERVE that approval untouched — vite-plus does
    // not manage these postinstalls when webdriverio is unused.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0', edgedriver: '^6.0.0' },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      ['allowBuilds:', '  edgedriver: true', '  geckodriver: false', ''].join('\n'),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      allowBuilds: Record<string, boolean>;
    };
    // user's approval survives; not-owned geckodriver denial stays
    expect(yaml.allowBuilds.edgedriver).toBe(true);
    expect(yaml.allowBuilds.geckodriver).toBe(false);
  });

  it('leaves an anchored (&/*) driver denial untouched without crashing (pnpm-workspace.yaml v10, no webdriverio)', () => {
    // Valid YAML can express the denial through an anchor/alias. With no webdriverio,
    // vite-plus does not touch allowBuilds, so the anchor (`&deny false`) and its alias
    // (`*deny`) are both preserved intact — there is no delete that could orphan the
    // alias and abort serialization with "Unresolved alias".
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0', geckodriver: '^4.0.0' },
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'pnpm-workspace.yaml'),
      ['allowBuilds:', '  edgedriver: &deny false', '  geckodriver: *deny', ''].join('\n'),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      allowBuilds: Record<string, boolean>;
    };
    // both the anchor-owner and the alias resolve to the preserved `false`.
    expect(yaml.allowBuilds.edgedriver).toBe(false);
    expect(yaml.allowBuilds.geckodriver).toBe(false);
  });

  it('preserves a user-approved (true) direct-driver dep on a re-migration (package.json pnpm sink, v10)', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0', geckodriver: '^4.0.0' },
        pnpm: { allowBuilds: { edgedriver: false, geckodriver: true }, overrides: {} },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const pnpm = (readJson(path.join(tmpDir, 'package.json')).pnpm ?? {}) as {
      allowBuilds?: Record<string, boolean>;
    };
    // No webdriverio -> vite-plus leaves the whole allowBuilds map untouched: the
    // user-approved geckodriver `true` and the user's edgedriver `false` both survive.
    expect(pnpm.allowBuilds?.geckodriver).toBe(true);
    expect(pnpm.allowBuilds?.edgedriver).toBe(false);
  });

  it('flips a stale package.json pnpm.allowBuilds denial to true when webdriverio is added (pnpm v10)', () => {
    // Same re-migration flip, but for the package.json `pnpm` sink (used when the
    // pnpm config lives in package.json and there is no pnpm-workspace.yaml).
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0', webdriverio: '^9.0.0' },
        pnpm: { allowBuilds: { edgedriver: false, geckodriver: false }, overrides: {} },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const pnpm = (readJson(path.join(tmpDir, 'package.json')).pnpm ?? {}) as {
      allowBuilds?: Record<string, boolean>;
    };
    expect(pnpm.allowBuilds?.edgedriver).toBe(true);
    expect(pnpm.allowBuilds?.geckodriver).toBe(true);
  });

  it('does not write a pnpm.allowBuilds default-deny in package.json when webdriverio is unused (pnpm v10)', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0' },
        pnpm: { overrides: {} },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    // No webdriverio -> nothing to manage -> no allowBuilds key added to the pnpm sink
    // (the webdriverio-present case still writes `true` here — see the flip test below).
    const pnpm = (readJson(path.join(tmpDir, 'package.json')).pnpm ?? {}) as {
      allowBuilds?: Record<string, boolean>;
    };
    expect(pnpm.allowBuilds).toBeUndefined();
  });

  it('appends edgedriver/geckodriver to onlyBuiltDependencies on pnpm v9 when webdriverio is used', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0', webdriverio: '^9.0.0' },
      }),
    );
    const workspaceInfo = makeWorkspaceInfo(tmpDir, PackageManager.pnpm);
    workspaceInfo.downloadPackageManager.version = '9.15.0';
    rewriteStandaloneProject(tmpDir, workspaceInfo, true, true);

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      onlyBuiltDependencies?: string[];
      allowBuilds?: Record<string, boolean>;
    };
    expect(yaml.onlyBuiltDependencies).toEqual(
      expect.arrayContaining(['edgedriver', 'geckodriver']),
    );
    // v10-shape key must not appear on v9 setups
    expect(yaml.allowBuilds).toBeUndefined();
  });

  it('leaves onlyBuiltDependencies untouched on pnpm v9 when webdriverio is unused', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'test', devDependencies: { vite: '^7.0.0' } }),
    );
    const workspaceInfo = makeWorkspaceInfo(tmpDir, PackageManager.pnpm);
    workspaceInfo.downloadPackageManager.version = '9.15.0';
    rewriteStandaloneProject(tmpDir, workspaceInfo, true, true);

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      onlyBuiltDependencies?: string[];
      allowBuilds?: Record<string, boolean>;
    };
    expect(yaml.onlyBuiltDependencies).toBeUndefined();
    expect(yaml.allowBuilds).toBeUndefined();
  });

  it('detects webdriverio in a monorepo sub-package and allows builds at the root', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'root', devDependencies: {} }),
    );
    fs.writeFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), 'packages:\n  - apps/*\n');
    const appDir = path.join(tmpDir, 'apps', 'e2e');
    fs.mkdirSync(appDir, { recursive: true });
    fs.writeFileSync(
      path.join(appDir, 'package.json'),
      JSON.stringify({
        name: '@vibe/e2e',
        devDependencies: { webdriverio: '^9.0.0' },
      }),
    );

    const workspaceInfo = makeWorkspaceInfo(tmpDir, PackageManager.pnpm);
    workspaceInfo.isMonorepo = true;
    workspaceInfo.packages = [{ name: '@vibe/e2e', path: 'apps/e2e' }];
    rewriteMonorepo(workspaceInfo, true);

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      allowBuilds: Record<string, boolean>;
    };
    expect(yaml.allowBuilds.edgedriver).toBe(true);
    expect(yaml.allowBuilds.geckodriver).toBe(true);
  });

  it('does not deny a driver the user depends on directly when webdriverio is unused (pnpm v10)', () => {
    // Non-webdriverio Selenium setup: the user manages their own edgedriver postinstall
    // approval. The migration writes no deny — neither for the user-owned edgedriver nor
    // for the not-owned geckodriver (never installed without webdriverio) — so no
    // allowBuilds block is written and pnpm keeps the user's own approval/prompt.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0', edgedriver: '^6.0.0' },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      allowBuilds?: Record<string, boolean>;
    };
    expect(yaml.allowBuilds).toBeUndefined();
  });

  it('auto-allows a user direct driver dep when webdriverio is present (pnpm v10)', () => {
    // The user depends on edgedriver directly AND uses webdriverio (which also
    // needs the driver built). The webdriverio signal makes builds allowed, so
    // write `allowBuilds.edgedriver = true` rather than leaving the key absent —
    // a driver webdriverio needs built must not be left to a pnpm prompt.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: {
          vite: '^7.0.0',
          webdriverio: '^9.0.0',
          edgedriver: '^6.0.0',
        },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      allowBuilds: Record<string, boolean>;
    };
    expect(yaml.allowBuilds.edgedriver).toBe(true);
    expect(yaml.allowBuilds.geckodriver).toBe(true);
  });

  it('writes no driver allowBuilds entries when no driver is a direct dep and webdriverio is unused (regression guard)', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'test', devDependencies: { vite: '^7.0.0' } }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      allowBuilds?: Record<string, boolean>;
    };
    expect(yaml.allowBuilds).toBeUndefined();
  });

  it('does not deny a driver the user depends on directly when webdriverio is unused (package.json pnpm config)', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'test',
        devDependencies: { vite: '^7.0.0', edgedriver: '^6.0.0' },
        pnpm: { overrides: {} },
      }),
    );
    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      pnpm?: { allowBuilds?: Record<string, boolean> };
    };
    expect(pkg.pnpm?.allowBuilds).toBeUndefined();
  });

  it('writes no workspace-yaml allowBuilds for a monorepo with a direct driver dep but no webdriverio (pnpm v10)', () => {
    // A sub-package has its own edgedriver postinstall approval but nothing in the
    // workspace uses webdriverio. The migration writes no deny for either driver, so the
    // sub-package's own edgedriver approval is left to pnpm.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'root', devDependencies: {} }),
    );
    fs.writeFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), 'packages:\n  - apps/*\n');
    const appDir = path.join(tmpDir, 'apps', 'e2e');
    fs.mkdirSync(appDir, { recursive: true });
    fs.writeFileSync(
      path.join(appDir, 'package.json'),
      JSON.stringify({
        name: '@vibe/e2e',
        devDependencies: { edgedriver: '^6.0.0' },
      }),
    );

    const workspaceInfo = makeWorkspaceInfo(tmpDir, PackageManager.pnpm);
    workspaceInfo.isMonorepo = true;
    workspaceInfo.packages = [{ name: '@vibe/e2e', path: 'apps/e2e' }];
    rewriteMonorepo(workspaceInfo, true);

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      allowBuilds?: Record<string, boolean>;
    };
    expect(yaml.allowBuilds).toBeUndefined();
  });

  it('writes no allowBuilds for a monorepo where the ROOT has a direct driver dep but no webdriverio (pnpm v10)', () => {
    // The workspace root has its own geckodriver postinstall approval but nothing uses
    // webdriverio. The migration writes no deny for either driver; the root's own
    // geckodriver approval is left to pnpm. In non-force mode the root pnpm config is
    // normalized into pnpm-workspace.yaml, so that is the operative allowBuilds sink.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'root',
        devDependencies: { geckodriver: '^5.0.0' },
      }),
    );
    fs.writeFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), 'packages:\n  - apps/*\n');
    const appDir = path.join(tmpDir, 'apps', 'e2e');
    fs.mkdirSync(appDir, { recursive: true });
    fs.writeFileSync(
      path.join(appDir, 'package.json'),
      JSON.stringify({ name: '@vibe/e2e', devDependencies: {} }),
    );

    const workspaceInfo = makeWorkspaceInfo(tmpDir, PackageManager.pnpm);
    workspaceInfo.isMonorepo = true;
    workspaceInfo.packages = [{ name: '@vibe/e2e', path: 'apps/e2e' }];
    rewriteMonorepo(workspaceInfo, true);

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      allowBuilds?: Record<string, boolean>;
    };
    expect(yaml.allowBuilds).toBeUndefined();
  });

  it('auto-allows a direct driver dep when another workspace package uses webdriverio (monorepo, pnpm v10)', () => {
    // Mixed workspace: package A depends on edgedriver directly while package B
    // uses webdriverio (which also needs edgedriver/geckodriver built). The
    // allowBuilds sink is workspace-global, so the webdriverio signal must write
    // `true` for BOTH drivers — including the one a package depends on directly.
    // Leaving edgedriver absent would force a pnpm prompt for a build webdriverio needs.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'root', devDependencies: {} }),
    );
    fs.writeFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), 'packages:\n  - apps/*\n');
    const driverApp = path.join(tmpDir, 'apps', 'selenium');
    fs.mkdirSync(driverApp, { recursive: true });
    fs.writeFileSync(
      path.join(driverApp, 'package.json'),
      JSON.stringify({ name: '@vibe/selenium', devDependencies: { edgedriver: '^6.0.0' } }),
    );
    const wdioApp = path.join(tmpDir, 'apps', 'wdio');
    fs.mkdirSync(wdioApp, { recursive: true });
    fs.writeFileSync(
      path.join(wdioApp, 'package.json'),
      JSON.stringify({ name: '@vibe/wdio', devDependencies: { webdriverio: '^9.0.0' } }),
    );

    const workspaceInfo = makeWorkspaceInfo(tmpDir, PackageManager.pnpm);
    workspaceInfo.isMonorepo = true;
    workspaceInfo.packages = [
      { name: '@vibe/selenium', path: 'apps/selenium' },
      { name: '@vibe/wdio', path: 'apps/wdio' },
    ];
    rewriteMonorepo(workspaceInfo, true);

    const yaml = readYamlObject(path.join(tmpDir, 'pnpm-workspace.yaml')) as {
      allowBuilds: Record<string, boolean>;
    };
    expect(yaml.allowBuilds.edgedriver).toBe(true);
    expect(yaml.allowBuilds.geckodriver).toBe(true);
  });
});

describe('rewriteMonorepo yarn catalog', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-yarn-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('rewrites named catalogs in .yarnrc.yml and keeps nodeLinker', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'yarn-monorepo',
        workspaces: ['packages/*'],
        devDependencies: { vite: 'catalog:vite7' },
        peerDependencies: { vite: 'catalog:vite7', vitest: 'catalog:test' },
        packageManager: 'yarn@4.10.0',
      }),
    );
    fs.writeFileSync(
      path.join(tmpDir, '.yarnrc.yml'),
      [
        'catalogs:',
        '  vite7:',
        '    react: ^18.0.0',
        '    vite: ^7.0.0',
        '  test:',
        '    vitest: ^4.0.0',
        '    oxlint: ^1.0.0',
        '',
      ].join('\n'),
    );

    rewriteMonorepo(makeWorkspaceInfo(tmpDir, PackageManager.yarn), true);

    const yarnrc = readYamlObject(path.join(tmpDir, '.yarnrc.yml')) as {
      nodeLinker: string;
      catalogs: Record<string, Record<string, string>>;
    };
    expect(yarnrc.nodeLinker).toBe('node-modules');
    expect(yarnrc.catalogs.vite7.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    expect(yarnrc.catalogs.vite7.react).toBe('^18.0.0');
    // Common case (no @vitest/* dep, no vitest source): `vitest` is not managed,
    // so the pre-existing named-catalog `vitest` entry is REMOVED.
    expect(yarnrc.catalogs.test.vitest).toBeUndefined();
    expect(yarnrc.catalogs.test.oxlint).toBeUndefined();

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      devDependencies: Record<string, string>;
      peerDependencies: Record<string, string>;
    };
    expect(pkg.devDependencies.vite).toBe('catalog:vite7');
    expect(pkg.peerDependencies.vite).toBe('^7.0.0');
    // `vitest` is no longer managed, so the peer entry is left as the user
    // wrote it (untouched).
    expect(pkg.peerDependencies.vitest).toBe('catalog:test');
  });
});

describe('rewriteMonorepo bun catalog', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-bun-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('writes catalog to top-level when workspaces is an array', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'bun-monorepo',
        workspaces: ['packages/*'],
        devDependencies: { vite: '^7.0.0' },
        packageManager: 'bun@1.3.11',
      }),
    );
    rewriteMonorepo(makeWorkspaceInfo(tmpDir, PackageManager.bun), true);

    const pkg = readJson(path.join(tmpDir, 'package.json'));
    // catalog should be at top level
    const catalog = pkg.catalog as Record<string, string>;
    expect(catalog.vite).toBeDefined();
    expect(catalog['vite-plus']).toBe('latest');
    // overrides should reference catalog:
    const overrides = pkg.overrides as Record<string, string>;
    expect(overrides.vite).toBe('catalog:');
  });

  it('writes catalog to workspaces.catalog when workspaces is an object with existing catalog', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'bun-monorepo',
        workspaces: {
          packages: ['packages/*'],
          catalog: { react: '^19.0.0' },
        },
        devDependencies: { vite: '^7.0.0' },
        packageManager: 'bun@1.3.11',
      }),
    );
    rewriteMonorepo(makeWorkspaceInfo(tmpDir, PackageManager.bun), true);

    const pkg = readJson(path.join(tmpDir, 'package.json'));
    // No top-level catalog
    expect(pkg.catalog).toBeUndefined();
    // workspaces.catalog should have merged entries
    const workspaces = pkg.workspaces as { packages: string[]; catalog: Record<string, string> };
    expect(workspaces.catalog.react).toBe('^19.0.0');
    expect(workspaces.catalog.vite).toBeDefined();
    expect(workspaces.catalog['vite-plus']).toBe('latest');
    // workspaces.packages should be preserved
    expect(workspaces.packages).toEqual(['packages/*']);
  });

  it('cleans stale top-level bun catalog when workspaces.catalog is preferred', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'bun-monorepo',
        workspaces: {
          packages: ['packages/*'],
          catalog: { vite: '^7.0.0' },
        },
        catalog: {
          vite: '^6.0.0',
          vitest: '^3.0.0',
          tsdown: '^0.1.0',
          react: '^19.0.0',
        },
        devDependencies: { vite: '^7.0.0' },
        packageManager: 'bun@1.3.11',
      }),
    );

    rewriteMonorepo(makeWorkspaceInfo(tmpDir, PackageManager.bun), true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      catalog: Record<string, string>;
      workspaces: { catalog: Record<string, string> };
    };
    expect(pkg.workspaces.catalog.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    expect(pkg.workspaces.catalog['vite-plus']).toBe('latest');
    expect(pkg.catalog.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    // Common case (no @vitest/* dep, no vitest source): `vitest` is not managed,
    // so the pre-existing catalog `vitest` entry is REMOVED.
    expect(pkg.catalog.vitest).toBeUndefined();
    expect(pkg.catalog.tsdown).toBeUndefined();
    expect(pkg.catalog.react).toBe('^19.0.0');
    expect(pkg.catalog['vite-plus']).toBeUndefined();
  });

  it('writes catalog to top-level when workspaces is an object without catalog', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'bun-monorepo',
        workspaces: {
          packages: ['packages/*'],
        },
        devDependencies: { vite: '^7.0.0' },
        packageManager: 'bun@1.3.11',
      }),
    );
    rewriteMonorepo(makeWorkspaceInfo(tmpDir, PackageManager.bun), true);

    const pkg = readJson(path.join(tmpDir, 'package.json'));
    // catalog should be at top level since workspaces.catalog didn't exist
    const catalog = pkg.catalog as Record<string, string>;
    expect(catalog.vite).toBeDefined();
    expect(catalog['vite-plus']).toBe('latest');
    // workspaces object should be preserved
    const workspaces = pkg.workspaces as { packages: string[] };
    expect(workspaces.packages).toEqual(['packages/*']);
  });

  it('rewrites top-level named catalogs and preserves named overrides', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'bun-monorepo',
        workspaces: ['packages/*'],
        catalogs: {
          build: { vite: '^7.0.0', react: '^19.0.0', tsdown: '^0.1.0' },
          test: { vitest: '^4.0.0' },
        },
        overrides: { vite: 'catalog:build' },
        devDependencies: { vite: 'catalog:build' },
        peerDependencies: { vite: 'catalog:build', vitest: 'catalog:test' },
        packageManager: 'bun@1.3.11',
      }),
    );

    rewriteMonorepo(makeWorkspaceInfo(tmpDir, PackageManager.bun), true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      catalog: Record<string, string>;
      catalogs: Record<string, Record<string, string>>;
      overrides: Record<string, string>;
      devDependencies: Record<string, string>;
      peerDependencies: Record<string, string>;
    };
    expect(pkg.catalog.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    expect(pkg.catalogs.build.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    expect(pkg.catalogs.build.react).toBe('^19.0.0');
    expect(pkg.catalogs.build.tsdown).toBeUndefined();
    // Common case (no @vitest/* dep, no vitest source): `vitest` is not managed,
    // so the pre-existing named-catalog `vitest` entry is REMOVED and no
    // `overrides.vitest` is injected.
    expect(pkg.catalogs.test.vitest).toBeUndefined();
    expect(pkg.overrides.vite).toBe('catalog:build');
    expect(pkg.overrides.vitest).toBeUndefined();
    expect(pkg.devDependencies.vite).toBe('catalog:build');
    expect(pkg.peerDependencies.vite).toBe('^7.0.0');
    // `vitest` is no longer managed, so the peer entry is left as the user
    // wrote it (untouched).
    expect(pkg.peerDependencies.vitest).toBe('catalog:test');
  });

  it('rewrites workspaces named catalogs and writes default catalog beside them', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'bun-monorepo',
        workspaces: {
          packages: ['packages/*'],
          catalogs: {
            build: { vite: '^7.0.0', oxlint: '^1.0.0' },
            test: { vitest: '^4.0.0', vite: '^7.0.0' },
          },
        },
        devDependencies: { vite: '^7.0.0' },
        packageManager: 'bun@1.3.11',
      }),
    );

    rewriteMonorepo(makeWorkspaceInfo(tmpDir, PackageManager.bun), true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      catalog?: Record<string, string>;
      workspaces: {
        catalog: Record<string, string>;
        catalogs: Record<string, Record<string, string>>;
      };
      overrides: Record<string, string>;
    };
    expect(pkg.catalog).toBeUndefined();
    expect(pkg.workspaces.catalog.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    expect(pkg.workspaces.catalog['vite-plus']).toBe('latest');
    expect(pkg.workspaces.catalogs.build.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    expect(pkg.workspaces.catalogs.build.oxlint).toBeUndefined();
    // Common case (no @vitest/* dep, no vitest source): `vitest` is not managed,
    // so the pre-existing named-catalog `vitest` entry is REMOVED.
    expect(pkg.workspaces.catalogs.test.vitest).toBeUndefined();
    expect(pkg.workspaces.catalogs.test.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    expect(pkg.overrides.vite).toBe('catalog:');
  });

  it('keeps an existing top-level default catalog when workspaces named catalogs exist', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        name: 'bun-monorepo',
        workspaces: {
          packages: ['packages/*'],
          catalogs: {
            build: { vite: '^7.0.0' },
          },
        },
        catalog: { react: '^19.0.0' },
        devDependencies: { vite: '^7.0.0' },
        packageManager: 'bun@1.3.11',
      }),
    );

    rewriteMonorepo(makeWorkspaceInfo(tmpDir, PackageManager.bun), true);

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      catalog: Record<string, string>;
      workspaces: {
        catalog?: Record<string, string>;
        catalogs: Record<string, Record<string, string>>;
      };
    };
    expect(pkg.catalog.react).toBe('^19.0.0');
    expect(pkg.catalog.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
    expect(pkg.workspaces.catalog).toBeUndefined();
    expect(pkg.workspaces.catalogs.build.vite).toBe('npm:@voidzero-dev/vite-plus-core@latest');
  });
});

describe('framework shim', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  describe('detectFramework', () => {
    it('returns [vue] when vue is in devDependencies', () => {
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({ devDependencies: { vue: '^3.0.0' } }),
      );
      expect(detectFramework(tmpDir)).toEqual(['vue']);
    });

    it('returns [astro] when astro is in devDependencies', () => {
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({ devDependencies: { astro: '^4.0.0' } }),
      );
      expect(detectFramework(tmpDir)).toEqual(['astro']);
    });

    it('returns [vue, astro] when both are present', () => {
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({ devDependencies: { vue: '^3.0.0', astro: '^4.0.0' } }),
      );
      expect(detectFramework(tmpDir)).toEqual(['vue', 'astro']);
    });

    it('returns [] when no framework dependency is present', () => {
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({ devDependencies: { vite: '^7.0.0' } }),
      );
      expect(detectFramework(tmpDir)).toEqual([]);
    });

    it('returns [] when package.json does not exist', () => {
      expect(detectFramework(tmpDir)).toEqual([]);
    });
  });

  describe('hasFrameworkShim', () => {
    it('returns true when src/env.d.ts contains vue shim', () => {
      const srcDir = path.join(tmpDir, 'src');
      fs.mkdirSync(srcDir);
      fs.writeFileSync(
        path.join(srcDir, 'env.d.ts'),
        "declare module '*.vue' { export default {} }\n",
      );
      expect(hasFrameworkShim(tmpDir, 'vue')).toBe(true);
    });

    it('returns false when src/env.d.ts does not contain vue shim', () => {
      const srcDir = path.join(tmpDir, 'src');
      fs.mkdirSync(srcDir);
      fs.writeFileSync(
        path.join(srcDir, 'env.d.ts'),
        '/// <reference types="vite-plus/client" />\n',
      );
      expect(hasFrameworkShim(tmpDir, 'vue')).toBe(false);
    });

    it('returns false when env.d.ts does not exist', () => {
      expect(hasFrameworkShim(tmpDir, 'vue')).toBe(false);
    });

    it('returns true when root env.d.ts contains astro/client reference', () => {
      fs.writeFileSync(path.join(tmpDir, 'env.d.ts'), '/// <reference types="astro/client" />\n');
      expect(hasFrameworkShim(tmpDir, 'astro')).toBe(true);
    });
  });

  describe('addFrameworkShim', () => {
    it('creates src/env.d.ts with vue shim when src/ exists and no env.d.ts', () => {
      fs.mkdirSync(path.join(tmpDir, 'src'));
      addFrameworkShim(tmpDir, 'vue');
      const content = fs.readFileSync(path.join(tmpDir, 'src', 'env.d.ts'), 'utf-8');
      expect(content).toContain("declare module '*.vue'");
      expect(content).toContain('DefineComponent');
    });

    it('creates root env.d.ts with vue shim when no src/ dir', () => {
      addFrameworkShim(tmpDir, 'vue');
      const content = fs.readFileSync(path.join(tmpDir, 'env.d.ts'), 'utf-8');
      expect(content).toContain("declare module '*.vue'");
    });

    it('appends vue shim to existing src/env.d.ts', () => {
      const srcDir = path.join(tmpDir, 'src');
      fs.mkdirSync(srcDir);
      const existing = '/// <reference types="vite-plus/client" />\n';
      fs.writeFileSync(path.join(srcDir, 'env.d.ts'), existing);
      addFrameworkShim(tmpDir, 'vue');
      const content = fs.readFileSync(path.join(srcDir, 'env.d.ts'), 'utf-8');
      expect(content).toContain('/// <reference types="vite-plus/client" />');
      expect(content).toContain("declare module '*.vue'");
    });

    it('sets frameworkShimAdded on report', () => {
      fs.mkdirSync(path.join(tmpDir, 'src'));
      const report = createMigrationReport();
      addFrameworkShim(tmpDir, 'vue', report);
      expect(report.frameworkShimAdded).toBe(true);
    });
  });

  describe('create flow integration', () => {
    it('does not add duplicate shim when template already wrote env.d.ts', () => {
      // Simulate create-vue having already written a shim into src/env.d.ts
      const srcDir = path.join(tmpDir, 'src');
      fs.mkdirSync(srcDir);
      const existingShim =
        "declare module '*.vue' {\n  import type { DefineComponent } from 'vue';\n  const component: DefineComponent;\n  export default component;\n}\n";
      fs.writeFileSync(path.join(srcDir, 'env.d.ts'), existingShim);
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({ devDependencies: { vue: '^3.0.0' } }),
      );

      expect(detectFramework(tmpDir)).toEqual(['vue']);
      // Gate check: shim already present, so addFrameworkShim should NOT be called
      expect(hasFrameworkShim(tmpDir, 'vue')).toBe(true);
      // Verify content is unchanged if caller respects the gate
      const contentBefore = fs.readFileSync(path.join(srcDir, 'env.d.ts'), 'utf-8');
      for (const framework of detectFramework(tmpDir)) {
        if (!hasFrameworkShim(tmpDir, framework)) {
          addFrameworkShim(tmpDir, framework);
        }
      }
      const contentAfter = fs.readFileSync(path.join(srcDir, 'env.d.ts'), 'utf-8');
      expect(contentAfter).toBe(contentBefore);
    });

    it('adds shim for vue project created without env.d.ts', () => {
      fs.mkdirSync(path.join(tmpDir, 'src'));
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({ devDependencies: { vue: '^3.0.0' } }),
      );
      for (const framework of detectFramework(tmpDir)) {
        if (!hasFrameworkShim(tmpDir, framework)) {
          addFrameworkShim(tmpDir, framework);
        }
      }
      const content = fs.readFileSync(path.join(tmpDir, 'src', 'env.d.ts'), 'utf-8');
      expect(content).toContain("declare module '*.vue'");
    });

    it('adds astro shim for astro project without env.d.ts', () => {
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({ devDependencies: { astro: '^4.0.0' } }),
      );
      for (const framework of detectFramework(tmpDir)) {
        if (!hasFrameworkShim(tmpDir, framework)) {
          addFrameworkShim(tmpDir, framework);
        }
      }
      const content = fs.readFileSync(path.join(tmpDir, 'env.d.ts'), 'utf-8');
      expect(content).toContain('/// <reference types="astro/client" />');
    });

    it('adds both vue and astro shims for Astro+Vue project', () => {
      fs.mkdirSync(path.join(tmpDir, 'src'));
      fs.writeFileSync(
        path.join(tmpDir, 'package.json'),
        JSON.stringify({ devDependencies: { vue: '^3.0.0', astro: '^4.0.0' } }),
      );
      for (const framework of detectFramework(tmpDir)) {
        if (!hasFrameworkShim(tmpDir, framework)) {
          addFrameworkShim(tmpDir, framework);
        }
      }
      const content = fs.readFileSync(path.join(tmpDir, 'src', 'env.d.ts'), 'utf-8');
      expect(content).toContain("declare module '*.vue'");
      expect(content).toContain('/// <reference types="astro/client" />');
    });
  });

  describe('injectCreateDefaultTemplate', () => {
    let tmpDir: string;

    beforeEach(() => {
      tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-migrator-create-default-'));
    });

    afterEach(() => {
      fs.rmSync(tmpDir, { recursive: true, force: true });
    });

    function writeViteConfig(body: string): void {
      fs.writeFileSync(
        path.join(tmpDir, 'vite.config.ts'),
        `import { defineConfig } from 'vite-plus';\n\nexport default defineConfig(${body});\n`,
      );
    }

    it('injects `create.defaultTemplate` when scope is set and no `create:` exists', () => {
      writeViteConfig('{ run: { cache: true } }');
      injectCreateDefaultTemplate(tmpDir, '@your-org', true);
      const content = fs.readFileSync(path.join(tmpDir, 'vite.config.ts'), 'utf-8');
      expect(content).toContain('create:');
      expect(content).toContain('"defaultTemplate":"@your-org"');
    });

    it('skips injection when scope is empty (no scope to default to)', () => {
      writeViteConfig('{ run: { cache: true } }');
      injectCreateDefaultTemplate(tmpDir, '', true);
      const content = fs.readFileSync(path.join(tmpDir, 'vite.config.ts'), 'utf-8');
      expect(content).not.toContain('create:');
      expect(content).not.toContain('defaultTemplate');
    });

    it('preserves an existing `create:` block instead of overwriting it', () => {
      writeViteConfig("{ create: { defaultTemplate: '@other' }, run: { cache: true } }");
      injectCreateDefaultTemplate(tmpDir, '@your-org', true);
      const content = fs.readFileSync(path.join(tmpDir, 'vite.config.ts'), 'utf-8');
      expect(content).toContain("'@other'");
      expect(content).not.toContain('@your-org');
    });
  });
});

// `vp create` / `vp migrate` inject default `lint`/`fmt` blocks into the
// scaffolded vite.config.ts. A custom template that already declares these
// keys via shorthand properties (`fmt,` / `lint,`, e.g. wiring in tooling
// modules) must be preserved verbatim, not get a duplicate inline key. See #1836.
describe('inject defaults — shorthand config keys', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-migrator-inject-shorthand-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  function writeShorthandViteConfig(): void {
    fs.writeFileSync(
      path.join(tmpDir, 'vite.config.ts'),
      `import { defineConfig } from 'vite-plus';

import { fmt } from './tooling/format';
import { lint } from './tooling/lint';

export default defineConfig(({ mode }) => {
  return {
    server: { port: 3000 },
    fmt,
    lint,
  };
});
`,
    );
  }

  it('does not inject a duplicate `fmt` key when one exists as a shorthand property', () => {
    writeShorthandViteConfig();
    const before = fs.readFileSync(path.join(tmpDir, 'vite.config.ts'), 'utf-8');

    injectFmtDefaults(tmpDir, true);

    const after = fs.readFileSync(path.join(tmpDir, 'vite.config.ts'), 'utf-8');
    expect(after).toBe(before);
    expect(after).not.toContain('fmt: {');
  });

  it('does not inject a duplicate `lint` key when one exists as a shorthand property', () => {
    writeShorthandViteConfig();
    const before = fs.readFileSync(path.join(tmpDir, 'vite.config.ts'), 'utf-8');

    injectLintTypeCheckDefaults(tmpDir, true);

    const after = fs.readFileSync(path.join(tmpDir, 'vite.config.ts'), 'utf-8');
    expect(after).toBe(before);
    expect(after).not.toContain('jsPlugins');
    expect(after).not.toContain('prefer-vite-plus-imports');
  });
});

describe('rewriteStandaloneProject — lazy plugin wrapping', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-lazy-plugins-'));
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'test', devDependencies: { vite: '^7.0.0' } }),
    );
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('wraps standalone inline plugin arrays after import rewriting', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'vite.config.ts'),
      `import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react(), nitro({ rollupConfig: { external: [/^@sentry\\//] } })],
});
`,
    );
    const report = createMigrationReport();

    rewriteStandaloneProject(
      tmpDir,
      makeWorkspaceInfo(tmpDir, PackageManager.pnpm),
      true,
      true,
      report,
    );

    const viteConfig = fs.readFileSync(path.join(tmpDir, 'vite.config.ts'), 'utf8');
    expect(viteConfig).toContain("import { defineConfig, lazyPlugins } from 'vite-plus'");
    expect(viteConfig).toContain(
      'plugins: lazyPlugins(() => [react(), nitro({ rollupConfig: { external: [/^@sentry\\//] } })])',
    );
    expect(viteConfig).not.toContain('plugins: [react(), nitro(');
    expect(report.wrappedPluginConfigCount).toBe(1);
  });

  it('leaves unsupported plugin expressions unchanged', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'vite.config.ts'),
      `import { defineConfig } from 'vite-plus';

const plugins = [react()];

export default defineConfig({
  plugins,
});
`,
    );
    const report = createMigrationReport();

    rewriteStandaloneProject(
      tmpDir,
      makeWorkspaceInfo(tmpDir, PackageManager.pnpm),
      true,
      true,
      report,
    );

    const viteConfig = fs.readFileSync(path.join(tmpDir, 'vite.config.ts'), 'utf8');
    expect(viteConfig).toContain('plugins,');
    expect(viteConfig).not.toContain('lazyPlugins');
    expect(report.wrappedPluginConfigCount).toBe(0);
  });

  it('wraps direct monorepo project rewrites used by create-monorepo flows', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'vite.config.ts'),
      `import { defineConfig } from 'vite-plus';

export default defineConfig({
  plugins: [react()],
});
`,
    );
    const report = createMigrationReport();

    rewriteMonorepoProject(tmpDir, PackageManager.pnpm, true, true, report);

    const viteConfig = fs.readFileSync(path.join(tmpDir, 'vite.config.ts'), 'utf8');
    expect(viteConfig).toContain("import { defineConfig, lazyPlugins } from 'vite-plus'");
    expect(viteConfig).toContain('plugins: lazyPlugins(() => [react()])');
    expect(report.wrappedPluginConfigCount).toBe(1);
  });

  it('wraps package-level inline plugin arrays in monorepos', () => {
    const appDir = path.join(tmpDir, 'apps', 'web');
    fs.mkdirSync(appDir, { recursive: true });
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'root', workspaces: ['apps/*'], devDependencies: { vite: '^7.0.0' } }),
    );
    fs.writeFileSync(
      path.join(appDir, 'package.json'),
      JSON.stringify({ name: 'web', devDependencies: { vite: '^7.0.0' } }),
    );
    fs.writeFileSync(
      path.join(appDir, 'vite.config.ts'),
      `import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [react()],
});
`,
    );
    const workspaceInfo = makeWorkspaceInfo(tmpDir, PackageManager.pnpm);
    workspaceInfo.isMonorepo = true;
    workspaceInfo.workspacePatterns = ['apps/*'];
    workspaceInfo.parentDirs = ['apps'];
    workspaceInfo.packages = [{ name: 'web', path: 'apps/web' }];
    const report = createMigrationReport();

    rewriteMonorepo(workspaceInfo, true, true, report);

    const viteConfig = fs.readFileSync(path.join(appDir, 'vite.config.ts'), 'utf8');
    expect(viteConfig).toContain("import { defineConfig, lazyPlugins } from 'vite-plus'");
    expect(viteConfig).toContain('plugins: lazyPlugins(() => [react()])');
    expect(report.wrappedPluginConfigCount).toBe(1);
  });
});

describe('rewriteStandaloneProject — tsconfig types rewriting', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-tsconfig-'));
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'test', devDependencies: { vite: '^7.0.0' } }),
    );
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('rewrites tsdown/client to vite-plus/pack/client in tsconfig.json', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'tsconfig.json'),
      JSON.stringify({ compilerOptions: { types: ['tsdown/client'] } }, null, 2),
    );

    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const tsconfig = readJson(path.join(tmpDir, 'tsconfig.json'));
    expect((tsconfig.compilerOptions as { types: string[] }).types).toContain(
      'vite-plus/pack/client',
    );
    expect((tsconfig.compilerOptions as { types: string[] }).types).not.toContain('tsdown/client');
  });

  it('rewrites vite/client to vite-plus/client in tsconfig.json', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'tsconfig.json'),
      JSON.stringify({ compilerOptions: { types: ['vite/client'] } }, null, 2),
    );

    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const tsconfig = readJson(path.join(tmpDir, 'tsconfig.json'));
    expect((tsconfig.compilerOptions as { types: string[] }).types).toContain('vite-plus/client');
    expect((tsconfig.compilerOptions as { types: string[] }).types).not.toContain('vite/client');
  });

  it('rewrites types in tsconfig.node.json as well', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'tsconfig.node.json'),
      JSON.stringify({ compilerOptions: { types: ['tsdown/client'] } }, null, 2),
    );

    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const tsconfig = readJson(path.join(tmpDir, 'tsconfig.node.json'));
    expect((tsconfig.compilerOptions as { types: string[] }).types).toContain(
      'vite-plus/pack/client',
    );
  });
});

describe('existing Vite+ core migration finalization', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-existing-vite-plus-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('detects and finalizes legacy scripts, imports, and tsconfig types without dependency rewrites', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify(
        {
          name: 'test',
          scripts: {
            dev: 'vite',
            build: 'tsc -b && vite build',
            preview: 'vite preview',
          },
          devDependencies: {
            'vite-plus': 'latest',
            '@voidzero-dev/vite-plus-core': 'latest',
          },
        },
        null,
        2,
      ),
    );
    fs.writeFileSync(
      path.join(tmpDir, 'vite.config.ts'),
      "import { defineConfig } from 'vite';\nexport default defineConfig({});\n",
    );
    fs.writeFileSync(
      path.join(tmpDir, 'tsconfig.app.json'),
      JSON.stringify({ compilerOptions: { types: ['vite/client'] } }, null, 2),
    );

    const workspaceInfo = makeWorkspaceInfo(tmpDir, PackageManager.npm);
    expect(detectPendingCoreMigration(workspaceInfo)).toEqual({
      scripts: true,
      tsconfigTypes: true,
    });

    expect(finalizeCoreMigrationForExistingVitePlus(workspaceInfo, true)).toEqual({
      scripts: true,
      tsconfigTypes: true,
      imports: true,
    });

    const pkg = readJson(path.join(tmpDir, 'package.json')) as {
      scripts: Record<string, string>;
      devDependencies: Record<string, string>;
      overrides?: Record<string, string>;
    };
    expect(pkg.scripts).toMatchObject({
      dev: 'vp dev',
      build: 'tsc -b && vp build',
      preview: 'vp preview',
    });
    expect(pkg.devDependencies).toEqual({
      'vite-plus': 'latest',
      '@voidzero-dev/vite-plus-core': 'latest',
    });
    expect(pkg.overrides).toBeUndefined();
    expect(fs.readFileSync(path.join(tmpDir, 'vite.config.ts'), 'utf8')).toContain(
      "from 'vite-plus'",
    );
    const tsconfig = readJson(path.join(tmpDir, 'tsconfig.app.json'));
    expect((tsconfig.compilerOptions as { types: string[] }).types).toEqual(['vite-plus/client']);
    expect(detectPendingCoreMigration(workspaceInfo)).toEqual({
      scripts: false,
      tsconfigTypes: false,
    });
  });

  it('detects package-level legacy signals in workspaces', () => {
    const appDir = path.join(tmpDir, 'packages', 'app');
    fs.mkdirSync(appDir, { recursive: true });
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'root', devDependencies: { 'vite-plus': 'latest' } }, null, 2),
    );
    fs.writeFileSync(
      path.join(appDir, 'package.json'),
      JSON.stringify({ name: 'app', scripts: { dev: 'vite' } }, null, 2),
    );
    const workspaceInfo = {
      ...makeWorkspaceInfo(tmpDir, PackageManager.pnpm),
      isMonorepo: true,
      packages: [{ name: 'app', path: 'packages/app' }],
    };

    expect(detectPendingCoreMigration(workspaceInfo).scripts).toBe(true);
    expect(finalizeCoreMigrationForExistingVitePlus(workspaceInfo, true).scripts).toBe(true);
    const appPkg = readJson(path.join(appDir, 'package.json')) as {
      scripts: Record<string, string>;
    };
    expect(appPkg.scripts.dev).toBe('vp dev');
  });
});

// Regression: templates such as `create-fate` ship a populated vite.config.ts
// alongside a standalone `.oxfmtrc.jsonc` / `.oxlintrc.json`. The merge step
// must not insert a second `fmt:` / `lint:` block when one is already present.
describe('rewriteStandaloneProject — preserves existing fmt/lint blocks', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-merge-existing-'));
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ name: 'test', devDependencies: { vite: '^7.0.0' } }),
    );
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('does not duplicate fmt block when vite.config.ts already has one and .oxfmtrc.jsonc exists', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'vite.config.ts'),
      `import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {
    singleQuote: true,
  },
});
`,
    );
    fs.writeFileSync(
      path.join(tmpDir, '.oxfmtrc.jsonc'),
      JSON.stringify({ singleQuote: false }, null, 2),
    );

    rewriteStandaloneProject(tmpDir, makeWorkspaceInfo(tmpDir, PackageManager.pnpm), true, true);

    const viteConfig = fs.readFileSync(path.join(tmpDir, 'vite.config.ts'), 'utf8');
    expect(viteConfig.match(/\bfmt\s*:/g)?.length).toBe(1);
    // Template-authored value wins (singleQuote: true) — standalone config dropped.
    expect(viteConfig).toContain('singleQuote: true');
    expect(viteConfig).not.toContain('singleQuote: false');
    // Redundant standalone file removed.
    expect(fs.existsSync(path.join(tmpDir, '.oxfmtrc.jsonc'))).toBe(false);
  });
});

describe('detectLegacyGitHooksMigrationCandidate', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-legacy-hooks-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('detects leftover husky and lint-staged in an existing Vite+ project', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        scripts: { prepare: 'husky' },
        devDependencies: { husky: '^9.1.7', 'lint-staged': '^16.2.7', 'vite-plus': 'latest' },
        'lint-staged': { '*': 'vp check --fix' },
      }),
    );

    expect(detectLegacyGitHooksMigrationCandidate(tmpDir)).toBe(true);
  });

  it('does not treat a completed Vite+ project as needing hook migration', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        scripts: { prepare: 'vp config' },
        devDependencies: { 'vite-plus': 'latest' },
      }),
    );
    fs.mkdirSync(path.join(tmpDir, '.vite-hooks'));

    expect(detectLegacyGitHooksMigrationCandidate(tmpDir)).toBe(false);
  });

  it('does not treat standalone lint-staged config as active hook migration', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        devDependencies: { 'vite-plus': 'latest' },
      }),
    );
    fs.writeFileSync(path.join(tmpDir, 'lint-staged.config.mjs'), 'export default {};\n');

    expect(detectLegacyGitHooksMigrationCandidate(tmpDir)).toBe(false);
  });

  it('does not treat a passive .husky directory as active hook migration', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        devDependencies: { 'vite-plus': 'latest' },
      }),
    );
    fs.mkdirSync(path.join(tmpDir, '.husky'));

    expect(detectLegacyGitHooksMigrationCandidate(tmpDir)).toBe(false);
  });

  it('does not treat passive husky or lint-staged dependencies as active hook migration', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        devDependencies: {
          husky: '^9.1.7',
          'lint-staged': '^16.2.7',
          'vite-plus': 'latest',
        },
      }),
    );

    expect(detectLegacyGitHooksMigrationCandidate(tmpDir)).toBe(false);
  });
});

describe('preflightGitHooksSetup husky catalog resolution', () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-test-husky-catalog-'));
    // A `.git` dir at the project root so the subdirectory check passes.
    fs.mkdirSync(path.join(tmpDir, '.git'));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it('resolves a `catalog:` husky version from the pnpm catalog and allows hooks', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ scripts: { prepare: 'husky' }, devDependencies: { husky: 'catalog:' } }),
    );
    fs.writeFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), 'catalog:\n  husky: ^9.1.7\n');

    expect(preflightGitHooksSetup(tmpDir, PackageManager.pnpm)).toBeNull();
  });

  it('resolves the explicit `catalog:default` alias from the top-level catalog', () => {
    // pnpm reserves `default` for the top-level `catalog:` map, so `catalog:default`
    // must resolve there rather than a named `catalogs.default` entry.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({
        scripts: { prepare: 'husky' },
        devDependencies: { husky: 'catalog:default' },
      }),
    );
    fs.writeFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), 'catalog:\n  husky: ^9.1.7\n');

    expect(preflightGitHooksSetup(tmpDir, PackageManager.pnpm)).toBeNull();
  });

  it('flags a `catalog:` husky version that resolves to <9 in the pnpm catalog', () => {
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ scripts: { prepare: 'husky' }, devDependencies: { husky: 'catalog:' } }),
    );
    fs.writeFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), 'catalog:\n  husky: ^8.0.0\n');

    expect(preflightGitHooksSetup(tmpDir, PackageManager.pnpm)).toContain('husky <9.0.0');
  });

  it('does not read a foreign catalog: a yarn project ignores a leftover pnpm-workspace.yaml', () => {
    // A `catalog:` spec is only meaningful to the active package manager, so a
    // stray pnpm-workspace.yaml in a yarn repo must not satisfy husky's version.
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ scripts: { prepare: 'husky' }, devDependencies: { husky: 'catalog:' } }),
    );
    fs.writeFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), 'catalog:\n  husky: ^9.1.7\n');

    // Yarn's catalog source (.yarnrc.yml) is absent, so husky stays unresolved
    // and the preflight warns instead of trusting the pnpm catalog.
    expect(preflightGitHooksSetup(tmpDir, PackageManager.yarn)).toContain(
      'Could not determine husky version from "catalog:"',
    );
  });

  it('uses the active package manager catalog over a foreign one', () => {
    // Discriminating case: yarn's own catalog pins a compatible husky while a
    // leftover pnpm-workspace.yaml pins an incompatible one. Reading yarn's
    // catalog returns null (allowed); wrongly reading pnpm's would warn about
    // husky <9, and broken resolution would warn "Could not determine".
    fs.writeFileSync(
      path.join(tmpDir, 'package.json'),
      JSON.stringify({ scripts: { prepare: 'husky' }, devDependencies: { husky: 'catalog:' } }),
    );
    fs.writeFileSync(path.join(tmpDir, '.yarnrc.yml'), 'catalog:\n  husky: ^9.1.7\n');
    fs.writeFileSync(path.join(tmpDir, 'pnpm-workspace.yaml'), 'catalog:\n  husky: ^8.0.0\n');

    expect(preflightGitHooksSetup(tmpDir, PackageManager.yarn)).toBeNull();
  });
});
