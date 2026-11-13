import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmodSync, mkdirSync, writeFileSync, symlinkSync } from 'node:fs';
import { delimiter, join } from 'node:path';

const scenario = process.argv[2];
const root = process.cwd();
const first = join(root, 'first-bin');
const nodeBin = join(root, 'node-bin');
const pmBin = join(root, 'pm-bin');
for (const dir of [first, nodeBin, pmBin]) {
  mkdirSync(dir, { recursive: true });
}
function executable(path, content) {
  writeFileSync(path, content);
  chmodSync(path, 0o755);
}
// Each later directory supplies a competing tool to expose accidental PATH promotion.
symlinkSync(process.execPath, join(nodeBin, 'node'));
executable(join(first, 'path-probe'), '#!/bin/sh\necho preferred\n');
executable(join(nodeBin, 'path-probe'), '#!/bin/sh\necho shadowed-by-node\n');
executable(join(pmBin, 'npm'), '#!/bin/sh\necho system-npm\n');
executable(join(pmBin, 'path-probe'), '#!/bin/sh\necho shadowed-by-pm\n');
executable(join(pmBin, 'node'), '#!/bin/sh\necho shadowed-by-pm\nexit 1\n');
const manager = '#!/bin/sh\nif [ "${1-}" = "--version" ]; then echo 10.18.0; else node probe.mjs; fi\n';
executable(join(pmBin, 'pnpm'), manager);
writeFileSync(
  'package.json',
  JSON.stringify({ name: 'system-first-path', private: true, packageManager: 'pnpm@10.18.0' }),
);
writeFileSync(
  'probe.mjs',
  `
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
assert.equal(execFileSync('path-probe', { encoding: 'utf8' }).trim(), 'preferred');
assert.match(execFileSync('node', ['--version'], { encoding: 'utf8' }), /^v/);
console.log('selected Node and original PATH precedence preserved');
`,
);
const env = {
  ...process.env,
  PATH: [first, nodeBin, pmBin, process.env.PATH].join(delimiter),
  // Start a fresh shell selection instead of inheriting the runner's Node selection.
  VP_PATH_INJECTED_TOOLS: '',
};
function run(args) {
  const result = spawnSync('vp', args, { env, encoding: 'utf8' });
  assert.equal(result.status, 0, `${args.join(' ')}\n${result.stdout}\n${result.stderr}`);
  return result.stdout;
}
run(['env', 'off', 'node']);
run(['env', 'off', 'pm']);
if (scenario === 'system-pm') {
  assert.match(run(['install']), /selected Node and original PATH precedence preserved/);
  run(['env', 'on', 'node']);
  assert.match(run(['install']), /selected Node and original PATH precedence preserved/);
  assert.match(
    run(['env', 'exec', '--node', process.versions.node, 'pnpm', 'install']),
    /selected Node and original PATH precedence preserved/,
  );
} else {
  const managed = join(process.env.VP_HOME, 'package_manager/pnpm/10.18.0/pnpm/bin');
  mkdirSync(managed, { recursive: true });
  for (const tool of ['pnpm', 'pnpx']) {
    executable(join(managed, tool), manager);
  }
  run(['env', 'on', 'pnpm']);
  if (scenario === 'shim') {
    const result = spawnSync(join(process.env.VP_HOME, 'bin/pnpm'), ['install'], {
      env,
      encoding: 'utf8',
    });
    assert.equal(result.status, 0, `${result.stdout}\n${result.stderr}`);
    assert.match(result.stdout, /selected Node and original PATH precedence preserved/);
  } else {
    assert.match(
      run(['env', 'exec', '--package-manager', 'pnpm@10.18.0', 'pnpm', 'install']),
      /selected Node and original PATH precedence preserved/,
    );
  }
}
console.log(`${scenario}: selected Node and original PATH precedence preserved`);
