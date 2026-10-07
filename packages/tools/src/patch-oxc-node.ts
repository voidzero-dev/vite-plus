/**
 * Prepare the vendored oxc-node source for compilation into the Vite+ native binding.
 *
 * `sync-remote` clones oxc-node into `oxc-node/`, and CI checks it out at the hash in
 * `.upstream-versions.json`. `patches/oxc-node.patch` adapts the upstream tree; every
 * change is marked `Vite+:` in the source. In short, it:
 *
 * - builds only the rlib, without the napi build script, and with `ast_visit`;
 * - drops the `#[global_allocator]` and the tracing `module_init`, which belong to the
 *   host binding;
 * - exports everything under an `oxcNode` namespace (and renames `TransformTask`), so
 *   nothing collides with Rolldown's `transform` and `TransformTask`;
 * - reads the explicit tsconfig from `VP_SCRIPT_TSCONFIG` only;
 * - fixes or extends transforms: enum evaluation, tsconfig `jsx` values,
 *   `verbatimModuleSyntax`, class features and `using` lowered only when needed, a clear
 *   error for standard decorators, TypeScript under `node_modules`, `.cts` files with
 *   ESM syntax, and per-request export conditions.
 *
 * This script applies the patch, then points the `oxc` dependency at the workspace
 * version so the binding links one copy of oxc. It is idempotent and fails loudly when
 * the patch no longer applies. Keep it free of package imports: CI runs it right after
 * checkout, before dependencies are installed.
 */

import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync, realpathSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

export const OXC_NODE_DIR = 'oxc-node';

export const OXC_NODE_PATCH = fileURLToPath(new URL('../patches/oxc-node.patch', import.meta.url));

function log(message: string) {
  console.log(`[patch-oxc-node] ${message}`);
}

function gitApply(dir: string, args: string[]): void {
  execFileSync('git', ['apply', ...args, OXC_NODE_PATCH], { cwd: dir, stdio: 'pipe' });
}

function workspaceOxcVersion(rootDir: string): string {
  const cargoToml = readFileSync(join(rootDir, 'Cargo.toml'), 'utf-8');
  const match = /^oxc\s*=\s*\{\s*version\s*=\s*"([^"]+)"/m.exec(cargoToml);
  if (!match) {
    throw new Error('[patch-oxc-node] Could not find the workspace `oxc` version in Cargo.toml');
  }
  return match[1];
}

/** Point oxc-node's `oxc` dependency at `oxcVersion`. */
export function setOxcVersion(cargoToml: string, oxcVersion: string): string {
  const oxcDependency = /^(oxc\s*=\s*\{\s*version\s*=\s*")[^"]+(")/m;
  if (!oxcDependency.test(cargoToml)) {
    throw new Error('[patch-oxc-node] Could not find the `oxc` dependency in oxc-node/Cargo.toml');
  }
  return cargoToml.replace(oxcDependency, `$1${oxcVersion}$2`);
}

export function patchOxcNode(rootDir: string = process.cwd()): void {
  const oxcNodeDir = join(rootDir, OXC_NODE_DIR);

  let applied = false;
  try {
    gitApply(oxcNodeDir, ['--reverse', '--check']);
    applied = true;
  } catch {
    // Not applied yet.
  }
  if (applied) {
    log('patches/oxc-node.patch is already applied');
  } else {
    try {
      gitApply(oxcNodeDir, []);
    } catch (error) {
      const stderr = (error as { stderr?: Buffer }).stderr?.toString().trim();
      throw new Error(
        `[patch-oxc-node] patches/oxc-node.patch does not apply to ${OXC_NODE_DIR}/.\n` +
          `  The upstream code may have changed; rebase the patch onto the pinned commit.\n${stderr ?? ''}`,
        { cause: error },
      );
    }
    log('✓ Applied patches/oxc-node.patch');
  }

  const oxcVersion = workspaceOxcVersion(rootDir);
  const cargoTomlPath = join(oxcNodeDir, 'Cargo.toml');
  const cargoToml = readFileSync(cargoTomlPath, 'utf-8');
  const patched = setOxcVersion(cargoToml, oxcVersion);
  if (patched !== cargoToml) {
    writeFileSync(cargoTomlPath, patched, 'utf-8');
    log(`✓ Cargo.toml: oxc ${oxcVersion}`);
  }
}

/**
 * Write the vendored checkout's changes back to `patches/oxc-node.patch`, after editing
 * `oxc-node/` by hand (`pnpm tool patch-oxc-node --update`). The workspace oxc version is
 * left out, so the patch keeps applying when that version moves.
 */
export function updateOxcNodePatch(rootDir: string = process.cwd()): void {
  const oxcNodeDir = join(rootDir, OXC_NODE_DIR);
  const cargoTomlPath = join(oxcNodeDir, 'Cargo.toml');
  const cargoToml = readFileSync(cargoTomlPath, 'utf-8');
  const upstream = execFileSync('git', ['show', 'HEAD:Cargo.toml'], {
    cwd: oxcNodeDir,
    encoding: 'utf-8',
  });
  const upstreamVersion = /^oxc\s*=\s*\{\s*version\s*=\s*"([^"]+)"/m.exec(upstream)?.[1];
  if (!upstreamVersion) {
    throw new Error('[patch-oxc-node] Could not find the upstream `oxc` version');
  }
  writeFileSync(cargoTomlPath, setOxcVersion(cargoToml, upstreamVersion), 'utf-8');
  try {
    const patch = execFileSync('git', ['diff', '--', 'Cargo.toml', 'src'], {
      cwd: oxcNodeDir,
      encoding: 'utf-8',
    });
    writeFileSync(OXC_NODE_PATCH, patch, 'utf-8');
  } finally {
    writeFileSync(cargoTomlPath, cargoToml, 'utf-8');
  }
  log('✓ Updated patches/oxc-node.patch');
}

if (
  process.argv[1] &&
  existsSync(process.argv[1]) &&
  fileURLToPath(import.meta.url) === realpathSync(process.argv[1])
) {
  patchOxcNode();
}
