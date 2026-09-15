// Run with node --test; no workspace dependencies are needed.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const script = fileURLToPath(new URL('../retry-install.sh', import.meta.url));
const assertion =
  'Assertion failed: !(handle->flags & UV_HANDLE_CLOSING), file src\\win\\async.c, line 76';

function run(t, attempts, runnerOS = 'Linux') {
  const directory = mkdtempSync(join(tmpdir(), 'ci install retry '));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const child = join(directory, 'install.cjs');
  writeFileSync(
    child,
    `const fs = require('node:fs');
const attempts = ${JSON.stringify(attempts)};
const counter = ${JSON.stringify(join(directory, 'counter'))};
const index = fs.existsSync(counter) ? Number(fs.readFileSync(counter)) : 0;
fs.writeFileSync(counter, String(index + 1));
fs.writeFileSync(process.env.SFW_JSON_REPORT_PATH, JSON.stringify({ attempt: index + 1 }));
const attempt = attempts[Math.min(index, attempts.length - 1)];
console.log(process.argv[2]);
console.error(attempt.output);
process.exit(attempt.status);
`,
  );
  const logPrefix = join(directory, 'logs', 'install');
  const result = spawnSync(
    'bash',
    [script, logPrefix, process.execPath, child]
      .map((path) => path.replaceAll('\\', '/'))
      .concat('argument with spaces'),
    {
      encoding: 'utf8',
      env: { ...process.env, RUNNER_OS: runnerOS, CI_INSTALL_RETRY_DELAY_SECONDS: '0' },
    },
  );
  assert.ifError(result.error);
  const logs = readdirSync(join(directory, 'logs'))
    .filter((name) => name.endsWith('.log'))
    .toSorted()
    .map((name) => readFileSync(join(directory, 'logs', name), 'utf8'));
  for (let attempt = 1; attempt <= logs.length; attempt++) {
    assert.deepEqual(JSON.parse(readFileSync(`${logPrefix}.${attempt}.log.json`, 'utf8')), {
      attempt,
    });
  }
  for (const log of logs) {
    assert.match(log, /argument with spaces/);
  }
  return { ...result, logs, attempts: Number(readFileSync(join(directory, 'counter'))) };
}

for (const output of [
  assertion,
  'Socket Firewall encountered an unexpected error: TypeError fetch failed\nCause: Error read ECONNRESET',
  'HTTPError: Response code 504 (Gateway Time-out)',
]) {
  test(`retries and preserves logs: ${output.split('\n')[0]}`, (t) => {
    const result = run(
      t,
      [
        { status: output === assertion ? 127 : 1, output },
        { status: 0, output: 'installed' },
      ],
      'Windows',
    );
    assert.equal(result.status, 0);
    assert.equal(result.attempts, 2);
    assert.ok(result.logs[0].includes(output));
    assert.match(result.logs[1], /installed/);
    assert.match(result.stdout, /::warning::/);
  });
}

test('fails after three transport failures with the original exit code', (t) => {
  const result = run(t, [{ status: 137, output: 'Error read ECONNRESET' }]);
  assert.equal(result.status, 137);
  assert.equal(result.attempts, 3);
});

for (const [status, output] of [
  [1, "npm error Cannot read properties of null (reading 'edgesOut')"],
  [1, 'HTTP status client error (409 Conflict)\nError read ECONNRESET'],
  [1, 'HTTPError: Response code 403 (Forbidden)\nHTTPError: Response code 504'],
  [1, 'Package blocked by policy\nError read ECONNRESET'],
  [1, 'ERR_PNPM_MINIMUM_RELEASE_AGE\nError read ECONNRESET'],
  [127, 'sfw: command not found'],
  [130, 'Error read ECONNRESET'],
  [143, 'Error read ECONNRESET'],
  [0, 'WARN ECONNRESET (recovered)'],
]) {
  test(`does not retry exit ${status}: ${output.split('\n')[0]}`, (t) => {
    const result = run(t, [{ status, output }]);
    assert.equal(result.status, status);
    assert.equal(result.attempts, 1);
  });
}

test('does not retry the Windows assertion on another platform', (t) => {
  const result = run(t, [{ status: 127, output: assertion }]);
  assert.equal(result.status, 127);
  assert.equal(result.attempts, 1);
});

test('does not accept a Socket Firewall internal error with exit zero', (t) => {
  const result = run(t, [
    { status: 0, output: 'Socket Firewall encountered an unexpected error: broken proxy' },
  ]);
  assert.equal(result.status, 1);
  assert.equal(result.attempts, 1);
});

test('recovers from a Socket Firewall transport error reported with exit zero', (t) => {
  const result = run(t, [
    { status: 0, output: 'Socket Firewall encountered an unexpected error: Error read ECONNRESET' },
    { status: 0, output: 'installed' },
  ]);
  assert.equal(result.status, 0);
  assert.equal(result.attempts, 2);
});
