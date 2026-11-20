import assert from 'node:assert/strict';
import { execFileSync, spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtempSync, readdirSync, rmSync } from 'node:fs';
import { connect } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createInterface } from 'node:readline';
import { text } from 'node:stream/consumers';

const workerMode = process.argv[2] === '--worker';
const hangupMode = process.argv[2] === '--hangup';

function stty(...args) {
  return execFileSync('stty', args, { encoding: 'utf8', stdio: ['inherit', 'pipe', 'inherit'] }).trim();
}

async function listenerInfo(stdout, launchPid) {
  let listener;
  for await (const line of createInterface({ input: stdout })) {
    const match = /^listening (\d+) (\d+)$/.exec(line);
    if (match) {
      listener = { port: Number(match[1]), pid: Number(match[2]) };
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

const cache = mkdtempSync(join(tmpdir(), 'vp-dev-cache-'));
const env = { ...process.env, NODE_COMPILE_CACHE: cache };
delete env.NODE_DISABLE_COMPILE_CACHE;
if (hangupMode) {
  env.VP_PROBE_RAW = '1';
}
const initialTerminal = hangupMode ? stty('-g') : undefined;
const args = ['dev', '--host', '127.0.0.1', '--port', '0', '--base', '/probe/'];
if (hangupMode) {
  args.push('--debug', 'config');
}
const child = spawn(
  workerMode ? process.execPath : 'vp',
  workerMode ? ['worker-host.mjs', ...args] : args,
  { stdio: [hangupMode ? 'inherit' : 'ignore', 'pipe', hangupMode ? 'pipe' : 'inherit'], env },
);
const exited = once(child, 'exit');
const debugOutput = hangupMode ? text(child.stderr) : undefined;

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
  } else if (hangupMode) {
    assert.notEqual(stty('-g'), initialTerminal, 'the Vite plugin switched the terminal to raw mode');
  } else {
    assert.ok(
      readdirSync(cache, { recursive: true, withFileTypes: true }).some((entry) => entry.isFile()),
      'vp saves its compile cache before starting Vite',
    );
    console.log('vp saved its compile cache before starting Vite');
  }

  process.kill(pid, hangupMode ? 'SIGHUP' : 'SIGTERM');
  const [code, signal] = await exited;
  const socket = connect(port, '127.0.0.1');
  try {
    await assert.rejects(once(socket, 'connect', { signal: AbortSignal.timeout(3_000) }), {
      code: 'ECONNREFUSED',
    });
  } finally {
    socket.destroy();
  }
  if (hangupMode) {
    assert.match(await debugOutput, /vite:config/);
    console.log('Vite printed its config debug logs');
    assert.deepEqual([code, signal], [null, 'SIGHUP']);
    assert.equal(stty('-g'), initialTerminal, 'SIGHUP restored the terminal mode');
    console.log('SIGHUP to the launch PID closed the port and restored the terminal');
  } else {
    assert.deepEqual([code, signal], [143, null]);
    const target = workerMode ? 'Vite child' : 'launch PID';
    console.log(`SIGTERM to the ${target} closed the port with exit code 143`);
  }
} catch (error) {
  console.error(error);
  process.exitCode = 1;
} finally {
  if (initialTerminal !== undefined) {
    stty(initialTerminal);
  }
  rmSync(cache, { recursive: true, force: true });
  if (process.exitCode === 1) {
    // The runner starts the probe in its own group. Stop its descendants
    // before an early failure can leave the runner waiting for their pipes.
    process.kill(-process.pid, 'SIGKILL');
  }
}
