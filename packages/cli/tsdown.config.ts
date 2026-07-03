import { createRequire } from 'node:module';

import { defineConfig } from 'tsdown';

const require = createRequire(import.meta.url);
const lintStagedPackageJson = require('lint-staged/package.json') as { version: string };

/**
 * Rewrite `../versions.js` → `./versions.js` at resolve time.
 *
 * `src/migration/migrator.ts` dynamically imports `../versions.js` (one directory up
 * from `src/migration/`). After bundling, all output lands in `dist/`, so the correct
 * runtime path is `./versions.js`. Using `resolveId` rewrites the specifier during
 * resolution rather than doing post-hoc string surgery on rendered chunks.
 */
const fixVersionsPathPlugin = {
  name: 'fix-versions-path',
  resolveId(source: string) {
    if (source === '../versions.js') {
      return { id: './versions.js', external: true };
    }
    return undefined;
  },
};

/**
 * Replace lint-staged's lib/version.js with a build-time version value.
 *
 * The original module reads ../package.json at runtime when debug logging is enabled,
 * but that file does not exist in the bundled dist/staged/bin.js.
 */
const inlineLintStagedVersionPlugin = {
  name: 'inline-lint-staged-version',
  load(id: string) {
    if (id.replaceAll('\\', '/').endsWith('/lint-staged/lib/version.js')) {
      return `export const getVersion = async () => ${JSON.stringify(lintStagedPackageJson.version)};\n`;
    }
    return undefined;
  },
};

export default defineConfig([
  // ESM — all entry points bundled to dist/
  {
    name: 'cli',
    entry: {
      bin: './src/bin.ts',
      index: './src/index.ts',
      'define-config': './src/define-config.ts',
      fmt: './src/fmt.ts',
      lint: './src/lint.ts',
      'oxlint-plugin': './src/oxlint-plugin.ts',
      'tsgolint-path': './src/utils/tsgolint-path.ts',
      pack: './src/pack.ts',
      'pack-bin': './src/pack-bin.ts',
      // Global commands — explicit entries ensure lazy loading via dynamic import in bin.ts.
      // Without these, tsdown inlines them into bin.js, breaking on-demand loading.
      'create/bin': './src/create/bin.ts',
      'migration/bin': './src/migration/bin.ts',
      'migration/compat/worker': './src/migration/compat/worker.ts',
      version: './src/version.ts',
      'config/bin': './src/config/bin.ts',
      'staged/bin': './src/staged/bin.ts',
    },
    outDir: 'dist',
    format: 'esm',
    fixedExtension: false,
    shims: true,
    dts: true,
    clean: true,
    // NAPI binding uses a relative path that tsdown can't auto-detect from package.json
    deps: { neverBundle: [/\.\.\/binding\/index\.(js|cjs)/] },
    inputOptions: {
      resolve: {
        // Prefer ESM entry (module field) over CJS/UMD (main field) for bundled deps.
        // Without this, packages like jsonc-parser resolve to their UMD entry which
        // has internal require('./impl/...') calls that break in bundled ESM output.
        mainFields: ['module', 'main'],
      },
    },
    plugins: [fixVersionsPathPlugin, inlineLintStagedVersionPlugin],
  },

  // CJS — dual-format entries
  {
    name: 'cli-cjs',
    entry: {
      'define-config': './src/define-config.ts',
      index: './src/index.cts',
    },
    outDir: 'dist',
    format: 'cjs',
    dts: false,
    clean: false,
  },
]);
