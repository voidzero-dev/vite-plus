#!/usr/bin/env node
import module from 'node:module';

import { resolveConfig } from '@voidzero-dev/vite-plus-core';
import { build, type UserConfig, globalLogger } from '@voidzero-dev/vite-plus-core/lib';
import { cac } from 'cac';

const cli = cac('vite lib');
cli.help();

const DEFAULT_ENV_PREFIXES = ['VITE_LIB_', 'TSDOWN_'];

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
  .option('--external <module>', 'Mark dependencies as external')
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
    // support `TSDOWN_` for migration compatibility
    default: DEFAULT_ENV_PREFIXES,
  })
  .option('--on-success <command>', 'Command to run on success')
  .option('--copy <dir>', 'Copy files to output dir')
  .option('--public-dir <dir>', 'Alias for --copy, deprecated')
  .option('--tsconfig <tsconfig>', 'Set tsconfig path')
  .option('--unbundle', 'Unbundle mode')
  .option('-W, --workspace [dir]', 'Enable workspace mode')
  .option('-F, --filter <pattern>', 'Filter configs (cwd or name), e.g. /pkg-name$/ or pkg-name')
  .option('--exports', 'Generate export-related metadata for package.json (experimental)')
  .action(async (input: string[], flags: UserConfig) => {
    const viteConfig = await resolveConfig({ root: process.cwd() }, 'build');
    if (input.length > 0) flags.entry = input;
    // TODO: set default envPrefix after tsdown upgrade
    await build({
      ...(viteConfig.lib as UserConfig),
      ...flags,
      config: false,
    });
  });

export async function runCLI(): Promise<void> {
  cli.parse(process.argv, { run: false });

  // TODO: enable debug after tsdown upgrade
  // enableDebug(cli.options)

  try {
    await cli.runMatchedCommand();
  } catch (error: any) {
    globalLogger.error(String(error.stack || error.message));
    process.exit(1);
  }
}

if (module.enableCompileCache) {
  module.enableCompileCache();
}
runCLI();
