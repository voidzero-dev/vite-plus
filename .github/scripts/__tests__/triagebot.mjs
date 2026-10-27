// Run with node --test; no workspace dependencies or live inference are needed.
import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { pathToFileURL } from 'node:url';

import { bundleUrl, registerGlmModel } from '../run-triagebot.mjs';

test('rejects an unreviewed upstream bundle', () => {
  assert.throws(() => registerGlmModel('main().catch((err2) => {'), /Unexpected triagebot bundle/);
});

test('GLM-5.3 Flash uses AI Gateway authentication and supports agent tools', async (t) => {
  // Verify against the actual pinned dependency, rather than duplicating its
  // provider implementation. A cached download avoids network access locally.
  const source = process.env.TRIAGEBOT_TEST_BUNDLE
    ? await readFile(process.env.TRIAGEBOT_TEST_BUNDLE, 'utf8')
    : await (async () => {
        const response = await fetch(bundleUrl, { signal: AbortSignal.timeout(60_000) });
        assert.ok(response.ok, `Failed to download upstream fixture: HTTP ${response.status}`);
        return response.text();
      })();
  const prepared = registerGlmModel(source).replace(
    /main\(\)\.catch\(\(err2\) => \{\n[\s\S]*?\n\}\);/,
    'export { resolveModel, completeSimple };',
  );
  const directory = await mkdtemp(join(tmpdir(), 'vite-plus-triagebot-test-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const entrypoint = join(directory, 'index.mjs');
  await writeFile(entrypoint, prepared);
  const { resolveModel, completeSimple } = await import(pathToFileURL(entrypoint).href);
  const model = resolveModel('cloudflare-ai-gateway/workers-ai/@cf/zai-org/glm-5.3-flash');
  assert.equal(model.contextWindow, 1048576);

  const requests = [];
  t.mock.method(globalThis, 'fetch', async (input, init) => {
    const request = new Request(input, init);
    requests.push({ request, body: await request.json() });
    return new Response(
      `data: ${JSON.stringify({
        id: 'test-completion',
        choices: [{ index: 0, delta: { content: 'Verified' }, finish_reason: 'stop' }],
      })}\n\ndata: [DONE]\n\n`,
      { headers: { 'content-type': 'text/event-stream' } },
    );
  });
  for (const [reasoning, expected] of [
    ['medium', 'high'],
    ['xhigh', 'max'],
  ]) {
    const result = await completeSimple(
      model,
      {
        messages: [{ role: 'user', content: 'Check the reproduction', timestamp: 0 }],
        tools: [{ name: 'bash', description: 'Run a command', parameters: { type: 'object' } }],
      },
      {
        apiKey: 'test-cloudflare-key',
        env: { CLOUDFLARE_ACCOUNT_ID: 'test-account', CLOUDFLARE_GATEWAY_ID: 'default' },
        reasoning,
      },
    );
    assert.equal(result.stopReason, 'stop', result.errorMessage);
    assert.equal(result.content[0].text, 'Verified');
    const { request, body } = requests.at(-1);
    assert.equal(
      request.url,
      'https://gateway.ai.cloudflare.com/v1/test-account/default/compat/chat/completions',
    );
    assert.equal(request.headers.get('cf-aig-authorization'), 'Bearer test-cloudflare-key');
    assert.equal(request.headers.get('authorization'), null);
    assert.equal(body.model, 'workers-ai/@cf/zai-org/glm-5.3-flash');
    assert.equal(body.reasoning_effort, expected);
    assert.equal(body.tools[0].function.name, 'bash');
  }
  assert.equal(requests.length, 2);
});
