import { describe, expect, test } from 'vitest';

import { patchOxcNodeCargoToml, patchOxcNodeLibRs } from '../patch-oxc-node.ts';

const upstreamCargoToml = `[package]
name = "oxc-node"
publish = false

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
oxc = { version = "0.153.0", features = [
  "codegen",
] }
oxc_resolver = { version = "11.24.3" }
`;

const upstreamLibRs = `use napi_derive::napi;

#[cfg(all(
    not(target_arch = "x86"),
    not(target_arch = "arm"),
    not(target_family = "wasm"),
    not(all(target_os = "windows", target_arch = "aarch64"))
))]
#[global_allocator]
static ALLOC: mimalloc_safe::MiMalloc = mimalloc_safe::MiMalloc;

const BUILTIN_MODULES: usize = 1;

#[cfg(not(target_family = "wasm"))]
#[napi]
pub fn init_tracing() {}

#[cfg_attr(not(target_family = "wasm"), napi_derive::module_init)]
fn init() {
    tracing_subscriber::registry().init();
}

#[napi]
pub struct Output {}

#[napi]
impl Output {
    #[napi]
    pub fn source(&self) -> String { String::new() }
}

pub struct TransformTask {}

#[napi]
impl Task for TransformTask {}

#[napi(object)]
pub struct LoadContext {}

fn transform_program() {
    let scoping = SemanticBuilder::new().build(program).semantic.into_scoping();
}

fn init_resolver() {
    let explicit_tsconfig =
        non_empty_env("TS_NODE_PROJECT").or_else(|| non_empty_env("OXC_TSCONFIG_PATH"));
}
`;

describe('patchOxcNodeCargoToml', () => {
  test('builds only the rlib against the workspace oxc version', () => {
    const patched = patchOxcNodeCargoToml(upstreamCargoToml, '0.152.0');
    expect(patched).toContain('crate-type = ["rlib"]');
    expect(patched).toContain('publish = false\nbuild = false\n');
    expect(patched).toContain('oxc = { version = "0.152.0", features = [');
    expect(patched).toContain('oxc_resolver = { version = "11.24.3" }');
    expect(patchOxcNodeCargoToml(patched, '0.152.0')).toBe(patched);
  });

  test('fails when upstream no longer matches', () => {
    expect(() => patchOxcNodeCargoToml('[package]\nname = "oxc-node"\n', '0.152.0')).toThrow(
      /Patch failed in oxc-node\/Cargo.toml/,
    );
  });
});

describe('patchOxcNodeLibRs', () => {
  const patched = patchOxcNodeLibRs(upstreamLibRs);

  test('leaves the allocator and tracing to the host binding', () => {
    expect(patched).not.toContain('#[global_allocator]');
    expect(patched).not.toContain('module_init');
    // The allocator's cfg must not attach to the next item.
    expect(patched).toContain(
      '// Vite+: the host binding declares the global allocator.\n\nconst BUILTIN_MODULES',
    );
  });

  test('reads the explicit tsconfig from VP_SCRIPT_TSCONFIG only', () => {
    expect(patched).toContain('non_empty_env("VP_SCRIPT_TSCONFIG");');
    expect(patched).not.toContain('TS_NODE_PROJECT');
  });

  test('evaluates enum members for the transformer', () => {
    expect(patched).toContain('SemanticBuilder::new().with_enum_eval(true).build(program)');
  });

  test('namespaces top-level exports and renames TransformTask', () => {
    expect(patched).toContain('#[napi(namespace = "oxcNode")]\npub fn init_tracing()');
    expect(patched).toContain('#[napi(namespace = "oxcNode")]\npub struct Output');
    expect(patched).toContain('#[napi(namespace = "oxcNode")]\nimpl Output');
    expect(patched).toContain('    #[napi]\n    pub fn source');
    expect(patched).toContain('#[napi]\nimpl Task for OxcNodeTransformTask');
    expect(patched).toContain('#[napi(object, namespace = "oxcNode")]\npub struct LoadContext');
    expect(patched).not.toMatch(/\bTransformTask\b/);
  });

  test('is idempotent', () => {
    expect(patchOxcNodeLibRs(patched)).toBe(patched);
  });

  test('fails when upstream no longer matches', () => {
    expect(() => patchOxcNodeLibRs(upstreamLibRs.replace('#[global_allocator]', ''))).toThrow(
      /Patch failed in oxc-node\/src\/lib.rs/,
    );
  });
});
