// A cached pnpm entrypoint makes install failures and concurrency deterministic.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const http = require('node:http');
const path = require('node:path');
const writePayload = require('./payload.cjs');

assert.deepEqual(process.argv.slice(2), [
  'install',
  '--ignore-workspace',
  '--frozen-lockfile',
  '--lockfile=true',
  '--config.auto-install-peers=true',
  '--config.exclude-links-from-lockfile=false',
  '--config.dedupe-peers=false',
  '--config.inject-workspace-packages=false',
  '--registry',
  'http://127.0.0.1:9',
]);
assert.equal(process.env.CI, 'true');
assert.equal(process.env.npm_config_peers_suffix_max_length, '1000');
assert.equal(process.env.NPM_CONFIG_PEERS_SUFFIX_MAX_LENGTH, '1000');
assert.equal(
  fs.readFileSync('pnpm-lock.yaml', 'utf8'),
  fs.readFileSync('bin/vp-install-lock.yaml', 'utf8'),
);
assert.ok(fs.existsSync('.vp-js-deps-pending'));
assert.ok(!fs.existsSync('node_modules/.vp-install-complete'));
const npmrc = fs.readFileSync('.npmrc', 'utf8');
assert.match(npmrc, /registry=http:\/\/127\.0\.0\.1:9/);
assert.match(npmrc, /minimum-release-age=0/);
fs.appendFileSync('install-count', 'install\n');

writePayload(path.resolve('node_modules/vite-plus'), process.versions.node);

// Leave plausible package files behind, as a failed real install can do.
if (process.env.TEST_INSTALL_FAIL === '1') process.exit(42);
if (process.env.TEST_INSTALL_GATE) {
  http
    .get(process.env.TEST_INSTALL_GATE, (response) => response.resume())
    .on('error', (error) => {
      throw error;
    });
}
