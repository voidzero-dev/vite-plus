#!/usr/bin/env node
import module from 'node:module';

import {
  buildWithConfigs,
  resolveUserConfig,
  globalLogger,
  enableDebug,
  type InlineConfig,
  type ResolvedConfig,
} from '@voidzero-dev/vite-plus-core/pack';
import { cac } from 'cac';

import { resolveViteConfig } from './resolve-vite-config.ts';

// Matches a `.d.ts` / `.d.mts` / `.d.cts` importer.
const RE_DTS = /\.d\.[cm]?ts$/;
// Bare specifier for postcss / lightningcss (root or any subpath).
const EXTERNAL_DTS_PKG_RE = /^(?:postcss|lightningcss)(?:\/|$)/;

/**
 * Rolldown plugin that keeps `postcss` / `lightningcss` external to the DTS
 * bundle.
 *
 * The DTS bundler resolves these packages and tries to inline their `.d.ts`
 * files. postcss ships its public types as a CJS `export = postcss` over a
 * `declare namespace postcss { export { AtRule, ... } }` (see
 * postcss/lib/postcss.d.ts), and its ESM types entry (postcss.d.mts) re-exports
 * those names with `export { AtRule, ... } from './postcss.js'`. The bundler
 * cannot map named imports onto an `export =`'d namespace's members, so every
 * consumer `import type { AtRule } from 'postcss'` becomes a MISSING_EXPORT
 * error.
 *
 * Marking these packages external for `.d.ts` importers leaves the import
 * untouched in the emitted declarations (`import type { AtRule } from 'postcss'`),
 * which is how third-party packages should be treated in a DTS bundle anyway.
 * They are only externalized when imported *from a declaration file*, so runtime
 * bundling is unaffected.
 */
function externalDtsTypeOnlyPlugin() {
  return {
    name: 'vite-plus:external-dts-type-only',
    resolveId: {
      order: 'pre' as const,
      handler(id: string, importer: string | undefined) {
        // Normalize Windows backslash paths to forward slashes for regex matching
        const normalizedImporter = importer?.replaceAll('\\', '/');
        if (normalizedImporter && RE_DTS.test(normalizedImporter) && EXTERNAL_DTS_PKG_RE.test(id)) {
          return { id, external: true };
        }
        return undefined;
      },
    },
  };
}

const cli = cac('vp pack');
cli.help();

// support `TSDOWN_` for migration compatibility
const DEFAULT_ENV_PREFIXES = ['VITE_PACK_', 'TSDOWN_'];

cli
  .command('[...files]', 'Bundle files', {
    ignoreOptionDefaultValue: true,
    allowUnknownOptions: true,
  })
  // Only support config file in vite.config.ts
  // .option('-c, --config <filename>', 'Use a custom config file')
  .option('--config-loader <loader>', 'Config loader to use: auto, native, tsx, unrun', {
    default: 'auto',
  })
  .option('--no-config', 'Disable config file')
  .option('-f, --format <format>', 'Bundle format: esm, cjs, iife, umd', {
    default: 'esm',
  })
  .option('--clean', 'Clean output directory, --no-clean to disable')
  .option('--deps.never-bundle <module>', 'Mark dependencies as external')
  .option('--minify', 'Minify output')
  .option('--devtools', 'Enable devtools integration')
  .option('--debug [feat]', 'Show debug logs')
  .option('--target <target>', 'Bundle target, e.g "es2015", "esnext"')
  .option('-l, --logLevel <level>', 'Set log level: info, warn, error, silent')
  .option('--fail-on-warn', 'Fail on warnings', { default: true })
  .option('--no-write', 'Disable writing files to disk, incompatible with watch mode')
  .option('-d, --out-dir <dir>', 'Output directory', { default: 'dist' })
  .option('--treeshake', 'Tree-shake bundle', { default: true })
  .option('--sourcemap', 'Generate source map', { default: false })
  .option('--shims', 'Enable cjs and esm shims', { default: false })
  .option('--platform <platform>', 'Target platform', {
    default: 'node',
  })
  .option('--dts', 'Generate dts files')
  .option('--publint', 'Enable publint', { default: false })
  .option('--attw', 'Enable Are the types wrong integration', {
    default: false,
  })
  .option('--unused', 'Enable unused dependencies check', { default: false })
  .option('-w, --watch [path]', 'Watch mode')
  .option('--ignore-watch <path>', 'Ignore custom paths in watch mode')
  .option('--from-vite [vitest]', 'Reuse config from Vite or Vitest')
  .option('--report', 'Size report', { default: true })
  .option('--env.* <value>', 'Define compile-time env variables')
  .option(
    '--env-file <file>',
    'Load environment variables from a file, when used together with --env, variables in --env take precedence',
  )
  .option('--env-prefix <prefix>', 'Prefix for env variables to inject into the bundle', {
    default: DEFAULT_ENV_PREFIXES,
  })
  .option('--on-success <command>', 'Command to run on success')
  .option('--copy <dir>', 'Copy files to output dir')
  .option('--public-dir <dir>', 'Alias for --copy, deprecated')
  .option('--tsconfig <tsconfig>', 'Set tsconfig path')
  .option('--unbundle', 'Unbundle mode')
  .option('--root <dir>', 'Root directory of input files')
  .option('--exe', 'Bundle as executable')
  .option('-W, --workspace [dir]', 'Enable workspace mode')
  .option('-F, --filter <pattern>', 'Filter configs (cwd or name), e.g. /pkg-name$/ or pkg-name')
  .option('--exports', 'Generate export-related metadata for package.json (experimental)')
  .action(async (input: string[], flags: InlineConfig) => {
    if (input.length > 0) {
      flags.entry = input;
    }
    if (flags.envPrefix === undefined) {
      flags.envPrefix = DEFAULT_ENV_PREFIXES;
    }

    async function runBuild() {
      const viteConfig = await resolveViteConfig(process.cwd(), {
        traverseUp: flags.config !== false,
      });

      const configDeps = new Set<string>();
      if (viteConfig.configFile) {
        configDeps.add(viteConfig.configFile);
      }

      const configs: ResolvedConfig[] = [];
      const packConfigs = Array.isArray(viteConfig.pack)
        ? viteConfig.pack
        : [viteConfig.pack ?? {}];
      for (const packConfig of packConfigs) {
        const merged = { ...packConfig, ...flags };
        // Keep postcss/lightningcss external to the dts bundle (see plugin doc)
        if (merged.dts) {
          const existingPlugins = Array.isArray(merged.plugins) ? merged.plugins : [];
          merged.plugins = [...existingPlugins, externalDtsTypeOnlyPlugin()];
        }
        const resolvedConfig = await resolveUserConfig(merged, flags, configDeps);
        configs.push(...resolvedConfig);
      }

      await buildWithConfigs(configs, configDeps, runBuild);
    }

    await runBuild();
  });

export async function runCLI(): Promise<void> {
  cli.parse(process.argv, { run: false });

  enableDebug(cli.options.debug);

  try {
    await cli.runMatchedCommand();
  } catch (error) {
    globalLogger.error(error instanceof Error ? error.stack || error.message : error);
    process.exit(1);
  }
}

if (module.enableCompileCache) {
  module.enableCompileCache();
}

await runCLI();
