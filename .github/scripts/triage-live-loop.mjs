import assert from 'node:assert/strict';
import child from 'node:child_process';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import { syncBuiltinESMExports } from 'node:module';
import path from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';
import { promisify } from 'node:util';

const wire = '/wire';
const configuredModel = process.argv[2];
assert.equal(configuredModel, 'cloudflare-ai/@cf/zai-org/glm-5.3');
const model = configuredModel.slice('cloudflare-ai/'.length);
const account = 'fixture';
const number = 428193;
const repo = 'synthetic/fixture';
const api = 'https://api.github.com/repos/' + repo;
const endpoint = 'https://api.cloudflare.com/client/v4/accounts/' + account + '/ai/v1/chat/completions';
const cwd = fs.realpathSync(fs.mkdtempSync('/tmp/triage-fixture-'));
const skill = '.agents/skills/triage';
const reportDir = '/tmp/vite-plus-triage/gh-' + number;
const report = reportDir + '/report.md';
const issue = {
  title: 'Feature request: add an optional purple progress indicator',
  body: 'This is a Vite+ feature request. All commands work as documented. I would like an optional purple progress indicator in a future release. There is no bug or failing command to reproduce.',
  number,
  user: { login: 'fixture' },
  created_at: '2026-11-20T00:00:00Z',
  state: 'open',
  labels: [],
  html_url: 'https://example.invalid/issues/' + number,
};

const writeSync = fs.writeFileSync.bind(fs);
const renameSync = fs.renameSync.bind(fs);
const existsSync = fs.existsSync.bind(fs);
const readSync = fs.readFileSync.bind(fs);
const statSync = fs.statSync.bind(fs);
process.chdir(cwd);
for (const key of Object.keys(process.env)) delete process.env[key];
Object.assign(process.env, {
  HOME: cwd,
  GITHUB_EVENT_PATH: path.join(cwd, 'event.json'),
  GITHUB_REPOSITORY: repo,
  'INPUT_CLOUDFLARE-API-KEY': 'fixture',
  'INPUT_CLOUDFLARE-ACCOUNT-ID': account,
  'INPUT_READ-TOKEN': 'fixture',
  'INPUT_WRITE-TOKEN': 'fixture',
  'INPUT_TRIAGE-SKILL': skill,
  'INPUT_TRIAGE-MODEL': configuredModel,
});
writeSync(process.env.GITHUB_EVENT_PATH, JSON.stringify({ action: 'opened', issue }));

const files = new Map([[path.join(cwd, 'AGENTS.md'), readSync('/fixture-guide', 'utf8')]]);
for (const name of ['SKILL.md', 'reproduce.md', 'diagnose.md', 'verify.md', 'fix.md']) {
  files.set(path.join(cwd, skill, name), readSync(path.join('/fixture-source', name), 'utf8'));
}
const dirs = new Set([cwd, path.join(cwd, '.agents'), path.join(cwd, '.agents/skills'), path.join(cwd, skill), reportDir]);
function readFixture(file) {
  const key = path.resolve(cwd, file);
  assert.ok(files.has(key), 'Missing public fixture file: ' + key);
  return files.get(key);
}
function writeReport(file, content) {
  assert.equal(path.resolve(cwd, file), report);
  assert.ok(typeof content === 'string' && content.length <= 64000);
  files.set(report, content);
}

fsp.readFile = async (file, encoding) => encoding ? readFixture(file) : Buffer.from(readFixture(file));
fsp.writeFile = async (file, content) => writeReport(file, content);
fsp.mkdir = async file => { assert.ok(dirs.has(path.resolve(cwd, file))); };
fsp.stat = async file => {
  const key = path.resolve(cwd, file);
  assert.ok(files.has(key) || dirs.has(key));
  return { isFile: () => files.has(key), isDirectory: () => dirs.has(key),
    isSymbolicLink: () => false, size: files.get(key)?.length ?? 0, mtime: new Date(0) };
};
fsp.readdir = async file => {
  const key = path.resolve(cwd, file);
  assert.ok(dirs.has(key));
  return [...files.keys(), ...dirs].filter(p => path.dirname(p) === key).map(p => path.basename(p));
};
fsp.access = async file => { await fsp.stat(file); };
fsp.rm = async () => { throw new Error('Fixture files cannot be removed'); };

function shell(command) {
  // All output is computed from in-memory fixtures. No model command executes.
  assert.ok(typeof command === 'string' && command.length <= 64000);
  const text = command.trim();
  if (text === 'git checkout -B "triagebot/fix-' + number + '"' || text === 'git diff main --stat') return '';
  if (text === 'pwd' || text === 'git rev-parse --show-toplevel') return cwd;
  if (/^mkdir -p ['"]?[^;&|<>\n]+['"]?$/.test(text)) return '';
  const glob = text.match(/^find '([^']*)' -type f -name '([^']*)' 2>\/dev\/null \| head -\d+$/);
  if (glob) {
    const directory = path.resolve(cwd, glob[1]);
    return [...files.keys()].filter(file => file.startsWith(directory + '/') &&
      path.matchesGlob(path.basename(file), glob[2])).join('\n');
  }
  const grep = text.match(/^grep -rn (?:--include='([^']+)' )?'([^']+)' '([^']+)'$/);
  if (grep) {
    const target = path.resolve(cwd, grep[3]);
    const matches = [];
    for (const [file, content] of files) {
      if (file !== target && !file.startsWith(target + '/')) continue;
      if (grep[1] && !path.matchesGlob(path.basename(file), grep[1])) continue;
      content.split('\n').forEach((line, i) => {
        if (line.includes(grep[2])) matches.push(file + ':' + (i + 1) + ':' + line);
      });
    }
    return matches.join('\n');
  }
  const cat = text.match(/^cat\s+['"]?([^'"\s;|<>]+)['"]?$/);
  if (cat) return readFixture(cat[1]);
  // A common shell equivalent of the write tool. Treat the body as literal text.
  const heredoc = text.match(/(?:^|\n|&&\s*)cat\s*>\s*['"]?([^'"\s]+)['"]?\s*<<\s*['"]?(\w+)['"]?\n([\s\S]*)\n\2$/);
  if (heredoc && heredoc[1] === report) {
    writeReport(report, heredoc[3] + '\n');
    return '';
  }
  throw new Error('Use the read tool for fixture files, write for ' + report + ', and finish to return your result.');
}
function execStub(command, options, callback) {
  if (typeof options === 'function') callback = options;
  try { callback(null, shell(command), ''); }
  catch (error) { callback(Object.assign(error, { code: 1 }), '', error.message); }
}
execStub[promisify.custom] = async command => {
  try { return { stdout: shell(command), stderr: '' }; }
  catch (error) { throw Object.assign(error, { code: 1, stdout: '', stderr: error.message }); }
};
child.exec = execStub;
for (const method of ['execFile', 'spawn', 'spawnSync', 'fork', 'execSync', 'execFileSync']) {
  child[method] = () => { throw new Error('Fixture subprocesses are disabled'); };
}
syncBuiltinESMExports();

let requests = 0;
let responses = 0;
const toolCalls = [];
const allowedTools = new Set(['finish', 'give_up', 'read', 'write', 'bash', 'glob', 'grep']);
const labels = [];
const comments = [];
function checkTools(bytes) {
  const messages = bytes.toString('utf8').replace(/\r\n/g, '\n').split(/\n\n+/);
  for (const message of messages) {
    const data = message.split('\n').filter(l => l.startsWith('data:')).map(l => l.slice(5).trimStart()).join('\n');
    if (!data || data === '[DONE]') continue;
    for (const choice of JSON.parse(data).choices ?? []) {
      for (const call of choice.delta?.tool_calls ?? []) {
        if (call.function?.name) {
          assert.ok(allowedTools.has(call.function.name), 'Unsupported tool from model');
          toolCalls.push(call.function.name);
        }
      }
    }
  }
}
globalThis.fetch = async (target, init) => {
  const url = typeof target === 'string' ? target : target.url ?? target.toString();
  if (url === endpoint) {
    assert.equal(init.method, 'POST');
    const headers = new Headers(init.headers);
    assert.equal(headers.get('authorization'), 'Bearer fixture');
    assert.equal(headers.get('cf-aig-collect-log-payload'), 'false');
    const body = JSON.parse(init.body);
    assert.equal(body.model, model);
    assert.equal(body.stream, true);
    assert.ok(body.tools.some(t => t.function.name === 'finish'));
    assert.ok(++requests <= 16, 'Model request budget exhausted');
    const requestPath = path.join(wire, 'request-' + requests + '.json');
    writeSync(requestPath + '.tmp', init.body, { flag: 'wx' });
    renameSync(requestPath + '.tmp', requestPath);
    const responsePath = path.join(wire, 'response-' + requests + '.sse');
    const deadline = Date.now() + 120000;
    while (!existsSync(responsePath)) {
      assert.ok(Date.now() < deadline, 'Relay timed out on request ' + requests);
      await sleep(100);
    }
    assert.ok(statSync(responsePath).size <= 4 * 1024 * 1024);
    const response = readSync(responsePath);
    checkTools(response);
    responses++;
    return new Response(response, { status: 200, headers: { 'Content-Type': 'text/event-stream' } });
  }
  if (url === api + '/issues/' + number) return Response.json(issue);
  if (url === api + '/issues/' + number + '/comments?per_page=100') return Response.json([]);
  if (url === api + '/labels?per_page=100&page=1') return Response.json([{ name: '- P1: chore', description: 'Maintenance' }]);
  if (url === api + '/issues/' + number + '/comments' && init.method === 'POST') {
    comments.push(JSON.parse(init.body).body);
    return Response.json({});
  }
  if (url === api + '/issues/' + number + '/labels' && init.method === 'POST') {
    labels.push(JSON.parse(init.body).labels);
    return Response.json([]);
  }
  throw new Error('Unexpected request blocked');
};

// The pinned entrypoint starts main() without exposing its promise.
const defaultInfo = console.info;
let result;
console.info = (...args) => {
  if (args[0] === 'Triage pipeline result:') result = args[1];
};
let passed = false;
process.once('exit', () => {
  if (passed) return;
  const toolCounts = Object.fromEntries([...allowedTools].map(name => [
    name, toolCalls.filter(tool => tool === name).length,
  ]));
  defaultInfo(JSON.stringify({ requests, responses, toolCounts }));
});
process.once('beforeExit', () => {
  assert.equal(process.exitCode ?? 0, 0, 'Triage action failed');
  assert.equal(result.completedStage, 'reproduce');
  assert.equal(result.skipped, true);
  assert.equal(result.skippedReason, 'not-actionable');
  assert.ok(requests >= 2 && responses === requests);
  assert.deepEqual(labels, [['triage: not actionable']]);
  assert.equal(comments.length, 1);
  assert.ok(typeof comments[0] === 'string' && comments[0].length > 40 && /Reproduced:/i.test(comments[0]));
  passed = true;
  defaultInfo(JSON.stringify({ stage: result.completedStage, skippedReason: result.skippedReason,
    label: labels[0][0], commentLength: comments[0].length, reportWritten: files.has(report),
    http200Responses: responses, toolCalls }));
});
await import('/probe/action/dist/index.mjs');
