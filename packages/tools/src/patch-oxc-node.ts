/**
 * Prepare the vendored oxc-node source for compilation into the Vite+ native binding.
 *
 * `sync-remote` clones oxc-node into `oxc-node/`, and CI checks it out at the hash in
 * `.upstream-versions.json`. The binding links it as an rlib next to `rolldown_binding`,
 * which needs these changes to the upstream tree:
 *
 * 1. Cargo.toml: build only the rlib without its napi build script, and follow the
 *    workspace's `oxc` version so the binding links one copy of oxc.
 * 2. lib.rs: drop the `#[global_allocator]`; `rolldown_binding` already declares one.
 * 3. lib.rs: drop the tracing `module_init`; it claims the global subscriber that
 *    `VP_LOG` relies on, and panics when another subscriber is already installed.
 * 4. lib.rs: read the explicit tsconfig from `VP_SCRIPT_TSCONFIG` only, so a stale
 *    `TS_NODE_PROJECT` or `OXC_TSCONFIG_PATH` cannot change `vpx` behavior.
 * 5. lib.rs: export everything under the `oxcNode` namespace and rename `TransformTask`,
 *    so the exports cannot collide with Rolldown's `transform` and `TransformTask`.
 * 6. lib.rs: build semantic data with `with_enum_eval(true)`, which enum lowering
 *    requires (upstream bug: string enums are miscompiled without it).
 *
 * Every patch is idempotent and fails loudly when the upstream code no longer matches.
 * Keep this file free of package imports: CI runs it right after checkout, before
 * dependencies are installed.
 */

import { existsSync, readFileSync, realpathSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

export const OXC_NODE_DIR = 'oxc-node';

const NAMESPACE = 'oxcNode';

function log(message: string) {
  console.log(`[patch-oxc-node] ${message}`);
}

function fail(file: string, what: string): never {
  throw new Error(
    `[patch-oxc-node] Patch failed in ${file}: ${what}\n` +
      '  The upstream code may have changed. Update packages/tools/src/patch-oxc-node.ts.',
  );
}

function replaceOnce(file: string, content: string, search: string, replacement: string): string {
  if (content.includes(replacement)) {
    return content;
  }
  if (!content.includes(search)) {
    fail(file, `could not find ${JSON.stringify(search)}`);
  }
  return content.replace(search, replacement);
}

function workspaceOxcVersion(rootDir: string): string {
  const cargoToml = readFileSync(join(rootDir, 'Cargo.toml'), 'utf-8');
  const match = /^oxc\s*=\s*\{\s*version\s*=\s*"([^"]+)"/m.exec(cargoToml);
  if (!match) {
    fail('Cargo.toml', 'could not find the workspace `oxc` version');
  }
  return match[1];
}

export function patchOxcNodeCargoToml(content: string, oxcVersion: string): string {
  const file = 'oxc-node/Cargo.toml';
  let patched = replaceOnce(
    file,
    content,
    'crate-type = ["cdylib", "rlib"]',
    'crate-type = ["rlib"]',
  );
  // `napi_build::setup()` only emits cdylib link args; the host binding's build
  // script already sets them up for the linked `.node` file.
  patched = replaceOnce(file, patched, 'publish = false\n', 'publish = false\nbuild = false\n');
  const oxcDependency = /^(oxc\s*=\s*\{\s*version\s*=\s*")[^"]+(")/m;
  if (!oxcDependency.test(patched)) {
    fail(file, 'could not find the `oxc` dependency');
  }
  patched = patched.replace(oxcDependency, `$1${oxcVersion}$2`);
  return patched;
}

export function patchOxcNodeLibRs(content: string): string {
  const file = 'oxc-node/src/lib.rs';
  let patched = content;

  patched = replaceOnce(
    file,
    patched,
    `#[cfg(all(
    not(target_arch = "x86"),
    not(target_arch = "arm"),
    not(target_family = "wasm"),
    not(all(target_os = "windows", target_arch = "aarch64"))
))]
#[global_allocator]
static ALLOC: mimalloc_safe::MiMalloc = mimalloc_safe::MiMalloc;
`,
    '// Vite+: the host binding declares the global allocator.\n',
  );

  patched = replaceOnce(
    file,
    patched,
    '#[cfg_attr(not(target_family = "wasm"), napi_derive::module_init)]\nfn init() {',
    '// Vite+: the host binding owns tracing (`VP_LOG`).\n' +
      '#[cfg_attr(not(target_family = "wasm"), allow(dead_code))]\nfn init() {',
  );

  patched = replaceOnce(
    file,
    patched,
    'non_empty_env("TS_NODE_PROJECT").or_else(|| non_empty_env("OXC_TSCONFIG_PATH"))',
    'non_empty_env("VP_SCRIPT_TSCONFIG")',
  );

  // Enum lowering reads member values that only `with_enum_eval(true)` computes;
  // without it, string enums lose their values (oxc#21667) and debug builds assert.
  patched = replaceOnce(
    file,
    patched,
    'SemanticBuilder::new().build(program)',
    'SemanticBuilder::new().with_enum_eval(true).build(program)',
  );

  // Top-level `#[napi]` items start at column 0; methods inside `impl` blocks are
  // indented and inherit their class's namespace. `impl Task` registers no export.
  const namespaced = `#[napi(namespace = "${NAMESPACE}")]`;
  const plainItems = /^#\[napi\]\n(?!impl Task )/gm;
  const objectItems = /^#\[napi\(object\)\]$/gm;
  if (!patched.includes(namespaced)) {
    if (!plainItems.test(patched) || !objectItems.test(patched)) {
      fail(file, 'could not find top-level `#[napi]` items');
    }
  }
  patched = patched.replace(plainItems, `${namespaced}\n`);
  patched = patched.replace(objectItems, `#[napi(object, namespace = "${NAMESPACE}")]`);

  patched = patched.replace(/\bTransformTask\b/g, 'OxcNodeTransformTask');

  return patched;
}

export function patchOxcNode(rootDir: string = process.cwd()): void {
  const oxcNodeDir = join(rootDir, OXC_NODE_DIR);
  const oxcVersion = workspaceOxcVersion(rootDir);

  const cargoTomlPath = join(oxcNodeDir, 'Cargo.toml');
  const cargoToml = readFileSync(cargoTomlPath, 'utf-8');
  const patchedCargoToml = patchOxcNodeCargoToml(cargoToml, oxcVersion);
  if (patchedCargoToml !== cargoToml) {
    writeFileSync(cargoTomlPath, patchedCargoToml, 'utf-8');
    log(`✓ Cargo.toml: rlib only, oxc ${oxcVersion}`);
  }

  const libRsPath = join(oxcNodeDir, 'src', 'lib.rs');
  const libRs = readFileSync(libRsPath, 'utf-8');
  const patchedLibRs = patchOxcNodeLibRs(libRs);
  if (patchedLibRs !== libRs) {
    writeFileSync(libRsPath, patchedLibRs, 'utf-8');
    log('✓ src/lib.rs: allocator, tracing, tsconfig env, and export namespace');
  }

  log('Done!');
}

if (
  process.argv[1] &&
  existsSync(process.argv[1]) &&
  fileURLToPath(import.meta.url) === realpathSync(process.argv[1])
) {
  patchOxcNode();
}
