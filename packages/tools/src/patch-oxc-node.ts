/**
 * Prepare the vendored oxc-node source for compilation into the Vite+ native binding.
 *
 * `sync-remote` clones oxc-node into `oxc-node/`, and CI checks it out at the hash in
 * `.upstream-versions.json`. `patches/oxc-node.patch` adapts the upstream tree: it builds
 * only the rlib, without the napi build script; exports everything under an `oxcNode`
 * namespace (renaming `TransformTask`), so nothing collides with Rolldown's exports; and
 * transforms TypeScript under `node_modules`, marked `Vite+:` in the source.
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

import { parseCargoOxcVersions, replaceCargoCrateVersion } from './cargo-toml.ts';

export const OXC_NODE_DIR = 'oxc-node';

export const OXC_NODE_PATCH = fileURLToPath(new URL('../patches/oxc-node.patch', import.meta.url));

function log(message: string) {
  console.log(`[patch-oxc-node] ${message}`);
}

function gitApply(dir: string, args: string[]): void {
  execFileSync('git', ['apply', ...args, OXC_NODE_PATCH], { cwd: dir, stdio: 'pipe' });
}

function oxcVersion(cargoToml: string, file: string): string {
  const version = parseCargoOxcVersions(cargoToml).get('oxc');
  if (!version) {
    throw new Error(`[patch-oxc-node] Could not find the \`oxc\` dependency in ${file}`);
  }
  return version;
}

/** Point oxc-node's `oxc` dependency at `version`. */
export function setOxcVersion(cargoToml: string, version: string): string {
  oxcVersion(cargoToml, 'oxc-node/Cargo.toml');
  return replaceCargoCrateVersion(cargoToml, 'oxc', version);
}

export function patchOxcNode(rootDir: string = process.cwd()): void {
  const oxcNodeDir = join(rootDir, OXC_NODE_DIR);

  // `git apply` is atomic, so a failed apply leaves the tree untouched; only then check
  // whether the patch is already applied.
  try {
    gitApply(oxcNodeDir, []);
    log('✓ Applied patches/oxc-node.patch');
  } catch (error) {
    try {
      gitApply(oxcNodeDir, ['--reverse', '--check']);
    } catch {
      const stderr = (error as { stderr?: Buffer }).stderr?.toString().trim();
      throw new Error(
        `[patch-oxc-node] patches/oxc-node.patch does not apply to ${OXC_NODE_DIR}/.\n` +
          `  The upstream code may have changed; rebase the patch onto the pinned commit.\n${stderr ?? ''}`,
        { cause: error },
      );
    }
    log('patches/oxc-node.patch is already applied');
  }

  const version = oxcVersion(readFileSync(join(rootDir, 'Cargo.toml'), 'utf-8'), 'Cargo.toml');
  const cargoTomlPath = join(oxcNodeDir, 'Cargo.toml');
  const cargoToml = readFileSync(cargoTomlPath, 'utf-8');
  const patched = setOxcVersion(cargoToml, version);
  if (patched !== cargoToml) {
    writeFileSync(cargoTomlPath, patched, 'utf-8');
    log(`✓ Cargo.toml: oxc ${version}`);
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
  const upstreamVersion = oxcVersion(upstream, 'the upstream oxc-node/Cargo.toml');
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
