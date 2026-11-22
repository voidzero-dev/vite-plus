const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

function vp(args) {
  const result = spawnSync('vp', args, { encoding: 'utf8' });
  assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
  return result.stdout;
}

if (process.argv[2] === 'unlink') {
  const pkg = JSON.parse(fs.readFileSync('package.json', 'utf8'));
  assert.equal(pkg.resolutions?.['linked-lib'], undefined, 'unlink must remove the portal resolution');
  console.log('The delegated Yarn removed the portal resolution.');
} else {
  const output = vp(['pm', 'patch', 'is-number', '--', '--json']);
  const { path: patchDir } = JSON.parse(output.trim());
  try {
    fs.writeFileSync(path.join(patchDir, 'vp-patch-probe.txt'), 'patched by vp\n');
    vp(['pm', 'patch-commit', patchDir]);
    const patches = fs.readdirSync('.yarn/patches');
    assert.equal(patches.length, 1);
    assert.match(fs.readFileSync(path.join('.yarn/patches', patches[0]), 'utf8'), /patched by vp/);
    const pkg = JSON.parse(fs.readFileSync('package.json', 'utf8'));
    assert.ok(Object.values(pkg.resolutions).some((value) => value.startsWith('patch:')));
    console.log('The delegated Yarn saved and registered the patch.');
  } finally {
    fs.rmSync(patchDir, { recursive: true, force: true });
  }
}
