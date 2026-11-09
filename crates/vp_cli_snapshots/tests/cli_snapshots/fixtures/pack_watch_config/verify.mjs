import assert from 'node:assert/strict';
import { fork } from 'node:child_process';
import { once } from 'node:events';
import { readFile, readdir, rename, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';

const require = createRequire(import.meta.url);
const cli = join(dirname(require.resolve('vite-plus/package.json')), 'dist/pack-bin.js');
const member = process.argv[2] === 'member';
const args = member ? ['--workspace', 'packages/*', '--concurrency', '1'] : [];
const configFile = member ? 'packages/a/tsdown.config.mjs' : 'vite.config.ts';
const outDir = member ? 'packages/a/dist' : 'dist';
const child = fork(cli, ['--watch', ...args], { silent: true });
const childExited = new AbortController();
child.once('exit', (code, signal) => {
  childExited.abort(new Error(`pack --watch exited before completing a build: ${signal ?? code}`));
});
let logs = '';
child.stdout.on('data', (data) => {
  logs += data;
});
child.stderr.on('data', (data) => {
  logs += data;
});

function waitForBuild() {
  return once(child, 'message', {
    signal: AbortSignal.any([childExited.signal, AbortSignal.timeout(20000)]),
  });
}

try {
  // The onSuccess IPC message synchronizes edits with completed builds on every platform.
  await waitForBuild();
  const output = join(outDir, (await readdir(outDir)).find((file) => /\.m?js$/.test(file)));
  const before = await readFile(output, 'utf8');
  assert.match(before, /\n/);

  const config = await readFile(configFile, 'utf8');
  // Replace the complete file so the watcher cannot reload a temporarily empty config.
  await writeFile(configFile + '.tmp', config.replace('minify: false', 'minify: true'));
  const rebuilt = waitForBuild();
  await Promise.all([rebuilt, rename(configFile + '.tmp', configFile)]);

  const after = await readFile(output, 'utf8');
  assert.notEqual(after, before, 'the rebuilt output must use the updated pack configuration');
  assert.ok(
    after.trim().split('\n').length < before.trim().split('\n').length,
    'the rebuilt output must be minified',
  );
  if (member) assert.match(logs, /--concurrency.*ignored/);
  console.log('pack --watch applied the updated minify option');
} catch (error) {
  console.error(logs);
  throw error;
} finally {
  if (child.exitCode === null && child.signalCode === null) {
    const exited = once(child, 'exit');
    child.kill();
    await exited;
  }
}
