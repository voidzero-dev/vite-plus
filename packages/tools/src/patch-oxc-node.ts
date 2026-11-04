/**
 * Prepare the vendored oxc-node source for compilation into the Vite+ native binding.
 *
 * `sync-remote` clones oxc-node into `oxc-node/`, and CI checks it out at the hash in
 * `.upstream-versions.json`. The binding links it with `default-features = false`, so it
 * registers no NAPI exports of its own; `packages/cli/binding/src/oxc_node.rs` forwards
 * the ones the loader uses under an `oxcNode` namespace.
 *
 * This script points the `oxc` dependency at the workspace version, so the binding links
 * one copy of oxc, and adapts an AST predicate for the workspace's Oxc version.
 * The warm-start prototype also applies a host transform-cache hook patch.
 * It is idempotent. Keep it free of package imports: CI runs it right after checkout,
 * before dependencies are installed.
 */

import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync, realpathSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { parseCargoOxcVersions, replaceCargoCrateVersion } from './cargo-toml.ts';

export const OXC_NODE_DIR = 'oxc-node';

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

/** Apply the prototype patch once, and fail clearly when the pinned upstream drifts. */
export function applyTransformCachePatch(rootDir: string, patch: string): boolean {
  const args = ['apply', '--directory=oxc-node', '--whitespace=error'];
  const run = (extra: string[]) =>
    spawnSync('git', [...args, ...extra], {
      cwd: rootDir,
      input: patch,
      encoding: 'utf8',
    });
  const check = run(['--check']);
  if (check.status === 0) {
    const applied = run([]);
    if (applied.status !== 0) {
      throw new Error(`[patch-oxc-node] Could not apply transform cache patch: ${applied.stderr}`);
    }
    return true;
  }
  if (run(['--reverse', '--check']).status === 0) {
    return false;
  }
  throw new Error(
    `[patch-oxc-node] Transform cache patch does not match the pinned upstream: ${check.stderr}`,
  );
}

export function patchOxcNode(rootDir: string = process.cwd()): void {
  const version = oxcVersion(readFileSync(join(rootDir, 'Cargo.toml'), 'utf-8'), 'Cargo.toml');
  const cargoTomlPath = join(rootDir, OXC_NODE_DIR, 'Cargo.toml');
  const cargoToml = readFileSync(cargoTomlPath, 'utf-8');
  const patched = setOxcVersion(cargoToml, version);
  if (patched !== cargoToml) {
    writeFileSync(cargoTomlPath, patched, 'utf-8');
    log(`✓ Cargo.toml: oxc ${version}`);
  }
  const libPath = join(rootDir, OXC_NODE_DIR, 'src/lib.rs');
  const source = readFileSync(libPath, 'utf-8');
  // Oxc 0.156 renamed this predicate; the workspace still uses 0.155.
  // Matching the variant preserves await-using detection with either version.
  const patchedSource = source.replace(
    'it.kind.is_await_using()',
    'matches!(it.kind, oxc::ast::ast::VariableDeclarationKind::AwaitUsing)',
  );
  if (patchedSource !== source) {
    writeFileSync(libPath, patchedSource, 'utf-8');
    log('✓ lib.rs: compatible await-using detection');
  }
  const cachePatch = readFileSync(
    new URL('../patches/oxc-node-transform-cache.patch', import.meta.url),
    'utf8',
  );
  if (applyTransformCachePatch(rootDir, cachePatch)) {
    log('✓ native transform-cache hook available to embedders (prototype)');
  }
}

if (
  process.argv[1] &&
  existsSync(process.argv[1]) &&
  fileURLToPath(import.meta.url) === realpathSync(process.argv[1])
) {
  patchOxcNode();
}
