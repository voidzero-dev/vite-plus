/**
 * Prepare the vendored oxc-node source for compilation into the Vite+ native binding.
 *
 * `sync-remote` clones oxc-node into `oxc-node/`, and CI checks it out at the hash in
 * `.upstream-versions.json`. The binding links it with `default-features = false`, so it
 * registers no NAPI exports of its own; `packages/cli/binding/src/oxc_node.rs` forwards
 * the ones the loader uses under an `oxcNode` namespace.
 *
 * This script points the `oxc` dependency at the workspace version, so the binding links
 * one copy of oxc, and applies the loader-context patch for workers with different
 * tsconfigs. Both steps are idempotent. Keep this free of package imports: CI runs
 * it right after checkout, before dependencies are installed.
 */

import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync, realpathSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { parseCargoOxcVersions, replaceCargoCrateVersion } from './cargo-toml.ts';

export const OXC_NODE_DIR = 'oxc-node';
const LOADER_CONTEXT_PATCH = fileURLToPath(
  new URL('../patches/oxc-node-loader-context.patch', import.meta.url),
);

function log(message: string) {
  console.log(`[patch-oxc-node] ${message}`);
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
  if (!parseCargoOxcVersions(cargoToml).has('oxc')) {
    throw new Error('[patch-oxc-node] Could not find the `oxc` dependency in oxc-node/Cargo.toml');
  }
  return replaceCargoCrateVersion(cargoToml, 'oxc', version);
}

/** Apply a complete upstream patch, or verify that it was already applied. */
export function applySourcePatch(sourceDir: string, patchFile: string): void {
  const apply = (...args: string[]) =>
    spawnSync('git', ['apply', ...args, patchFile], { cwd: sourceDir, encoding: 'utf8' });
  const check = apply('--check');
  if (check.status !== 0) {
    if (apply('--reverse', '--check').status === 0) {
      return;
    }
    throw new Error(
      `[patch-oxc-node] Loader context patch does not match the upstream source: ${check.error?.message ?? check.stderr}`,
    );
  }
  const applied = apply();
  if (applied.status !== 0) {
    throw new Error(
      `[patch-oxc-node] Cannot apply loader context patch: ${applied.error?.message ?? applied.stderr}`,
    );
  }
  log('✓ Isolated loader contexts');
}

export function patchOxcNode(rootDir: string = process.cwd()): void {
  applySourcePatch(join(rootDir, OXC_NODE_DIR), LOADER_CONTEXT_PATCH);
  const version = oxcVersion(readFileSync(join(rootDir, 'Cargo.toml'), 'utf-8'), 'Cargo.toml');
  const cargoTomlPath = join(rootDir, OXC_NODE_DIR, 'Cargo.toml');
  const cargoToml = readFileSync(cargoTomlPath, 'utf-8');
  const patched = setOxcVersion(cargoToml, version);
  if (patched !== cargoToml) {
    writeFileSync(cargoTomlPath, patched, 'utf-8');
    log(`✓ Cargo.toml: oxc ${version}`);
  }
}

if (
  process.argv[1] &&
  existsSync(process.argv[1]) &&
  fileURLToPath(import.meta.url) === realpathSync(process.argv[1])
) {
  patchOxcNode();
}
