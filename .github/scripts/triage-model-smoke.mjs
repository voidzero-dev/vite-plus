import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { cpSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const [stage, model, skillPath, bundlePath] = process.argv.slice(2);
assert.ok(['triage', 'verification'].includes(stage));
assert.ok(model.startsWith('cloudflare-workers-ai/'));
const bundle = pathToFileURL(resolve(bundlePath));
const work = mkdtempSync(join(tmpdir(), 'issue-triage-'));
cpSync(skillPath, join(work, skillPath), { recursive: true });
process.chdir(work);
execFileSync('git', ['init', '-q', '-b', 'main']);
execFileSync('git', ['add', skillPath]);
execFileSync('git', ['-c', 'user.name=Fixture', '-c', 'user.email=test@example.invalid', 'commit', '-qm', 'Fixture']);

const verification = stage === 'verification';
const issue = {
  number: 1,
  title: 'Synthetic feature request',
  body: 'Please add a new display option.',
  labels: verification ? [{ name: 'triage: fix pending' }] : [],
  user: { login: 'reporter' },
  state: 'open',
  created_at: '2026-01-01T00:00:00Z',
  html_url: 'https://example.invalid/issues/1',
};
const comment = { user: issue.user, body: 'How do I try the preview?', author_association: 'NONE', created_at: issue.created_at };
const event = verification ? { action: 'created', issue, comment } : { action: 'opened', issue };
const eventPath = join(work, '.git', 'event.json');
writeFileSync(eventPath, JSON.stringify(event));
Object.assign(process.env, {
  GITHUB_EVENT_PATH: eventPath,
  GITHUB_REPOSITORY: 'synthetic/fixture',
  INPUT_TRIAGE_SKILL: skillPath,
  INPUT_READ_TOKEN: 'fixture',
  INPUT_WRITE_TOKEN: 'fixture',
  INPUT_CLOUDFLARE_API_KEY: 'fixture',
  INPUT_CLOUDFLARE_ACCOUNT_ID: 'fixture',
  ['INPUT_' + stage.toUpperCase() + '_MODEL']: model,
});

const modelId = model.slice('cloudflare-workers-ai/'.length);
const report = '- **Reproduced:** No. Synthetic feature request.';
const results = verification
  ? [{ status: 'inconclusive', reasoning: 'The reporter asked a question.' }]
  : [{ reproducible: false, skipped: true, skippedReason: 'not-actionable' }, { result: report }];
let requests = 0;
const writes = [];
globalThis.fetch = async (input, init) => {
  const url = new URL(input.url ?? input);
  const method = init?.method ?? 'GET';
  if (url.href === 'https://api.cloudflare.com/client/v4/accounts/fixture/ai/v1/chat/completions') {
    assert.equal(method, 'POST');
    const body = JSON.parse(init.body);
    assert.equal(body.model, modelId);
    assert.equal(body.stream, true);
    const finish = body.tools.find((tool) => tool.function.name === 'finish').function;
    const result = results[requests++];
    assert.deepEqual(Object.keys(result).sort(), finish.parameters.required.slice().sort());
    const tool = { index: 0, id: 'fixture-call-' + requests, type: 'function', function: { name: 'finish', arguments: JSON.stringify(result) } };
    const chunk = (delta, reason) => 'data: ' + JSON.stringify({
      id: 'fixture', object: 'chat.completion.chunk', model: modelId,
      choices: [{ index: 0, delta, finish_reason: reason }],
    }) + '\n\n';
    return new Response(chunk({ role: 'assistant', tool_calls: [tool] }, null) + chunk({}, 'tool_calls') + 'data: [DONE]\n\n', {
      headers: { 'Content-Type': 'text/event-stream' },
    });
  }

  assert.equal(url.origin, 'https://api.github.com');
  const prefix = '/repos/synthetic/fixture';
  assert.ok(url.pathname.startsWith(prefix + '/'));
  const path = url.pathname.slice(prefix.length);
  if (method === 'GET') {
    if (path === '/issues/1') return Response.json(issue);
    if (path === '/issues/1/comments') return Response.json(verification ? [comment] : []);
    if (path === '/labels') return Response.json([]);
    if (path === '/git/matching-refs/heads/triagebot/fix-1') return Response.json([{ ref: 'refs/heads/triagebot/fix-1' }]);
  }
  if (method === 'POST' && ['/issues/1/comments', '/issues/1/labels'].includes(path)) {
    writes.push(JSON.parse(init.body));
    return Response.json({});
  }
  throw new Error('Unexpected fixture request: ' + method + ' ' + url.href);
};

console.log('Stage:', stage, 'Model:', model);
process.once('beforeExit', () => {
  console.log('Intercepted Workers AI requests:', requests);
  if (process.exitCode) return;
  assert.equal(requests, results.length);
  assert.deepEqual(writes, verification ? [] : [{ body: report }, { labels: ['triage: not actionable'] }]);
});
await import(bundle);
