const assert = require('node:assert/strict');
const { execFileSync, spawnSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const projectDir = path.resolve('app');
const pkg = JSON.parse(fs.readFileSync(path.join(projectDir, 'package.json'), 'utf8'));
assert.ok(pkg.devDependencies?.['vite-plus'], 'Vite+ integration must finish');
assert.ok(fs.existsSync(path.join(projectDir, 'node_modules', 'vite-plus', 'package.json')));
assert.ok(fs.existsSync(path.join(projectDir, '.git')), 'Vite+ must initialize Git');
assert.ok(fs.existsSync(path.join(projectDir, 'src', 'index.ts')));
assert.equal(
  execFileSync('git', ['config', 'user.name'], { cwd: projectDir, encoding: 'utf8' }).trim(),
  '',
  'the test must run without a Git author name',
);
const head = spawnSync('git', ['rev-parse', '--verify', 'HEAD'], {
  cwd: projectDir,
  stdio: 'ignore',
});
assert.ifError(head.error);
assert.notEqual(head.status, 0, 'project creation must not require an initial commit');
console.log('Vite+ integrated and initialized Git without an initial commit.');
