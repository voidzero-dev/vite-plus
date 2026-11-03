import assert from 'node:assert/strict';
import { registerHooks } from 'node:module';

// The runner exposes its checkout in an ancestor node_modules directory.
// Reject that fallback: consumers do not install a plugin's devDependencies.
registerHooks({
  resolve(specifier, context, nextResolve) {
    assert(
      specifier !== 'vite-plus' && !specifier.startsWith('vite-plus/'),
      'The published plugin must not depend on vite-plus',
    );
    return nextResolve(specifier, context);
  },
});

const { default: plugin } = await import('oxlint-plugin-optional-example');
assert.equal(plugin.meta.name, 'optional');
assert.equal(typeof plugin.rules['no-foo'].create, 'function');
console.log('The published plugin loads with its optional API and without vite-plus.');
