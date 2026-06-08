import { describe, expect, test } from '@voidzero-dev/vite-plus-test';
import * as semver from 'semver';

import { mergePnpmWorkspaces } from '../sync-remote-deps.ts';

describe('mergePnpmWorkspaces() minimumReleaseAgeExclude', () => {
  test('drops versioned upstream entries already covered by a glob or bare pattern', () => {
    // The main workspace already excludes whole namespaces via globs
    // (`@oxc-minify/*`) and bare names (`oxc-parser`). Upstream rolldown/vite
    // workspaces list the exact versioned bindings explicitly. Those are
    // redundant: pnpm already excludes every version under the broader rule,
    // so they must not be re-added to pnpm-workspace.yaml.
    const main = {
      minimumReleaseAgeExclude: [
        '@oxc-minify/*',
        '@oxc-parser/*',
        '@oxc-project/*',
        '@oxc-transform/*',
        'oxc-minify',
        'oxc-parser',
        'oxc-transform',
        'lodash-es@4.18.1',
      ],
    };
    const rolldown = {
      minimumReleaseAgeExclude: [
        '@oxc-minify/binding-darwin-arm64@0.134.0',
        '@oxc-minify/binding-linux-x64-gnu@0.134.0',
        '@oxc-parser/binding-darwin-arm64@0.134.0',
        '@oxc-project/runtime@0.134.0',
        '@oxc-project/types@0.134.0',
        '@oxc-transform/binding-darwin-arm64@0.134.0',
        'oxc-minify@0.134.0',
        'oxc-parser@0.134.0',
        'oxc-transform@0.134.0',
      ],
    };
    const rolldownVite = {};

    const result = mergePnpmWorkspaces(main, rolldown, rolldownVite, semver);

    // Nothing redundant should survive; the original broad rules plus the
    // genuinely-specific `lodash-es@4.18.1` pin remain.
    expect(result.minimumReleaseAgeExclude).toEqual([
      '@oxc-minify/*',
      '@oxc-parser/*',
      '@oxc-project/*',
      '@oxc-transform/*',
      'oxc-minify',
      'oxc-parser',
      'oxc-transform',
      'lodash-es@4.18.1',
    ]);
  });

  test('keeps a versioned entry when no broader pattern covers it', () => {
    const main = {
      minimumReleaseAgeExclude: ['lodash-es@4.18.1'],
    };
    const rolldown = {
      minimumReleaseAgeExclude: ['some-pkg@1.2.3'],
    };

    const result = mergePnpmWorkspaces(main, rolldown, {}, semver);

    expect(result.minimumReleaseAgeExclude).toContain('lodash-es@4.18.1');
    expect(result.minimumReleaseAgeExclude).toContain('some-pkg@1.2.3');
  });

  test('keeps version-less patterns and dedupes exact duplicates', () => {
    const main = {
      minimumReleaseAgeExclude: ['@oxc-parser/*', 'oxc-parser'],
    };
    const rolldown = {
      minimumReleaseAgeExclude: ['oxc-parser', '@oxc-parser/*'],
    };

    const result = mergePnpmWorkspaces(main, rolldown, {}, semver);

    expect(result.minimumReleaseAgeExclude).toEqual(['@oxc-parser/*', 'oxc-parser']);
  });
});
