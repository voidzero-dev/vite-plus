import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';

const registry = process.env.NPM_CONFIG_REGISTRY;
assert.ok(registry, 'Fixture overlay registry was not started');
assert.equal(new URL(registry).hostname, '127.0.0.1');

const metadataResponse = await fetch(`${registry}/@vp-smoke/hello`);
assert.equal(metadataResponse.status, 200);
const metadata = await metadataResponse.json();
assert.equal(metadata.name, '@vp-smoke/hello');
assert.equal(metadata['dist-tags'].latest, '1.0.0');

const { dist } = metadata.versions['1.0.0'];
assert.equal(new URL(dist.tarball).origin, new URL(registry).origin);
const tarballResponse = await fetch(dist.tarball);
assert.equal(tarballResponse.status, 200);
const tarball = Buffer.from(await tarballResponse.arrayBuffer());
assert.equal(`sha512-${createHash('sha512').update(tarball).digest('base64')}`, dist.integrity);
console.log('Fixture metadata and tarball integrity verified');
