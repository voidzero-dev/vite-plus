/**
 * Unified entry point for both the local CLI (via bin/vp) and the global CLI (via Rust vp binary).
 *
 * Global commands (create, migrate, config, hooks, staged, --version) are handled by tsdown-bundled modules.
 * All other commands are delegated to the Rust core through NAPI bindings, which
 * uses JavaScript tool resolver functions to locate tool binaries.
 *
 * When called from the global CLI, the Rust binary resolves the project's local
 * vite-plus installation using oxc_resolver and runs its dist/bin.js directly.
 * If no local installation is found, this global dist/bin.js is used as fallback.
 */

import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { ensureBlockingStdio, run } from '../binding/index.js';
import { maybePrintCommandHelp } from './help.ts';
import { applyToolInitConfigToViteConfig, inspectInitCommand } from './init-config.ts';
import { doc } from './resolve-doc.ts';
import { fmt } from './resolve-fmt.ts';
import { lint } from './resolve-lint.ts';
import { pack } from './resolve-pack.ts';
import { test } from './resolve-test.ts';
import { resolveUniversalViteConfig } from './resolve-vite-config.ts';
import { vite } from './resolve-vite.ts';
import { accent, errorMsg, log } from './utils/terminal.ts';

// Node.js sets O_NONBLOCK when pipe-backed stdio is first accessed. Materialize
// the output streams before restoring the blocking semantics expected by Rust.
void process.stdout;
void process.stderr;
ensureBlockingStdio();

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
const explicitChdirMarker = process.env.VP_EXPLICIT_CHDIR;
let explicitChdir = explicitChdirMarker === '1';
if (explicitChdirMarker !== undefined) {
  delete process.env.VP_EXPLICIT_CHDIR;
}

// Global `-C <dir>` flag: run as if vp was started in <dir>. The global Rust
// CLI parses this itself and spawns bin.js with the target cwd already set;
// this branch covers direct local-bin invocations (`pnpm exec vp -C <dir> ...`).
// Accepts `-C dir`, `-Cdir`, and `-C=dir`, matching the clap grammar.
if (args[0]?.startsWith('-C')) {
  explicitChdir = true;
  const inline = args[0].length > 2;
  const dir = inline ? args[0].slice(args[0][2] === '=' ? 3 : 2) : args[1];
  if (!dir) {
    errorMsg('-C requires a directory argument');
    process.exit(1);
  }
  const target = path.resolve(dir);
  // chdir is the single validation point: a pre-check stat can itself throw
  // (EACCES on a parent, ELOOP), while chdir reports every failure mode
  // through one catchable path.
  try {
    process.chdir(target);
  } catch (err) {
    const code = typeof err === 'object' && err !== null && 'code' in err ? err.code : undefined;
    if (code === 'ENOENT' || code === 'ENOTDIR') {
      errorMsg(`directory not found: ${dir}`);
    } else {
      errorMsg(`cannot change directory to ${dir}: ${getErrorMessage(err)}`);
    }
    process.exit(1);
  }
  if (process.platform !== 'win32') {
    // Keep the POSIX PWD in sync, like a real `cd`.
    process.env.PWD = target;
  }
  args = args.slice(inline ? 1 : 2);
  process.argv = process.argv.slice(0, 2).concat(args);
}

// `vpr` is shorthand for `vp run`: the bin/vpr shim imports this file
// unchanged (argv0 tells them apart, like the Rust shim dispatch), and the
// rewrite happens here, after -C consumption, so `vpr -C <dir> <task>`
// orders itself correctly by construction.
if (path.basename(process.argv[1] ?? '') === 'vpr') {
  args = ['run', ...args];
  process.argv = process.argv.slice(0, 2).concat(args);
}

// The list the Rust CLI parses: after the -C and vpr rewrites above (both
// are consumed/settled here), before the help transform below (the Rust CLI
// applies `help [command]` itself and needs the untransformed list to tell
// `vp help fmt` apart from `vp fmt --help`).
const rustCliArgs = args;

// Transform `vp help [command]` into `vp [command] --help`
if (args[0] === 'help' && args[1]) {
  args = [args[1], '--help', ...args.slice(2)];
  process.argv = process.argv.slice(0, 2).concat(args);
}

const command = args[0];
const cliDistDir = path.dirname(fileURLToPath(import.meta.url));
const vitePlusPackagePath = path.dirname(cliDistDir);

if (maybePrintCommandHelp(args)) {
  // Help is rendered by the local CLI so it matches the installed toolchain.
} else if (command === 'create') {
  await import('./create/bin.js');
} else if (command === 'migrate') {
  await import('./migration/bin.js');
} else if (command === 'config') {
  await import('./config/bin.js');
} else if (command === 'hooks') {
  await import('./hooks/bin.js');
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
      toolchainManifestPath: path.join(cliDistDir, 'toolchain.json'),
      vitePlusPackagePath,
      resolveUniversalViteConfig,
      explicitChdir,
      args: rustCliArgs,
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
