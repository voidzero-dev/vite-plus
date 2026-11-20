import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const cliRequire = createRequire(require.resolve('vite-plus/package.json'));
const vitestRequire = createRequire(cliRequire.resolve('vitest/package.json'));
const cliCore = cliRequire('vite/package.json');
const cliAlias = cliRequire('vite-plus/package.json').dependencies.vite;
const project = require('./package.json');

assert.equal(cliCore.name, '@voidzero-dev/vite-plus-core');
assert.equal(cliAlias, `npm:${cliCore.name}@${cliCore.version}`);

const projectVite = require.resolve('vite/package.json');
const cliVite = cliRequire.resolve('vite/package.json');
assert.equal(projectVite, vitestRequire.resolve('vite/package.json'));

const mode = process.argv[2];
if (mode === 'migrated') {
  assert.equal(project.overrides.vite, cliAlias);
} else {
  assert.equal(project.overrides, undefined);
}
if (!mode || mode === 'migrated') {
  assert.equal(project.devDependencies.vite, cliAlias);
  assert.equal(projectVite, cliVite);
  console.log(
    mode === 'migrated'
      ? 'npm shares the migrated core alias between the project, CLI, and Vitest'
      : 'npm shares the exact core alias between the project, CLI, and Vitest without overrides',
  );
} else {
  if (mode === 'cli-only') {
    assert.deepEqual(Object.keys(project.devDependencies), ['vite-plus']);
  } else {
    assert.equal(mode, 'tagged');
    assert.equal(project.devDependencies.vite, 'npm:@voidzero-dev/vite-plus-core@latest');
  }
  // The runner exposes the checkout core and its dev dependency on semver.
  const checkoutRequire = createRequire(
    require.resolve('@voidzero-dev/vite-plus-core/package.json'),
  );
  const satisfies = checkoutRequire('semver/functions/satisfies');
  const range = vitestRequire('vitest/package.json').peerDependencies.vite;
  const shared = projectVite === cliVite;
  if (!shared) {
    assert.ok(!satisfies(cliCore.version, range), 'npm must share a peer-compatible core');
    const upstream = require('vite/package.json');
    assert.equal(upstream.name, 'vite');
    assert.ok(satisfies(upstream.version, range));
  }
  if (mode === 'tagged') {
    const result = spawnSync('vp', ['pack', 'entry.js'], { encoding: 'utf8', timeout: 30_000 });
    assert.ifError(result.error);
    const output = result.stdout + result.stderr;
    assert.equal(result.status, shared ? 0 : 1, output);
    if (!shared) {
      assert.match(output, /Expected @voidzero-dev\/vite-plus-core@.*but found vite@/);
      assert.match(output, /Run `vp migrate` to align the Vite alias, then run `vp install`\./);
    }
  }
  console.log('Checked npm resolution against the Vite peer range declared by Vitest');
}
