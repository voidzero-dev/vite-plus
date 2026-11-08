import assert from 'node:assert/strict';

for (const file of process.argv.slice(2)) {
  const expected = file.startsWith('packages/a/') ? 11 : 55;
  assert.equal((await import(`./${file}`)).value, expected);
}
