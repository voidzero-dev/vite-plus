import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';

import cliPkg from 'vite-plus/package.json' with { type: 'json' };
import { versions } from 'vite-plus/versions';

const require = createRequire(import.meta.url);
const cliRequire = createRequire(require.resolve('vite-plus/package.json'));
const before = JSON.parse(readFileSync('package.before.json', 'utf8')).devDependencies;
const after = JSON.parse(readFileSync('package.json', 'utf8')).devDependencies;
const workspace = readFileSync('pnpm-workspace.yaml', 'utf8');

assert(workspace.includes(`  vite: npm:@voidzero-dev/vite-plus-core@${cliPkg.version}\n`));
assert(workspace.includes(`  vite-plus: ${cliPkg.version}\n`));
assert(workspace.includes(`  vitest: ${versions.vitest}\n`));
assert(workspace.includes(`  "@vitest/browser-playwright": ${versions.vitest}\n`));
assert.equal(after['vite-plus'], 'catalog:');
assert.equal(after['@vitest/browser'], undefined);

for (const name of ['vite', 'vitest', '@vitest/browser', '@vitest/browser-playwright']) {
  const previous = before[name].replace(/^\^/, '');
  const bundled = versions[name === 'vite' ? 'vite' : 'vitest'];
  // All versions in this fixture are stable releases. Keep the fixture newer
  // than the bundle so a future version bump cannot hide a lost reproduction.
  assert(bundled.localeCompare(previous, 'en', { numeric: true }) < 0, `${name} must downgrade`);
  if (name === 'vite') {
    assert.equal(require(`${name}/package.json`).name, '@voidzero-dev/vite-plus-core');
    assert.equal(require(`${name}/package.json`).version, cliPkg.version);
  } else {
    const resolve = name === '@vitest/browser' ? cliRequire : require;
    assert.equal(resolve(`${name}/package.json`).version, bundled);
  }
  if (name !== '@vitest/browser') {
    assert.equal(after[name], 'catalog:');
  }
  console.log(`${name}: downgraded to the running CLI's bundled version`);
}
