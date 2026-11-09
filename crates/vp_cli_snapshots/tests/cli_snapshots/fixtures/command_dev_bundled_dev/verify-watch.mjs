import assert from 'node:assert/strict';
import { readFileSync, readdirSync, realpathSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join, sep } from 'node:path';
import { pathToFileURL } from 'node:url';

const projectRequire = createRequire(import.meta.url);
const corePackage = projectRequire.resolve('vite/package.json');
assert.ok(realpathSync(corePackage).startsWith(realpathSync('node_modules') + sep));
assert.equal(projectRequire('vite/package.json').name, '@voidzero-dev/vite-plus-core');

// Resolve the emitted initializer without depending on its chunk hash or export alias.
const sharedDir = join(dirname(corePackage), 'dist/rolldown/shared');
const bindingFiles = readdirSync(sharedDir).filter((file) => /^binding-[\w-]+\.mjs$/.test(file));
assert.equal(bindingFiles.length, 1);
const bindingUrl = pathToFileURL(join(sharedDir, bindingFiles[0]));
const bindingExport = readFileSync(bindingUrl, 'utf8').match(/\brequire_binding as (\w+)/)?.[1];
assert.ok(bindingExport, 'the emitted loader must export its binding initializer');
const binding = (await import(bindingUrl.href))[bindingExport]();

const originalDevEngine = binding.BindingDevEngine;
const nativeWatchOptions = [];
binding.BindingDevEngine = new Proxy(originalDevEngine, {
  construct(target, args) {
    nativeWatchOptions.push(args[1].watch);
    return Reflect.construct(target, args);
  },
});

const watch = {
  usePolling: true,
  interval: 1234,
  compareContentsForPolling: true,
  useDebounce: true,
  debounceDuration: 250,
  debounceTickRate: 50,
};

let server;
try {
  // Import after installing the observer, and still construct the real native engine.
  const { createServer } = await import('vite');
  server = await createServer({
    root: import.meta.dirname,
    configFile: false,
    logLevel: 'error',
    experimental: { bundledDev: true },
    server: { host: '127.0.0.1', port: 0, watch },
  });
  await server.listen();

  // Request the page so the test exercises a build, not just the static dev client.
  const response = await fetch(server.resolvedUrls.local[0], {
    signal: AbortSignal.timeout(10000),
  });
  assert.equal(response.status, 200);
  await response.text();
  assert.ok(nativeWatchOptions.length > 0, 'the page must start the native dev engine');
  console.log('HTTP status:', response.status);
  console.log('Requested watch options:', JSON.stringify(watch));
  console.log('Native watch options:', JSON.stringify(nativeWatchOptions));

  for (const nativeWatch of nativeWatchOptions) {
    assert.equal(nativeWatch.skipWrite, true);
    // Rolldown 1.2 receives these settings directly on watch.
    assert.deepEqual(
      {
        usePolling: nativeWatch.usePolling,
        pollInterval: nativeWatch.pollInterval,
        compareContentsForPolling: nativeWatch.compareContentsForPolling,
        useDebounce: nativeWatch.useDebounce,
        debounceDuration: nativeWatch.debounceDuration,
        debounceTickRate: nativeWatch.debounceTickRate,
      },
      {
        usePolling: true,
        pollInterval: 1234,
        compareContentsForPolling: true,
        useDebounce: true,
        debounceDuration: 250,
        debounceTickRate: 50,
      },
      'bundled dev must forward polling and debounce options to the native watcher',
    );
  }
} finally {
  try {
    await server?.close();
  } finally {
    binding.BindingDevEngine = originalDevEngine;
  }
}
