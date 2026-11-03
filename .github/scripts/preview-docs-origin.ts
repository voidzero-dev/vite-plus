import { resolveDocsSiteOrigin } from '../../docs/.vitepress/site-origin.ts';
import { previewUrl } from './docs-fork-preview.mjs';

/** Use the same URLs as Workers Builds (same-repo) and the fork docs workflow. */
export function previewDocsOrigin(pr: {
  number: number;
  head: { ref: string; repo: { full_name: string } };
  base: { repo: { full_name: string } };
}): string {
  if (pr.head.repo.full_name !== pr.base.repo.full_name) {
    return previewUrl(pr.number);
  }
  return (
    resolveDocsSiteOrigin({ WORKERS_CI: '1', WORKERS_CI_BRANCH: pr.head.ref }) ??
    'https://viteplus.dev'
  );
}
