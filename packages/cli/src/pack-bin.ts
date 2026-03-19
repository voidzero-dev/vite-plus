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

import { resolveViteConfig } from './resolve-vite-config.js';

/**
 * Rolldown plugin that transforms value imports/exports to type-only in external
 * packages' .d.ts files. Some packages (e.g. postcss, lightningcss) use
 * `import { X }` and `export { X } from` instead of their type-only equivalents,
 * which causes MISSING_EXPORT warnings from the DTS bundler.
 *
 * Since .d.ts files contain only type information, all imports/exports are
 * inherently type-only, so this transformation is always safe.
 */
const EXTERNAL_DTS_INTERNAL_RE = /node_modules\/(postcss|lightningcss)\/.*\.d\.(ts|mts|cts)$/;
// Match consumer .d.ts files that import from postcss/lightningcss.
// In CI (installed from tgz): node_modules/vite-plus-core/dist/...
// In local development (symlinked workspace): packages/core/dist/...
const EXTERNAL_DTS_CONSUMER_RE =
  /(?:vite-plus-core|packages\/core)\/.*lightningcssOptions\.d\.ts$|(?:vite-plus-core|packages\/core)\/dist\/.*\.d\.ts$/;
const EXTERNAL_DTS_FIX_RE = new RegExp(
  `${EXTERNAL_DTS_INTERNAL_RE.source}|${EXTERNAL_DTS_CONSUMER_RE.source}`,
);

function externalDtsTypeOnlyPlugin() {
  return {
    name: 'vite-plus:external-dts-type-only',
    transform: {
      filter: { id: { include: [EXTERNAL_DTS_FIX_RE] } },
      handler(code: string, rawId: string) {
        // Normalize Windows backslash paths to forward slashes for regex matching
        const id = rawId.replaceAll('\\', '/');
        if (EXTERNAL_DTS_INTERNAL_RE.test(id)) {
          // postcss/lightningcss internal files: transform imports only
          // (exports may include value re-exports like `export const Features`)
          return code.replace(/^(import\s+)(?!type\s)/gm, 'import type ');
        }
        // Consumer files: only transform imports from postcss/lightningcss
        return code.replace(
          /^(import\s+)(?!type\s)(.+from\s+['"](?:postcss|lightningcss)['"])/gm,
          'import type $2',
        );
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
  .option('--config-loader <loader>', 'Config loader to use: auto, native, unrun', {
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
      const viteConfig = await resolveViteConfig(process.cwd());

      const configFiles: string[] = [];
      if (viteConfig.configFile) {
        configFiles.push(viteConfig.configFile);
      }

      const configs: ResolvedConfig[] = [];
      const packConfigs = Array.isArray(viteConfig.pack)
        ? viteConfig.pack
        : [viteConfig.pack ?? {}];
      for (const packConfig of packConfigs) {
        const merged = { ...packConfig, ...flags };
        // Inject plugin to fix MISSING_EXPORT warnings from external .d.ts files
        // (postcss, lightningcss use `import`/`export` instead of `import type`/`export type`)
        if (merged.dts) {
          const existingPlugins = Array.isArray(merged.plugins) ? merged.plugins : [];
          merged.plugins = [...existingPlugins, externalDtsTypeOnlyPlugin()];
        }
        const resolvedConfig = await resolveUserConfig(merged, flags);
        configs.push(...resolvedConfig);
      }

      await buildWithConfigs(configs, configFiles, runBuild);
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
