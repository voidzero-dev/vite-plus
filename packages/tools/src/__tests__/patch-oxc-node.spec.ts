import { describe, expect, test } from 'vitest';

import { exposeLoaderInitialization, setOxcVersion } from '../patch-oxc-node.ts';

describe('setOxcVersion', () => {
  const cargoToml = `[dependencies]
oxc = { version = "0.153.0", features = [
  "codegen",
] }
oxc_resolver = { version = "11.24.3" }
`;

  test('follows the workspace oxc version and leaves other crates alone', () => {
    const patched = setOxcVersion(cargoToml, '0.152.0');
    expect(patched).toContain('oxc = { version = "0.152.0", features = [');
    expect(patched).toContain('oxc_resolver = { version = "11.24.3" }');
    expect(setOxcVersion(patched, '0.152.0')).toBe(patched);
  });

  test('fails without an oxc dependency', () => {
    expect(() => setOxcVersion('[dependencies]\n', '0.152.0')).toThrow(/`oxc` dependency/);
  });
});

describe('exposeLoaderInitialization', () => {
  const source = `#[cfg(feature = "napi")]
#[napi(js_name = "initializeLoader")]
pub fn initialize_loader_js(path: Option<String>, source: Option<String>) -> Result<()> {}

#[cfg(feature = "napi")]
#[napi(js_name = "freezeTsconfig")]
pub fn freeze_tsconfig_js(env: Env, cwd: String) -> Result<FrozenTsconfig> {}

#[cfg(feature = "napi")]
#[napi(js_name = "transformAsync")]
pub fn transform_async_js() {}
`;

  test('exposes initialization while keeping NAPI registration conditional', () => {
    const patched = exposeLoaderInitialization(source);
    expect(patched).toContain('#[cfg_attr(feature = "napi", napi(js_name = "initializeLoader"))]');
    expect(patched).toContain('#[cfg_attr(feature = "napi", napi(js_name = "freezeTsconfig"))]');
    expect(patched).toContain('#[cfg(feature = "napi")]\n#[napi(js_name = "transformAsync")]');
    expect(exposeLoaderInitialization(patched)).toBe(patched);
  });

  test('fails when upstream changes a required initialization API', () => {
    expect(() =>
      exposeLoaderInitialization(source.replace('freezeTsconfig', 'freezeConfig')),
    ).toThrow(/freezeTsconfig registration/);
  });
});
