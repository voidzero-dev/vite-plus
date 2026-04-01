import { builtinModules } from 'node:module';

import { defineConfig } from 'rolldown';

// Node.js built-in modules (both bare and node:-prefixed).
// Needed because lint-staged's CJS dependencies use require('util') etc.
const nodeBuiltins = new Set(builtinModules.flatMap((m) => [m, `node:${m}`]));

export default defineConfig({
  input: {
    create: './src/create/bin.ts',
    migrate: './src/migration/bin.ts',
    version: './src/version.ts',
    config: './src/config/bin.ts',
    mcp: './src/mcp/bin.ts',
    staged: './src/staged/bin.ts',
  },
  treeshake: false,
  external(source) {
    if (nodeBuiltins.has(source)) {
      return true;
    }
    if (
      source === 'cross-spawn' ||
      source === 'picocolors' ||
      source === '@voidzero-dev/vite-plus-core'
    ) {
      return true;
    }
    if (source === '../../binding/index.js' || source === '../binding/index.js') {
      return true;
    }
    if (source === '../versions.js') {
      return true;
    }
    return false;
  },
  plugins: [
    {
      name: 'fix-external-paths',
      // Rewrite external import paths for the output directory (dist/global/).
      // Rolldown normalizes relative paths from source locations, but the output
      // is two directories deep (dist/global/), so the paths need adjustment.
      renderChunk(code) {
        let result = code;
        // ../../binding/index.js → Rolldown normalizes to ../binding/index.js, needs ../../
        if (result.includes('../binding/index.js')) {
          result = result.replaceAll('../binding/index.js', '../../binding/index.js');
        }
        // ../versions.js → Rolldown normalizes to ./versions.js, needs ../
        if (result.includes('./versions.js')) {
          result = result.replaceAll('./versions.js', '../versions.js');
        }
        return result !== code ? { code: result } : null;
      },
    },
    {
      name: 'inject-cjs-require',
      // Inject createRequire into chunks that use rolldown's __require CJS shim.
      // lint-staged's CJS dependencies use require('util') etc., which fails in ESM.
      // By providing a real `require` via createRequire, the shim works correctly.
      renderChunk(code) {
        if (code.includes('typeof require')) {
          const injection = `import { createRequire as __createRequire } from 'node:module';\nconst require = __createRequire(import.meta.url);\n`;
          return { code: injection + code };
        }
        return null;
      },
    },
  ],
  output: {
    format: 'esm',
    dir: './dist/global',
    cleanDir: true,
  },
});
