import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const generator = path.resolve('tools/my-generator/bin/index.ts');
const directory = fs.mkdtempSync(path.join(process.cwd(), 'generator-assert-'));
const invoke = (args, interactive = '0') => {
  const result = spawnSync(process.execPath, [generator, ...args, '--skip-requests'], {
    cwd: directory,
    env: { ...process.env, VP_CREATE_INTERACTIVE: interactive },
    encoding: 'utf8',
    timeout: 15_000,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  assert.ifError(result.error);
  return { status: result.status, output: result.stdout + result.stderr };
};

try {
  for (const args of [[], ['--name', 'demo'], ['--directory', 'missing-name']]) {
    const result = invoke(args);
    assert.equal(result.status, 1, result.output);
    assert.doesNotMatch(result.output, /What will|unsettled top-level await/);
    assert.deepEqual(fs.readdirSync(directory), []);
  }
  fs.mkdirSync(path.join(directory, 'existing'));
  fs.writeFileSync(path.join(directory, 'existing', 'keep.txt'), 'keep');
  const existing = invoke(['--directory', 'existing', '--name', 'demo']);
  assert.equal(existing.status, 1, existing.output);
  assert.match(existing.output, /Directory already exists/);
  assert.equal(fs.readFileSync(path.join(directory, 'existing', 'keep.txt'), 'utf8'), 'keep');
  assert.deepEqual(fs.readdirSync(path.join(directory, 'existing')), ['keep.txt']);

  const result = invoke(['--directory', 'generated', '--name', '@demo/button', '--offline']);
  assert.equal(result.status, 0, result.output);
  const pkg = JSON.parse(
    fs.readFileSync(path.join(directory, 'generated', 'package.json'), 'utf8'),
  );
  assert.equal(pkg.name, '@demo/button');
  assert.match(
    fs.readFileSync(path.join(directory, 'generated', 'src', 'index.ts'), 'utf8'),
    /@demo\/button/,
  );
  const interactive = invoke(
    ['--directory', 'interactive', '--name', 'interactive', '--offline'],
    '1',
  );
  assert.equal(interactive.status, 0, interactive.output);
  assert.match(interactive.output, /Running with mode --setup/);
  console.log('Generator non-interactive assertions passed');
} finally {
  fs.rmSync(directory, { recursive: true, force: true });
}
