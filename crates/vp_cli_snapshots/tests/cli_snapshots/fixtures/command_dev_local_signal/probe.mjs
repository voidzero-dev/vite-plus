import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { existsSync, mkdirSync, mkdtempSync, readdirSync, rmSync } from 'node:fs';
import { connect } from 'node:net';
import { tmpdir } from 'node:os';
import { delimiter, join, resolve } from 'node:path';
import { createInterface } from 'node:readline';

const workerMode = process.argv[2] === '--worker';
// A Node.js flag on the launcher keeps Vite in a child process.
const profileMode = process.argv[2] === '--cpu-prof';
const childMode = workerMode || profileMode;

async function listenerInfo(stdout, launchPid) {
  let listener;
  for await (const line of createInterface({ input: stdout })) {
    const match = /^listening (\d+) (\d+)$/.exec(line);
    if (match) {
      listener = { port: Number(match[1]), pid: Number(match[2]) };
      if (profileMode) {
        assert.notEqual(listener.pid, launchPid, 'Vite replaced the profiled launcher');
        return listener;
      }
      if (!workerMode) {
        assert.equal(listener.pid, launchPid);
        return listener;
      }
      assert.notEqual(listener.pid, launchPid, 'Vite replaced the Worker host');
    }
    if (listener && line === 'worker host alive') {
      return listener;
    }
  }
  assert.fail('Vite did not listen in the expected process');
}

function localLauncher() {
  const launcher = process.env.PATH.split(delimiter)
    .map((directory) => resolve(directory, 'vp'))
    .find(existsSync);
  assert.ok(launcher, 'expected the project-local vp launcher on PATH');
  return launcher;
}

const cache = mkdtempSync(join(tmpdir(), 'vp-dev-cache-'));
const cacheProof = join(cache, 'cache-before-vite');
const profileDir = join(cache, 'profiles');
const env = { ...process.env, NODE_COMPILE_CACHE: cache };
delete env.NODE_DISABLE_COMPILE_CACHE;
if (!childMode) {
  env.NODE_OPTIONS = ['--require=./check-launcher-cache.cjs', env.NODE_OPTIONS]
    .filter(Boolean)
    .join(' ');
  env.VP_PROBE_CACHE_PROOF = cacheProof;
}
const args = ['dev', '--host', '127.0.0.1', '--port', '0', '--base', '/probe/'];
let command = 'vp';
let commandArgs = args;
if (workerMode) {
  command = process.execPath;
  commandArgs = ['worker-host.mjs', ...args];
} else if (profileMode) {
  mkdirSync(profileDir);
  command = process.execPath;
  commandArgs = ['--cpu-prof', `--cpu-prof-dir=${profileDir}`, localLauncher(), ...args];
}
const child = spawn(command, commandArgs, { stdio: ['ignore', 'pipe', 'inherit'], env });

try {
  const { port, pid } = await listenerInfo(child.stdout, child.pid);
  const response = await fetch(`http://127.0.0.1:${port}/probe/`, {
    signal: AbortSignal.timeout(3_000),
  });
  assert.equal(response.status, 200);
  const html = await response.text();
  assert.match(html, /Project-local Vite/);
  assert.match(html, /src="\/probe\/@vite\/client"/);
  console.log('Vite served the project with the client under /probe/');
  if (workerMode) {
    console.log('The Worker host stayed alive while Vite ran as a child');
  } else if (profileMode) {
    console.log('The profiled launcher kept Vite as a child process');
  } else {
    assert.ok(
      existsSync(cacheProof),
      'Vite startup checked the launcher cache before loading its entry point',
    );
    console.log('vp saved its compile cache before starting Vite');
  }

  process.kill(pid, 'SIGTERM');
  const [code, signal] = await once(child, 'exit', { signal: AbortSignal.timeout(5_000) });
  const socket = connect(port, '127.0.0.1');
  try {
    await assert.rejects(once(socket, 'connect', { signal: AbortSignal.timeout(3_000) }), {
      code: 'ECONNREFUSED',
    });
  } finally {
    socket.destroy();
  }
  assert.deepEqual([code, signal], [143, null]);
  const target = childMode ? 'Vite child' : 'launch PID';
  console.log(`SIGTERM to the ${target} closed the port with exit code 143`);
  if (profileMode) {
    assert.ok(
      readdirSync(profileDir).some((name) => name.endsWith('.cpuprofile')),
      'the launcher wrote its CPU profile',
    );
    console.log('The launcher wrote its CPU profile');
  }
} catch (error) {
  console.error(error);
  process.exitCode = 1;
} finally {
  rmSync(cache, { recursive: true, force: true });
  if (process.exitCode === 1) {
    // The runner starts the probe in its own group. Stop its descendants
    // before an early failure can leave the runner waiting for their pipes.
    process.kill(-process.pid, 'SIGKILL');
  }
}
