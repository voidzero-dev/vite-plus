/**
 * Unified entry point for both the local CLI (via bin/vp) and the global CLI (via Rust vp binary).
 *
 * Global commands (create, migrate, config, staged, --version) are handled by tsdown-bundled modules.
 * All other commands are delegated to the Rust core through NAPI bindings, which
 * uses JavaScript tool resolver functions to locate tool binaries.
 *
 * When called from the global CLI, the Rust binary resolves the project's local
 * vite-plus installation using oxc_resolver and runs its dist/bin.js directly.
 * If no local installation is found, this global dist/bin.js is used as fallback.
 */

import path from 'node:path';

import { run } from '../binding/index.js';
import { applyToolInitConfigToViteConfig, inspectInitCommand } from './init-config.ts';
import { doc } from './resolve-doc.ts';
import { fmt } from './resolve-fmt.ts';
import { lint } from './resolve-lint.ts';
import { pack } from './resolve-pack.ts';
import { test } from './resolve-test.ts';
import { resolveUniversalViteConfig } from './resolve-vite-config.ts';
import { vite } from './resolve-vite.ts';
import { accent, errorMsg, log } from './utils/terminal.ts';

function getErrorMessage(err: unknown): string {
  if (err instanceof Error) {
    return err.message;
  }

  if (typeof err === 'object' && err && 'message' in err && typeof err.message === 'string') {
    return err.message;
  }

  return String(err);
}

// Parse command line arguments
let args = process.argv.slice(2);

// Transform `vp help [command]` into `vp [command] --help`
if (args[0] === 'help' && args[1]) {
  args = [args[1], '--help', ...args.slice(2)];
  process.argv = process.argv.slice(0, 2).concat(args);
}

const command = args[0];
process.env.VP_COMMAND = command ?? '';

// Global commands — handled by tsdown-bundled modules in dist/
if (command === 'create') {
  await import('./create/bin.js');
} else if (command === 'migrate') {
  await import('./migration/bin.js');
} else if (command === 'config') {
  await import('./config/bin.js');
} else if (command === '--version' || command === '-V') {
  await import('./version.js');
} else if (command === 'staged') {
  await import('./staged/bin.js');
} else {
  // All other commands — delegate to Rust core via NAPI binding
  try {
    const initInspection = inspectInitCommand(command, args.slice(1));
    if (
      initInspection.handled &&
      initInspection.configKey &&
      initInspection.hasExistingConfigKey &&
      initInspection.existingViteConfigPath
    ) {
      log(
        `Skipped initialization: '${accent(initInspection.configKey)}' already exists in '${accent(path.basename(initInspection.existingViteConfigPath))}'.`,
      );
      process.exit(0);
    }

    const exitCode = await run({
      lint,
      pack,
      fmt,
      vite,
      test,
      doc,
      resolveUniversalViteConfig,
      args: process.argv.slice(2),
    });

    let finalExitCode = exitCode;
    if (exitCode === 0) {
      try {
        const result = await applyToolInitConfigToViteConfig(command, args.slice(1));
        if (
          result.handled &&
          result.action === 'added' &&
          result.configKey &&
          result.viteConfigPath
        ) {
          log(
            `Added '${accent(result.configKey)}' to '${accent(path.basename(result.viteConfigPath))}'.`,
          );
        }
        if (
          result.handled &&
          result.action === 'skipped-existing' &&
          result.configKey &&
          result.viteConfigPath
        ) {
          log(
            `Skipped initialization: '${accent(result.configKey)}' already exists in '${accent(path.basename(result.viteConfigPath))}'.`,
          );
        }
      } catch (err) {
        console.error('[Vite+] Failed to initialize config in vite.config.ts:', err);
        finalExitCode = 1;
      }
    }

    process.exit(finalExitCode);
  } catch (err) {
    errorMsg(getErrorMessage(err));
    process.exit(1);
  }
}
