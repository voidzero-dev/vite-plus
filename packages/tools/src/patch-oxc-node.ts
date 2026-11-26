/**
 * Prepare the vendored oxc-node source for compilation into the Vite+ native binding.
 *
 * `sync-remote` clones oxc-node into `oxc-node/`, and CI checks it out at the hash in
 * `.upstream-versions.json`. The binding links it with `default-features = false`, so it
 * registers no NAPI exports of its own; `packages/cli/binding/src/oxc_node.rs` forwards
 * the ones the loader uses under an `oxcNode` namespace.
 *
 * This script points the `oxc` and `oxc_sourcemap` dependencies at the workspace
 * versions, so the binding links one compatible copy of each, and adapts AST APIs
 * for the workspace's Oxc version.
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

export function patchOxcNode(rootDir: string = process.cwd()): void {
  const rootCargoToml = readFileSync(join(rootDir, 'Cargo.toml'), 'utf8');
  const version = oxcVersion(rootCargoToml, 'Cargo.toml');
  const sourcemapVersion = parseCargoOxcVersions(rootCargoToml).get('oxc_sourcemap');
  if (!sourcemapVersion) {
    throw new Error('[patch-oxc-node] Could not find the `oxc_sourcemap` dependency in Cargo.toml');
  }
  const cargoTomlPath = join(rootDir, OXC_NODE_DIR, 'Cargo.toml');
  const cargoToml = readFileSync(cargoTomlPath, 'utf8');
  const patched = replaceCargoCrateVersion(
    setOxcVersion(cargoToml, version),
    'oxc_sourcemap',
    sourcemapVersion,
  );
  if (patched !== cargoToml) {
    writeFileSync(cargoTomlPath, patched, 'utf8');
    log(`✓ Cargo.toml: oxc ${version}, oxc_sourcemap ${sourcemapVersion}`);
  }
  const libPath = join(rootDir, OXC_NODE_DIR, 'src/lib.rs');
  const source = readFileSync(libPath, 'utf8');
  // Oxc 0.156 renamed this predicate. Matching the variant preserves
  // await-using detection on either side of the rename.
  let patchedSource = source.replace(
    'it.kind.is_await_using()',
    'matches!(it.kind, oxc::ast::ast::VariableDeclarationKind::AwaitUsing)',
  );
  if (patchedSource !== source) {
    log('✓ lib.rs: compatible await-using detection');
  }
  if (version === '0.156.0') {
    const assignmentTargetSource = patchedSource;
    patchedSource = patchedSource.replace(
      '                    maybe_default(property.value),\n                    &builder,',
      '                    maybe_default(property.value),\n                    property.computed,\n                    &builder,',
    );
    if (patchedSource !== assignmentTargetSource) {
      log('✓ lib.rs: compatible assignment-target properties');
    }
  }
  if (patchedSource !== source) {
    writeFileSync(libPath, patchedSource, 'utf8');
  }
}

if (
  process.argv[1] &&
  existsSync(process.argv[1]) &&
  import.meta.filename === realpathSync(process.argv[1])
) {
  patchOxcNode();
}
