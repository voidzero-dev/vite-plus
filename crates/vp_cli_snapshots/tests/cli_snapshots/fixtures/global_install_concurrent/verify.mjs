import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const home = process.env.VP_HOME;
const executable = (name) => join(home, 'bin', name + (process.platform === 'win32' ? '.exe' : ''));
const vp = executable('vp');
const packages = join(home, 'packages');
const name = 'global-lock-demo';
const barrier = resolve('barrier');
await mkdir(barrier);

async function makePackage(directory, packageName, version) {
  await mkdir(directory);
  await writeFile(join(directory, 'package.json'), JSON.stringify({
    name: packageName,
    version,
    bin: { [name]: 'cli.js' },
    scripts: { postinstall: 'node ../prepare.mjs' },
  }));
  await writeFile(join(directory, 'cli.js'), `#!/usr/bin/env node\nconsole.log('${version}');\n`);
}

function run(binary, args, env = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(binary, args, {
      env: { ...process.env, ...env },
      stdio: ['ignore', 'pipe', 'pipe'],
      timeout: 30_000,
      killSignal: 'SIGKILL',
    });
    let output = '';
    child.stdout.on('data', (chunk) => { output += chunk; });
    child.stderr.on('data', (chunk) => { output += chunk; });
    child.on('error', reject);
    child.on('close', (code) => {
      if (code === 0) resolve(output.trim());
      else reject(new Error(`${args.join(' ')} exited ${code}: ${output}`));
    });
  });
}

await makePackage('first', name, '1.0.0');
await makePackage('second', name, '2.0.0');
await Promise.all(['first', 'second'].map((participant) => run(vp, ['install', '-g', `./${participant}`], {
  VP_TEST_BARRIER: barrier,
  VP_TEST_PARTICIPANT: participant,
})));

const metadata = JSON.parse(await readFile(join(packages, `${name}.json`), 'utf8'));
const installations = (await readdir(join(packages, name), { withFileTypes: true }))
  .filter((entry) => entry.isDirectory()).map((entry) => entry.name);
assert.deepEqual(installations, [metadata.installId]);
assert.equal(await run(executable(name), []), metadata.version);
console.log('Concurrent preparation completed; the active installation and command survived.');

await makePackage('replacement', 'global-lock-replacement', '3.0.0');
await run(vp, ['install', '-g', '--force', './replacement']);
assert.equal(await run(executable(name), []), '3.0.0');
await assert.rejects(readFile(join(packages, `${name}.json`)), { code: 'ENOENT' });
await run(vp, ['uninstall', '-g', 'global-lock-replacement']);
await assert.rejects(readFile(join(packages, 'global-lock-replacement.json')), { code: 'ENOENT' });
console.log('Forced replacement and removal completed without deadlock.');
