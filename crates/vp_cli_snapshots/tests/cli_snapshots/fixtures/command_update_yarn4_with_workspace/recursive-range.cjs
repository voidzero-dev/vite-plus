const assert = require('node:assert/strict');
const fs = require('node:fs');
const { resolve } = require('node:path');
// Widen every manifest to ^1.0.0 while keeping 1.0.0 installed and locked,
// so a recursive update has an outdated in-range resolution to refresh.
for (const dir of ['.', 'packages/app', 'packages/utils']) {
  const file = resolve(dir, 'package.json');
  const manifest = JSON.parse(fs.readFileSync(file, 'utf8'));
  manifest.dependencies.testnpm2 = '^1.0.0';
  manifest.dependencies['@test/scoped'] = 'npm:testnpm2@^1.0.0';
  fs.writeFileSync(file, JSON.stringify(manifest, null, 2) + '\n');
}
let lock = fs.readFileSync('yarn.lock', 'utf8');
assert.equal(lock.split('testnpm2: "npm:1.0.0"').length, 4);
assert.equal(lock.split('"@test/scoped": "npm:testnpm2@1.0.0"').length, 4);
const descriptor = '"@test/scoped@npm:testnpm2@1.0.0, testnpm2@npm:1.0.0":';
assert.ok(lock.includes(descriptor));
lock = lock.replaceAll('testnpm2: "npm:1.0.0"', 'testnpm2: "npm:^1.0.0"');
lock = lock.replaceAll('"@test/scoped": "npm:testnpm2@1.0.0"', '"@test/scoped": "npm:testnpm2@^1.0.0"');
lock = lock.replace(descriptor, '"@test/scoped@npm:testnpm2@^1.0.0, testnpm2@npm:^1.0.0":');
fs.writeFileSync('yarn.lock', lock);
