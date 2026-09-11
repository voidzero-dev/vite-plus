import { spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import * as semver from 'semver';
import { describe, expect, test } from 'vitest';
import * as yaml from 'yaml';

import { VITEST_VERSION } from '../../../cli/src/utils/constants.ts';
import {
  mergePnpmWorkspaces,
  syncCargoOxcVersions,
  syncViteDevtoolsDependencies,
} from '../sync-remote-deps.ts';
import { alignVendoredVitestDependencies } from '../vendored-vitest.mjs';

describe('vendored Vitest v5 bridge', () => {
  test('can be imported from stdin without running the bootstrap', () => {
    const root = mkdtempSync(join(tmpdir(), 'vp-vendored-vitest-import-'));
    try {
      const url = new URL('../vendored-vitest.mjs', import.meta.url).href;
      const result = spawnSync(process.execPath, ['--input-type=module', '-'], {
        cwd: root,
        encoding: 'utf8',
        input: `import ${JSON.stringify(url)};`,
      });
      expect(result.stderr).toBe('');
      expect(result.status).toBe(0);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  test('keeps the bootstrap runtime pin in sync with the root catalog', () => {
    const workspace = yaml.parse(
      readFileSync(new URL('../../../../pnpm-workspace.yaml', import.meta.url), 'utf8'),
    );
    expect(workspace.catalog.vitest).toBe(VITEST_VERSION);
  });

  test.each(['4.1.10', '^5.0.0', '5.1.0-beta.1'])(
    'rejects an unsupported bootstrap runtime pin %s before changing manifests',
    (version) => {
      const root = mkdtempSync(join(tmpdir(), 'vp-vendored-vitest-bootstrap-'));
      try {
        const constantsDir = join(root, 'packages/cli/src/utils');
        mkdirSync(constantsDir, { recursive: true });
        writeFileSync(
          join(constantsDir, 'constants.ts'),
          `export const VITEST_VERSION = '${version}';\n`,
        );
        const script = join(root, 'vendored-vitest.mjs');
        copyFileSync(new URL('../vendored-vitest.mjs', import.meta.url), script);
        const result = spawnSync(process.execPath, [script], { cwd: root, encoding: 'utf8' });
        expect(result.status).not.toBe(0);
        expect(result.stderr).toContain('VITEST_VERSION must declare an exact stable v5 version');
      } finally {
        rmSync(root, { recursive: true, force: true });
      }
    },
  );

  test.each(['5.0.0', '5.1.2'])(
    'runs before dependency installation with the selected %s runtime pin',
    (version) => {
      const root = mkdtempSync(join(tmpdir(), 'vp-vendored-vitest-bootstrap-'));
      try {
        const constantsDir = join(root, 'packages/cli/src/utils');
        mkdirSync(constantsDir, { recursive: true });
        writeFileSync(
          join(constantsDir, 'constants.ts'),
          `export const VITEST_VERSION = '${version}';\n`,
        );
        // Copy the entry point outside the repo so it cannot resolve node_modules.
        const script = join(root, 'vendored-vitest.mjs');
        copyFileSync(new URL('../vendored-vitest.mjs', import.meta.url), script);
        const directory = join(root, 'vite/packages/vite');
        mkdirSync(directory, { recursive: true });
        const manifest = join(directory, 'package.json');
        writeFileSync(
          manifest,
          JSON.stringify({
            dependencies: { '@vitest/utils': '4.1.10' },
            devDependencies: { vitest: 'catalog:' },
            optionalDependencies: { '@vitest/spy': '^4.1.10' },
          }),
        );
        const run = () => spawnSync(process.execPath, [script], { cwd: root, encoding: 'utf8' });
        const first = run();
        expect(first.stderr).toBe('');
        expect(first.status).toBe(0);
        const source = readFileSync(manifest, 'utf8');
        expect(JSON.parse(source)).toEqual({
          dependencies: { '@vitest/utils': version },
          devDependencies: { vitest: 'catalog:' },
          optionalDependencies: { '@vitest/spy': version },
        });
        expect(run().status).toBe(0);
        expect(readFileSync(manifest, 'utf8')).toBe(source);
      } finally {
        rmSync(root, { recursive: true, force: true });
      }
    },
  );

  test('resolves the reviewed Vitest major catalog conflict', () => {
    const merged = mergePnpmWorkspaces(
      { catalog: { vitest: '5.0.0' } },
      { catalog: { vitest: '^4.1.6' } },
      { catalog: { vitest: '^4.1.10' } },
      semver,
    );
    expect(merged.catalog?.vitest).toBe('5.0.0');
  });

  test.each(['=4.1.11', '^5.1.0', '5.2.0', '^6.0.0'])(
    'preserves the selected exact runner version against upstream %s',
    (upstream) => {
      const merged = mergePnpmWorkspaces(
        { catalog: { vitest: '5.0.0' } },
        { catalog: { vitest: upstream } },
        {},
        semver,
      );
      expect(merged.catalog?.vitest).toBe('5.0.0');
    },
  );

  test('aligns official catalog entries to the selected runner and preserves the community provider', () => {
    const merged = mergePnpmWorkspaces(
      {
        catalog: {
          vitest: '5.1.2',
          '@vitest/browser': '5.0.0',
          '@vitest/browser-webdriverio': '5.0.0-rc.1',
        },
      },
      {
        catalog: {
          vitest: '^4.1.6',
          '@vitest/browser': '^5.2.0',
          '@vitest/browser-webdriverio': '5.3.0',
          '@vitest/utils': '4.1.11',
          '@vitest/expect': '4.1.11',
          '@vitest/runner': '4.1.11',
          '@vitest/eslint-plugin': '^1.0.0',
        },
      },
      {
        catalog: {
          '@vitest/coverage-v8': '4.1.11',
          '@vitest/coverage-istanbul': '^4.1.11',
          '@vitest/ui': '^5.2.0',
          '@vitest/web-worker': '4.1.11',
        },
      },
      semver,
    );
    expect(merged.catalog).toEqual({
      vitest: '5.1.2',
      '@vitest/browser': '5.1.2',
      '@vitest/browser-webdriverio': '5.0.0-rc.1',
      '@vitest/utils': '5.1.2',
      '@vitest/coverage-v8': '5.1.2',
      '@vitest/coverage-istanbul': '5.1.2',
      '@vitest/ui': '5.1.2',
      '@vitest/web-worker': '5.1.2',
      '@vitest/eslint-plugin': '^1.0.0',
    });
  });

  test.each(['^5.0.0', '=5.0.0', '4.1.11', '6.0.0', '5.1.0-beta.1'])(
    'rejects an unreviewed root runner version %s',
    (vitest) => {
      expect(() => mergePnpmWorkspaces({ catalog: { vitest } }, {}, {}, semver)).toThrow(
        'The root Vitest catalog entry must be an exact stable v5 version',
      );
    },
  );

  test('does not infer a missing root pin from an upstream catalog', () => {
    expect(() => mergePnpmWorkspaces({}, { catalog: { vitest: '^4.1.6' } }, {}, semver)).toThrow(
      'The root Vitest catalog entry must be an exact stable v5 version',
    );
  });

  test('still resolves tinybench to the higher major version', () => {
    const merged = mergePnpmWorkspaces(
      { catalog: { tinybench: '^6.0.0' } },
      { catalog: { tinybench: '^2.9.0' } },
      {},
      semver,
    );
    expect(merged.catalog?.tinybench).toBe('^6.0.0');
  });

  test.each(['@vitest/expect', '@vitest/runner'])(
    'rejects actual dependencies on removed package %s',
    (name) => {
      const root = mkdtempSync(join(tmpdir(), 'vp-vendored-vitest-'));
      try {
        mkdirSync(join(root, 'vite', 'packages'), { recursive: true });
        writeFileSync(
          join(root, 'vite', 'package.json'),
          JSON.stringify({ devDependencies: { [name]: 'catalog:' } }),
        );
        expect(() => alignVendoredVitestDependencies(root, '5.0.0')).toThrow(
          `Migrate removed ${name} use`,
        );
      } finally {
        rmSync(root, { recursive: true, force: true });
      }
    },
  );

  test('aligns direct Vite workspace dependencies and leaves fixture manifests alone', () => {
    const root = mkdtempSync(join(tmpdir(), 'vp-vendored-vitest-'));
    try {
      const directory = join(root, 'vite');
      mkdirSync(join(directory, 'packages/test/fixtures'), { recursive: true });
      writeFileSync(
        join(directory, 'package.json'),
        JSON.stringify({
          devDependencies: { vitest: '^4.0.0', '@vitest/eslint-plugin': '^1.0.0' },
        }),
      );
      writeFileSync(
        join(directory, 'packages/test/package.json'),
        JSON.stringify({
          devDependencies: {
            vitest: 'catalog:',
            '@vitest/utils': '4.1.10',
            '@vitest/web-worker': 'catalog:legacy',
            '@vitest/browser-webdriverio': '^5.0.0-beta.5',
          },
        }),
      );
      writeFileSync(join(directory, 'packages/test/fixtures/package.json'), '\ufeff{}');
      alignVendoredVitestDependencies(root, '5.0.0');
      expect(
        JSON.parse(readFileSync(join(directory, 'package.json'), 'utf8')).devDependencies,
      ).toEqual({ vitest: '5.0.0', '@vitest/eslint-plugin': '^1.0.0' });
      expect(
        JSON.parse(readFileSync(join(directory, 'packages/test/package.json'), 'utf8'))
          .devDependencies,
      ).toEqual({
        vitest: 'catalog:',
        '@vitest/utils': '5.0.0',
        '@vitest/web-worker': '5.0.0',
        '@vitest/browser-webdriverio': '^5.0.0-beta.5',
      });
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });

  test('leaves Rolldown Vitest dependencies unchanged', () => {
    const root = mkdtempSync(join(tmpdir(), 'vp-vendored-vitest-'));
    try {
      mkdirSync(join(root, 'vite/packages'), { recursive: true });
      const directory = join(root, 'rolldown/packages/browser-tests');
      mkdirSync(directory, { recursive: true });
      const source = JSON.stringify({
        devDependencies: {
          vitest: 'catalog:',
          '@vitest/browser-playwright': '4.1.10',
          '@vitest/runner': '4.1.10',
        },
      });
      const manifest = join(directory, 'package.json');
      writeFileSync(manifest, source);
      alignVendoredVitestDependencies(root, '5.0.0');
      expect(readFileSync(manifest, 'utf8')).toBe(source);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  });
});

describe('syncViteDevtoolsDependencies()', () => {
  test('uses the DevTools ranges declared by Vite', () => {
    const corePackage = {
      devDependencies: { '@vitejs/devtools': '^0.6.1' },
      peerDependencies: { '@vitejs/devtools': '^0.4.0 || ^0.5.0 || ^0.6.0' },
    };
    const vitePackage = {
      devDependencies: { '@vitejs/devtools': '^0.4.12' },
      peerDependencies: { '@vitejs/devtools': '^0.4.0 || ^0.5.0' },
    };

    syncViteDevtoolsDependencies(corePackage, vitePackage);

    expect(corePackage.devDependencies['@vitejs/devtools']).toBe('^0.4.12');
    expect(corePackage.peerDependencies['@vitejs/devtools']).toBe('^0.4.0 || ^0.5.0');
  });

  test('fails when Vite no longer declares a DevTools range', () => {
    expect(() => syncViteDevtoolsDependencies({}, {})).toThrow(
      'Vite package.json must define @vitejs/devtools in devDependencies and peerDependencies',
    );
  });
});

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
