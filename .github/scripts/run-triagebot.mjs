import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

// withastro/triagebot-action has no custom-model input. Keep its bundle pinned
// until upstream's model registry includes GLM-5.3 Flash through AI Gateway.
export const bundleUrl =
  'https://raw.githubusercontent.com/withastro/triagebot-action/51d30da13c0bd571a3822fc56882de40fead0086/dist/index.mjs';
const bundleSha256 = '4d474301341eb57b730fff881dac9e56bcbb10757ef7cd4c9e70e2d9ac48af85';

export function registerGlmModel(source) {
  assert.equal(
    createHash('sha256').update(source).digest('hex'),
    bundleSha256,
    'Unexpected triagebot bundle; review the model registration before updating the pin',
  );

  const entrypoint = 'main().catch((err2) => {';
  assert.equal(source.split(entrypoint).length, 2, 'Expected one triagebot entrypoint');
  return source.replace(
    entrypoint,
    `
// GLM-5.3 Flash: https://developers.cloudflare.com/workers-ai/models/glm-5.3-flash/
init_models2();
{
  const models = modelRegistry.get('cloudflare-ai-gateway');
  const template = models.get('workers-ai/@cf/zai-org/glm-4.7-flash');
  const id = 'workers-ai/@cf/zai-org/glm-5.3-flash';
  models.set(id, {
    ...template,
    id,
    name: 'GLM-5.3 Flash',
    compat: { ...template.compat, supportsReasoningEffort: true },
    input: ['text', 'image'],
    contextWindow: 1048576,
    maxTokens: 65536,
    thinkingLevelMap: { off: null, minimal: 'low', medium: 'high', xhigh: 'max' },
    cost: { input: 0.15, output: 0.5, cacheRead: 0.03, cacheWrite: 0 },
  });
}
${entrypoint}`,
  );
}

async function main() {
  const response = await fetch(bundleUrl, { signal: AbortSignal.timeout(60_000) });
  assert.ok(response.ok, `Failed to download triagebot: HTTP ${response.status}`);
  const source = registerGlmModel(await response.text());
  // The action stages all repository changes, so keep its patched bundle outside
  // the checkout alongside the other temporary triage files.
  const directory = await mkdtemp(join(tmpdir(), 'vite-plus-triagebot-'));
  try {
    const entrypoint = join(directory, 'index.mjs');
    await writeFile(entrypoint, source);
    const result = spawnSync(process.execPath, [entrypoint], { stdio: 'inherit' });
    if (result.error) {
      throw result.error;
    }
    process.exitCode = result.status ?? 1;
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await main();
}
