/**
 * Prepare the vendored oxc-node source for compilation into the Vite+ native binding.
 *
 * `sync-remote` clones oxc-node into `oxc-node/`, and CI checks it out at the hash in
 * `.upstream-versions.json`. The binding links it with `default-features = false`, so it
 * registers no NAPI exports of its own; `packages/cli/binding/src/oxc_node.rs` forwards
 * the ones the loader uses under an `oxcNode` namespace.
 *
 * This script points the `oxc` dependency at the workspace version, so the binding links
 * one copy of oxc, and exposes the frozen loader initialization APIs to embedders
 * without registering upstream NAPI exports. It is idempotent. Keep it free of package
 * imports: CI runs it right after checkout, before dependencies are installed.
 */

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

/** Keep the loader's JS environment and diagnostic source without its NAPI exports. */
export function exposeLoaderInitialization(source: string): string {
  for (const name of ['freezeTsconfig', 'initializeLoader']) {
    const original = `#[cfg(feature = "napi")]\n#[napi(js_name = "${name}")]`;
    const replacement = `#[cfg_attr(feature = "napi", napi(js_name = "${name}"))]`;
    if (source.includes(original)) {
      source = source.replace(original, replacement);
    } else if (!source.includes(replacement)) {
      throw new Error(
        `[patch-oxc-node] Could not find the ${name} registration in oxc-node/src/lib.rs`,
      );
    }
  }
  return source;
}

/**
 * Track the oxc 0.156 rename of `VariableDeclarationKind::is_await` to
 * `is_await_using` (same `AwaitUsing` semantics). oxc-node v1.0.1 still calls the
 * old name and does not compile against the workspace oxc version without this.
 * Remove once the pinned oxc-node calls `is_await_using` itself.
 */
export function fixVariableDeclarationKindRename(source: string): string {
  const original = 'self.0 |= it.kind.is_await();';
  const replacement = 'self.0 |= it.kind.is_await_using();';
  if (source.includes(original)) {
    return source.replace(original, replacement);
  }
  if (!source.includes(replacement)) {
    throw new Error(
      '[patch-oxc-node] Could not find the `is_await` call in oxc-node/src/lib.rs. ' +
        'The vendored oxc-node may have absorbed the oxc rename; drop fixVariableDeclarationKindRename.',
    );
  }
  return source;
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
  const exposedSource = exposeLoaderInitialization(source);
  const patchedSource = fixVariableDeclarationKindRename(exposedSource);
  if (patchedSource !== source) {
    writeFileSync(libPath, patchedSource, 'utf-8');
    const changes = [
      exposedSource !== source && 'loader initialization available to embedders',
      patchedSource !== exposedSource && 'is_await_using rename',
    ].filter(Boolean);
    log(`✓ lib.rs: ${changes.join(', ')}`);
  }
}

if (
  process.argv[1] &&
  existsSync(process.argv[1]) &&
  fileURLToPath(import.meta.url) === realpathSync(process.argv[1])
) {
  patchOxcNode();
}
