const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

function vp(args) {
  const result = spawnSync('vp', args, { encoding: 'utf8' });
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
  return result.stdout;
}

function readPackage() {
  return JSON.parse(fs.readFileSync('package.json', 'utf8'));
}

const mode = process.argv[2];
if (mode === 'linked') {
  assert.match(readPackage().resolutions?.['linked-lib'] ?? '', /^portal:/, 'link must add a portal resolution');
  console.log('The project has a portal resolution for linked-lib.');
} else if (mode === 'unlink') {
  assert.equal(readPackage().resolutions?.['linked-lib'], undefined, 'unlink must remove the portal resolution');
  console.log('The delegated Yarn removed the portal resolution.');
} else if (mode === 'patch') {
  const output = vp(['pm', 'patch', 'is-number', '--', '--json']);
  const { path: patchDir } = JSON.parse(output.trim());
  try {
    fs.writeFileSync(path.join(patchDir, 'vp-patch-probe.txt'), 'patched by vp\n');
    vp(['pm', 'patch-commit', patchDir]);
    const patches = fs.readdirSync('.yarn/patches');
    assert.equal(patches.length, 1);
    assert.match(fs.readFileSync(path.join('.yarn/patches', patches[0]), 'utf8'), /patched by vp/);
    assert.ok(Object.values(readPackage().resolutions).some((value) => value.startsWith('patch:')));
    console.log('The delegated Yarn saved and registered the patch.');
  } finally {
    fs.rmSync(patchDir, { recursive: true, force: true });
  }
} else {
  throw new Error(`Unknown verify mode: ${mode}`);
}
