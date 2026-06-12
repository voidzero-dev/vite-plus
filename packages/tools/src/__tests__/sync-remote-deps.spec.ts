import { describe, expect, test } from 'vitest';
import * as semver from 'semver';

import { mergePnpmWorkspaces, syncCargoOxcVersions } from '../sync-remote-deps.ts';

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

// Reproduces the upstream-upgrade build break: when the bumped rolldown hash
// pins a newer oxc release (e.g. 0.135.0 / oxc_index 5), the vendored rolldown
// crates fail to compile against vp's stale `Cargo.toml` oxc pin (0.134.0). The
// root `Cargo.toml` oxc versions must follow rolldown's `Cargo.toml`.
describe('syncCargoOxcVersions()', () => {
  const mainCargo = `[workspace]
members = ["crates/*"]

[workspace.dependencies]
serde = "1"

# oxc crates with the same version
oxc = { version = "0.134.0", features = [
  "ast_visit",
  "transformer",
] }
oxc_allocator = { version = "0.134.0", features = ["pool"] }
oxc_ast = "0.134.0"
oxc_parser = "0.134.0"
oxc_span = "0.134.0"
oxc_traverse = "0.134.0"

# oxc crates in their own repos
oxc_index = { version = "4", features = ["rayon", "serde"] }
oxc_resolver = { version = "11.21.0", features = ["yarn_pnp"] }
oxc_sourcemap = "7"

[profile.release]
lto = true
`;

  const rolldownCargo = `[workspace]
members = ["crates/*"]

[workspace.dependencies]
# oxc crates with the same version
oxc = { version = "0.135.0", features = [
  "ast_visit",
  "transformer",
] }
oxc_allocator = { version = "0.135.0", features = ["pool"] }
oxc_traverse = { version = "0.135.0" }

# oxc crates in their own repos
oxc_index = { version = "5", features = ["rayon", "serde"] }
oxc_resolver = { version = "11.21.0", features = ["yarn_pnp"] }
oxc_sourcemap = { version = "7" }
`;

  test('bumps the oxc same-version family and oxc_index to match rolldown', () => {
    const { content, changes } = syncCargoOxcVersions(mainCargo, rolldownCargo);

    // Same-version family follows rolldown's umbrella `oxc` version, including
    // crates rolldown does not declare explicitly (oxc_ast/oxc_parser/oxc_span).
    expect(content).toContain('oxc = { version = "0.135.0"');
    expect(content).toContain('oxc_allocator = { version = "0.135.0"');
    expect(content).toContain('oxc_ast = "0.135.0"');
    expect(content).toContain('oxc_parser = "0.135.0"');
    expect(content).toContain('oxc_span = "0.135.0"');
    expect(content).toContain('oxc_traverse = "0.135.0"');
    // Independently-versioned crate follows rolldown's own pin.
    expect(content).toContain('oxc_index = { version = "5"');
    // Unchanged crates stay put.
    expect(content).toContain('oxc_resolver = { version = "11.21.0"');
    expect(content).toContain('oxc_sourcemap = "7"');
    // Features and unrelated entries are preserved.
    expect(content).toContain('"ast_visit",');
    expect(content).toContain('serde = "1"');

    const changed = Object.fromEntries(changes.map((c) => [c.key, c.to]));
    expect(changed).toMatchObject({
      oxc: '0.135.0',
      oxc_allocator: '0.135.0',
      oxc_ast: '0.135.0',
      oxc_parser: '0.135.0',
      oxc_span: '0.135.0',
      oxc_traverse: '0.135.0',
      oxc_index: '5',
    });
    // No spurious changes for already-matching crates.
    expect(changes.find((c) => c.key === 'oxc_resolver')).toBeUndefined();
    expect(changes.find((c) => c.key === 'oxc_sourcemap')).toBeUndefined();
  });

  test('is a no-op when versions already match', () => {
    const { content, changes } = syncCargoOxcVersions(mainCargo, mainCargo);
    expect(content).toBe(mainCargo);
    expect(changes).toEqual([]);
  });

  test('only rewrites entries inside [workspace.dependencies]', () => {
    const withPatch = `${mainCargo}
[patch.crates-io]
# pinned override, must not be touched by the oxc sync
oxc_ast = { git = "https://example.com/oxc", rev = "abc" }
`;
    const { content } = syncCargoOxcVersions(withPatch, rolldownCargo);
    // The dependency entry is bumped...
    expect(content).toContain('oxc_ast = "0.135.0"');
    // ...but the [patch] git override is left intact.
    expect(content).toContain('oxc_ast = { git = "https://example.com/oxc", rev = "abc" }');
  });
});
