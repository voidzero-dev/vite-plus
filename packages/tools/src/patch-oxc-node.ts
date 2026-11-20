/**
 * Prepare the vendored oxc-node source for compilation into the Vite+ native binding.
 *
 * `sync-remote` clones oxc-node into `oxc-node/`, and CI checks it out at the hash in
 * `.upstream-versions.json`. The binding links it with `default-features = false`, so it
 * registers no NAPI exports of its own; `packages/cli/binding/src/oxc_node.rs` forwards
 * the ones the loader uses under an `oxcNode` namespace.
 *
 * This script aligns `oxc` and `oxc_sourcemap` with the workspace so that the source maps
 * from Oxc's code generator have the type oxc-node expects. It also adapts an AST predicate.
 * It is idempotent. Keep it free of package imports: CI runs it right after checkout,
 * before dependencies are installed.
 */

import { existsSync, readFileSync, realpathSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import { parseCargoOxcVersions, replaceCargoCrateVersion } from './cargo-toml.ts';

export const OXC_NODE_DIR = 'oxc-node';

function log(message: string) {
  console.log(`[patch-oxc-node] ${message}`);
}

export function syncOxcVersions(cargoToml: string, workspaceToml: string): string {
  const workspaceVersions = parseCargoOxcVersions(workspaceToml);
  const vendoredVersions = parseCargoOxcVersions(cargoToml);
  for (const crate of ['oxc', 'oxc_sourcemap']) {
    const version = workspaceVersions.get(crate);
    if (!version || !vendoredVersions.has(crate)) {
      const file = version ? 'oxc-node/Cargo.toml' : 'Cargo.toml';
      throw new Error(`[patch-oxc-node] Could not find the \`${crate}\` dependency in ${file}`);
    }
    cargoToml = replaceCargoCrateVersion(cargoToml, crate, version);
  }
  return cargoToml;
}

export function patchOxcNode(rootDir: string = process.cwd()): void {
  const workspaceToml = readFileSync(join(rootDir, 'Cargo.toml'), 'utf8');
  const cargoTomlPath = join(rootDir, OXC_NODE_DIR, 'Cargo.toml');
  const cargoToml = readFileSync(cargoTomlPath, 'utf8');
  const patched = syncOxcVersions(cargoToml, workspaceToml);
  if (patched !== cargoToml) {
    writeFileSync(cargoTomlPath, patched, 'utf8');
    log('✓ Cargo.toml: synchronized oxc and oxc_sourcemap');
  }
  const libPath = join(rootDir, OXC_NODE_DIR, 'src/lib.rs');
  const source = readFileSync(libPath, 'utf8');
  // Matching the variant works across the predicate rename in Oxc 0.156.
  const patchedSource = source.replace(
    'it.kind.is_await_using()',
    'matches!(it.kind, oxc::ast::ast::VariableDeclarationKind::AwaitUsing)',
  );
  if (patchedSource !== source) {
    writeFileSync(libPath, patchedSource, 'utf8');
    log('✓ lib.rs: compatible await-using detection');
  }
}

if (
  process.argv[1] &&
  existsSync(process.argv[1]) &&
  import.meta.filename === realpathSync(process.argv[1])
) {
  patchOxcNode();
}
