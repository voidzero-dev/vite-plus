import assert from 'node:assert/strict';

for (const key of ['CUSTOM_PACK_PROBE', 'OTHER_PACK_PROBE', 'VITE_PACK_PROBE', 'TSDOWN_PACK_PROBE', 'CLI_PACK_PROBE']) {
  delete process.env[key];
}
const defaults = [undefined, undefined, 'default', 'compat', undefined];
const expected = {
  string: ['custom', undefined, undefined, undefined, undefined],
  array: ['custom', 'other', undefined, undefined, undefined],
  empty: [undefined, undefined, undefined, undefined, undefined],
  defaults,
};
if (process.argv[2] === 'no-config') {
  assert.deepEqual((await import('./dist/entry.mjs')).values, defaults);
} else {
  for (const [name, values] of Object.entries(expected)) {
    assert.deepEqual((await import(`./dist/${name}/entry.mjs`)).values,
      process.argv[2] === 'override' ? [undefined, undefined, undefined, undefined, 'cli'] : values,
      name);
  }
}
console.log('Environment prefix values and exclusions match.');
