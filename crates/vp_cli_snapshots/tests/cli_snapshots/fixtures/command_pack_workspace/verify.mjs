import assert from 'node:assert/strict';
import { existsSync, rmSync } from 'node:fs';

assert.equal(existsSync('dist/index.mjs'), false, 'unexpected root build');
const expected = process.argv.slice(2);
for (const [name, value] of Object.entries({ a: 11, b: 55 })) {
  const file = `packages/${name}/dist/index.mjs`;
  assert.equal(existsSync(file), expected.includes(name), file);
  if (expected.includes(name)) {
    assert.equal((await import(`./${file}`)).value, value);
    rmSync(`packages/${name}/dist`, { recursive: true });
  }
}
