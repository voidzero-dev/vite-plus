import assert from 'node:assert/strict';
import { fork, spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtempSync, readdirSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { failAndStopGroup, linesWithin, localLauncher, parseListening } from './helpers.mjs';

// `vp dev` runs Vite in the launcher process, so each Node.js option, preload,
// and IPC channel of the launcher applies to Vite once, as when Vite runs directly.
const launcher = localLauncher();
const devArgs = ['dev', '--host', '127.0.0.1', '--port', '0'];

function withNodeOptions(value) {
  return [value, process.env.NODE_OPTIONS].filter(Boolean).join(' ');
}

function checkOneProfile({ dir, pid }) {
  const found = readdirSync(dir).filter((name) => name.endsWith('.cpuprofile'));
  assert.equal(found.length, 1, `expected one CPU profile, found: ${found.join(', ')}`);
  assert.ok(found[0].includes(`.${pid}.`), 'the profile belongs to the launch PID');
  const { nodes } = JSON.parse(readFileSync(join(dir, found[0]), 'utf8'));
  assert.ok(
    nodes.some(({ callFrame }) => /[\\/]vite[\\/](bin|node)[\\/]/.test(callFrame.url)),
    'the profile covers Vite',
  );
}

const settings = [
  {
    name: 'node --cpu-prof',
    nodeArgs: (dir) => ['--cpu-prof', `--cpu-prof-dir=${dir}`],
    check: checkOneProfile,
  },
  {
    name: 'NODE_OPTIONS=--cpu-prof',
    env: (dir) => ({ NODE_OPTIONS: withNodeOptions(`--cpu-prof --cpu-prof-dir=${dir}`) }),
    check: checkOneProfile,
  },
  {
    name: 'NODE_OPTIONS=--inspect',
    env: () => ({ NODE_OPTIONS: withNodeOptions('--inspect=127.0.0.1:0') }),
    check: ({ stderr }) =>
      assert.equal(stderr.match(/Debugger listening on /g)?.length, 1, 'one debugger session'),
  },
  {
    name: 'NODE_V8_COVERAGE',
    env: (dir) => ({ NODE_V8_COVERAGE: dir }),
    check: ({ dir, pid }) => {
      const found = readdirSync(dir).filter((name) => name.startsWith('coverage-'));
      assert.deepEqual(
        found.map((name) => name.split('-')[1]),
        [String(pid)],
        'one coverage file, from the launch PID',
      );
    },
  },
  {
    name: 'NODE_OPTIONS=--import',
    env: () => ({ NODE_OPTIONS: withNodeOptions('--import=./exit-hook.mjs') }),
    check: ({ stderr }) => {
      assert.equal(stderr.match(/^preload loaded$/gm)?.length, 1, 'the preload loaded once');
      assert.equal(stderr.match(/^preload exit hook ran$/gm)?.length, 1, 'its exit hook ran once');
    },
  },
  {
    name: 'vp dev --debug config',
    args: ['--debug', 'config'],
    check: ({ stderr }) => assert.match(stderr, /vite:config/, 'Vite printed its debug log'),
  },
  {
    name: 'fork() with IPC',
    fork: true,
    check: ({ messages, pid }) =>
      assert.deepEqual(
        messages.map((message) => message.pid),
        [pid],
        'Vite sent a message over the IPC channel',
      ),
  },
];

async function vitePid(stdout, seen) {
  for await (const line of linesWithin(stdout)) {
    seen.push(line);
    const listener = parseListening(line);
    if (listener) {
      return listener.pid;
    }
  }
  assert.fail('Vite did not start listening');
}

async function run(setting) {
  const dir = mkdtempSync(join(tmpdir(), 'vp-dev-in-process-'));
  const env = { ...process.env, ...setting.env?.(dir) };
  const args = [...devArgs, ...(setting.args ?? [])];
  const stdio = ['ignore', 'pipe', 'pipe'];
  let child;
  if (setting.fork) {
    child = fork(launcher, args, { env, stdio: [...stdio, 'ipc'] });
  } else if (setting.nodeArgs) {
    child = spawn(process.execPath, [...setting.nodeArgs(dir), launcher, ...args], { env, stdio });
  } else {
    child = spawn('vp', args, { env, stdio });
  }
  // The inspector prints a random URL to stderr, so keep stderr out of the snapshot.
  let stderr = '';
  child.stderr.on('data', (chunk) => (stderr += chunk));
  const messages = [];
  child.on('message', (message) => messages.push(message));
  const stdout = [];
  try {
    const pid = await vitePid(child.stdout, stdout);
    assert.equal(pid, child.pid, 'Vite runs in the launcher process');
    process.kill(pid, 'SIGTERM');
    // `close` waits for the output that exit hooks write.
    const [code, signal] = await once(child, 'close', { signal: AbortSignal.timeout(5_000) });
    assert.deepEqual([code, signal], [143, null]);
    setting.check({ dir, pid, stderr, messages });
    console.log(`${setting.name}: applied once, to Vite in the launch PID`);
  } catch (error) {
    throw new Error(
      `${setting.name} failed.\nLauncher stdout:\n${stdout.join('\n')}\nLauncher stderr:\n${stderr}`,
      { cause: error },
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

try {
  for (const setting of settings) {
    await run(setting);
  }
} catch (error) {
  failAndStopGroup(error);
}
