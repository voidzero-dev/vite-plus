import { defineConfig } from 'tsdown';

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
      pack: './src/pack.ts',
      'pack-bin': './src/pack-bin.ts',
      // Global commands — explicit entries ensure lazy loading via dynamic import in bin.ts.
      // Without these, tsdown inlines them into bin.js, breaking on-demand loading.
      'create/bin': './src/create/bin.ts',
      'migration/bin': './src/migration/bin.ts',
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
    plugins: [fixVersionsPathPlugin],
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
