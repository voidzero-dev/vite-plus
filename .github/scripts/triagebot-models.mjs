import { createHash } from 'node:crypto';
import { registerHooks } from 'node:module';

// The pinned action's model registry predates Opus 5.5. Remove this hook once
// the action registers it; the checksum rejects unexpected bundle changes.
const actionCommit = '51d30da13c0bd571a3822fc56882de40fead0086';
const bundleSha256 = '4d474301341eb57b730fff881dac9e56bcbb10757ef7cd4c9e70e2d9ac48af85';

registerHooks({
  load(url, context, nextLoad) {
    const loaded = nextLoad(url, context);
    if (!url.endsWith(`/withastro/triagebot-action/${actionCommit}/dist/index.mjs`)) {
      return loaded;
    }

    const source = Buffer.from(loaded.source).toString('utf8');
    if (createHash('sha256').update(source).digest('hex') !== bundleSha256) {
      throw new Error('Unexpected triagebot bundle; review the Opus 5.5 compatibility hook.');
    }

    // https://developers.cloudflare.com/ai/models/anthropic/claude-opus-5.5/
    // Opus 5.5 uses the same Messages API with adaptive thinking always enabled.
    // Flue's default thinking level is medium, matching Astro's factory config.
    const registration = `
init_models2();
modelRegistry.get('cloudflare-ai-gateway').set('claude-opus-5-5', {
  ...getModel('cloudflare-ai-gateway', 'claude-opus-4-8'),
  id: 'claude-opus-5-5',
  name: 'Claude Opus 5.5',
  thinkingLevelMap: { off: null },
  cost: { input: 4, output: 20, cacheRead: 0.2, cacheWrite: 5 },
});
`;

    return {
      ...loaded,
      source: source.replace(
        'main().catch((err2) => {',
        `${registration}\nmain().catch((err2) => {`,
      ),
    };
  },
});
