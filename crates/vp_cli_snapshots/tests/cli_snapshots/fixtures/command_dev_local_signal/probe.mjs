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
const terminalSignal = {
  '--hangup': 'SIGHUP',
  '--reraise-sigint': 'SIGINT',
}[process.argv[2]];

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
if (terminalSignal) {
  env.VP_PROBE_RAW = '1';
}
if (terminalSignal === 'SIGINT') {
  env.VP_PROBE_RERAISE_SIGINT = '1';
}
const initialTerminal = terminalSignal ? stty('-g') : undefined;
const args = ['dev', '--host', '127.0.0.1', '--port', '0', '--base', '/probe/'];
if (terminalSignal) {
  args.push('--debug', 'config');
}
const child = spawn(
  workerMode ? process.execPath : 'vp',
  workerMode ? ['worker-host.mjs', ...args] : args,
  { stdio: [terminalSignal ? 'inherit' : 'ignore', 'pipe', terminalSignal ? 'pipe' : 'inherit'], env },
);
const debugOutput = terminalSignal ? text(child.stderr) : undefined;

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
  } else if (terminalSignal) {
    assert.notEqual(stty('-g'), initialTerminal, 'the Vite plugin switched the terminal to raw mode');
  } else {
    assert.ok(
      readdirSync(cache, { recursive: true, withFileTypes: true }).some((entry) => entry.isFile()),
      'vp saves its compile cache before starting Vite',
    );
    console.log('vp saved its compile cache before starting Vite');
  }

  process.kill(pid, terminalSignal ?? 'SIGTERM');
  const [code, signal] = await once(child, 'exit', { signal: AbortSignal.timeout(5_000) });
  const socket = connect(port, '127.0.0.1');
  try {
    await assert.rejects(once(socket, 'connect', { signal: AbortSignal.timeout(3_000) }), {
      code: 'ECONNREFUSED',
    });
  } finally {
    socket.destroy();
  }
  if (terminalSignal) {
    const stderr = await debugOutput;
    assert.match(stderr, /vite:config/);
    console.log('Vite printed its config debug logs');
    if (terminalSignal === 'SIGINT') {
      assert.match(stderr, /Vite plugin re-sent SIGINT/);
      console.log('Vite ran the plugin callback and re-sent SIGINT');
    }
    assert.deepEqual([code, signal], [null, terminalSignal]);
    assert.equal(stty('-g'), initialTerminal, `${terminalSignal} restored the terminal mode`);
    console.log(`${terminalSignal} to the launch PID closed the port and restored the terminal`);
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
