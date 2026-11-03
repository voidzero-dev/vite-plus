import assert from 'node:assert/strict';
import { createRequire } from 'node:module';

const instances = { cli: new Set(), vite: new Set(), vitest: new Set() };
for (const directory of ['.', 'packages/utils', 'packages/app']) {
  const project = createRequire(new URL(`${directory}/package.json`, import.meta.url));
  const manifest = project('./package.json');
  const cliPath = project.resolve('vite-plus/package.json');
  const cli = createRequire(cliPath);
  const vitestPath = cli.resolve('vitest/package.json');
  const vitest = createRequire(vitestPath);
  const vitePath = cli.resolve('vite/package.json');

  // #1932: Vitest must not bind to an auto-installed upstream Vite, or to a
  // separate core instance, when its consumer has no direct Vite dependency.
  assert.equal(vitest.resolve('vite/package.json'), vitePath);
  assert.equal(cli('vite/package.json').name, '@voidzero-dev/vite-plus-core');
  if (directory === 'packages/app') {
    assert.ok(manifest.devDependencies.vite, 'preserve the existing Vite dependency');
    assert.equal(project.resolve('vite/package.json'), vitePath);
  } else {
    assert.equal(manifest.devDependencies.vite, undefined, `${directory} needs no direct Vite`);
  }
  instances.cli.add(cliPath);
  instances.vite.add(vitePath);
  instances.vitest.add(vitestPath);
}
for (const [name, paths] of Object.entries(instances)) {
  assert.equal(paths.size, 1, `${name} must have one instance across the workspace`);
}
console.log('Root and utils have no direct Vite; app keeps its declared Vite');
console.log('All packages share one Vite+, bundled Vite, and Vitest instance');
