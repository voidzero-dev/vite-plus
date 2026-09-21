import { createHash } from 'node:crypto';

// Keep this host aligned with docs/wrangler.jsonc and the Workers subdomain.
const workersHost = 'viteplus-dev.voidzero-docs.workers.dev';

export function resolveDocsSiteOrigin(env: NodeJS.ProcessEnv = process.env): string | undefined {
  if (env.DOCS_SITE_ORIGIN) {
    return env.DOCS_SITE_ORIGIN.replace(/\/+$/, '');
  }

  const branch = env.WORKERS_CI_BRANCH;
  // GitHub deploys supply DOCS_SITE_ORIGIN. Workers Builds needs its own
  // branch origin; main and local builds retain the production fallback.
  if (env.WORKERS_CI !== '1' || !branch || branch === 'main') {
    return undefined;
  }

  // Match Wrangler's generatePreviewAlias, including its long-branch hash:
  // https://developers.cloudflare.com/workers/versions-and-deployments/preview-urls/
  let alias = branch
    .replace(/[^a-zA-Z0-9-]/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-+|-+$/g, '')
    .toLowerCase();
  if (!/^[a-z](?:[a-z0-9-]*[a-z0-9])?$/.test(alias)) {
    throw new Error('Set DOCS_SITE_ORIGIN: this branch has no valid Workers preview alias.');
  }
  const maxAliasLength = 63 - workersHost.split('.')[0].length - 1;
  if (alias.length > maxAliasLength) {
    const hash = createHash('sha256').update(branch).digest('hex').slice(0, 4);
    alias = `${alias.slice(0, maxAliasLength - hash.length - 1)}-${hash}`;
  }
  return `https://${alias}-${workersHost}`;
}
