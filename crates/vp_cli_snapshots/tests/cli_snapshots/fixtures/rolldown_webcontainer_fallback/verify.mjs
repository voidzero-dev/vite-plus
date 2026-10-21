import assert from 'node:assert/strict';
import { readFileSync, readdirSync, realpathSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join, sep } from 'node:path';
import { pathToFileURL } from 'node:url';
import { createContext, SourceTextModule, SyntheticModule } from 'node:vm';

const mode = process.argv[2];
assert.ok(mode === 'missing' || mode === 'cached');
const projectRequire = createRequire(import.meta.url);
const corePackage = projectRequire.resolve('@voidzero-dev/vite-plus-core/package.json');
assert.ok(realpathSync(corePackage).startsWith(realpathSync('node_modules') + sep));
const coreRequire = createRequire(corePackage);
assert.throws(() => coreRequire.resolve('rolldown/package.json'), { code: 'MODULE_NOT_FOUND' });
const core = coreRequire('./package.json');
const version = core.bundledVersions.rolldown;
assert.notEqual(core.version, version, 'the fixture must distinguish core and Rolldown versions');

const sharedDir = join(dirname(corePackage), 'dist/rolldown/shared');
const files = readdirSync(sharedDir).filter((file) => /^binding-[\w-]+\.mjs$/.test(file));
assert.equal(files.length, 1);
const bindingUrl = pathToFileURL(join(sharedDir, files[0])).href;
const bindingRequire = createRequire(bindingUrl);
const source = readFileSync(new URL(bindingUrl), 'utf8');
const bindingExport = source.match(/\brequire_binding as (\w+)/)?.[1];
assert.ok(bindingExport, 'the emitted loader must export its binding initializer');

// Use WebContainer's POSIX paths even when the snapshot runs on Windows.
// No real /tmp files or package-manager processes are used by the loader.
const baseDir = `/tmp/rolldown-${version}`;
const bindingEntry = `${baseDir}/node_modules/@rolldown/binding-wasm32-wasi/rolldown-binding.wasi.cjs`;
const wasmBinding = {};
let cached = mode === 'cached';
let downloads = 0;
let cacheChecks = 0;
const fakeFs = {
  readFileSync(file, encoding) {
    assert.equal(file, corePackage, 'only core metadata may be read');
    return readFileSync(file, encoding);
  },
  existsSync(file) {
    assert.equal(file, bindingEntry);
    cacheChecks++;
    return cached;
  },
  rmSync(dir, options) {
    assert.equal(dir, baseDir);
    assert.deepEqual({ ...options }, { recursive: true, force: true });
    assert.equal(cached, false);
  },
  mkdirSync(dir, options) {
    assert.equal(dir, baseDir);
    assert.deepEqual({ ...options }, { recursive: true });
  },
};
const fakeChildProcess = {
  execFileSync(command, args, options) {
    assert.equal(command, 'pnpm');
    assert.deepEqual(Array.from(args), ['i', `@rolldown/binding-wasm32-wasi@${version}`]);
    assert.equal(options.cwd, baseDir);
    assert.equal(options.stdio, 'inherit');
    assert.equal(cached, false);
    downloads++;
    cached = true;
    console.log('Installer selected the bundled Rolldown version');
  },
};

function missingModule(specifier) {
  return Object.assign(new Error(`Cannot find module '${specifier}'`), {
    code: 'MODULE_NOT_FOUND',
  });
}

function unavailableBinding(specifier) {
  return (
    specifier.endsWith('.node') ||
    specifier.includes('binding-wasm32-wasi') ||
    specifier.endsWith('.wasi.cjs')
  );
}

const isolatedRequire = Object.assign(
  (specifier) => {
    if (specifier === 'fs' || specifier === 'node:fs') return fakeFs;
    if (specifier === 'child_process' || specifier === 'node:child_process') return fakeChildProcess;
    if (specifier === bindingEntry) {
      assert.equal(cached, true, 'the WASM binding must be cached or installed first');
      return wasmBinding;
    }
    if (unavailableBinding(specifier)) throw missingModule(specifier);
    return bindingRequire(specifier);
  },
  {
    resolve(specifier) {
      if (unavailableBinding(specifier)) throw missingModule(specifier);
      // Keep real Node resolution for metadata: the old rolldown/package.json
      // lookup must fail in this packed installation, not in a mocked resolver.
      return bindingRequire.resolve(specifier);
    },
  },
);

const context = createContext({
  console: {
    log(message) {
      // The installer and cache assertions above check the exact version.
      console.log(message.replaceAll(`@${version}`, '@<bundled-rolldown-version>'));
    },
  },
  process: {
    env: { NAPI_RS_NATIVE_LIBRARY_PATH: '/unavailable-native-binding.node' },
    versions: { webcontainer: '1' },
    platform: 'linux',
    arch: 'x64',
  },
});
const loader = new SourceTextModule(source, {
  context,
  identifier: bindingUrl,
  initializeImportMeta(meta) {
    meta.url = bindingUrl;
  },
});
await loader.link((specifier) => {
  assert.equal(specifier, 'node:module');
  return new SyntheticModule(
    ['createRequire'],
    function () {
      this.setExport('createRequire', (url) => {
        assert.equal(url, bindingUrl);
        return isolatedRequire;
      });
    },
    { context },
  );
});
await loader.evaluate();

console.log('Standalone rolldown: missing');
console.log(`WASM cache: ${mode}`);
const binding = loader.namespace[bindingExport]();
assert.equal(binding, wasmBinding);
assert.equal(binding.__napiBindingTarget, 'wasm32-wasi');
assert.equal(cacheChecks, 1);
assert.equal(downloads, mode === 'missing' ? 1 : 0);
console.log('Loaded the WebContainer WASM binding');
console.log(`Downloads: ${downloads}`);
