import { lstat, readdir } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const repository = 'voidzero-dev/vite-plus';
const buildWorkflow = '.github/workflows/build-docs-fork-preview.yml';
const marker = '<!-- cloudflare-docs-fork-preview -->';

export function previewUrl(number) {
  if (!Number.isSafeInteger(number) || number <= 0) {
    throw new Error('Invalid pull request number');
  }
  return `https://pr-${number}-viteplus-dev.voidzero-docs.workers.dev`;
}

function previewRun(context) {
  const run = context.payload.workflow_run;
  if (
    `${context.repo.owner}/${context.repo.repo}` !== repository ||
    run?.path !== buildWorkflow ||
    run.event !== 'pull_request' ||
    run.conclusion !== 'success' ||
    !/^[a-f0-9]{40}$/.test(run.head_sha) ||
    !run.head_repository?.id ||
    !run.head_repository.owner?.login ||
    !run.head_repository.full_name ||
    !run.head_branch
  ) {
    throw new Error('Invalid docs preview workflow run');
  }
  return run;
}

function matchesPreview(pr, run) {
  return (
    pr.state === 'open' &&
    pr.base.repo.full_name === repository &&
    pr.base.ref === 'main' &&
    pr.head.repo?.full_name !== repository &&
    pr.head.repo?.id === run.head_repository.id &&
    pr.head.repo?.full_name === run.head_repository.full_name &&
    pr.head.ref === run.head_branch &&
    pr.head.sha === run.head_sha
  );
}

export async function authorizePreview({ github, context, core }) {
  const run = previewRun(context);
  if (run.head_repository.full_name === repository) {
    return;
  }

  // workflow_run.pull_requests and commit association can be empty for forks.
  // Resolve by source owner/branch, then require the exact repo and current SHA.
  const pulls = await github.paginate(github.rest.pulls.list, {
    ...context.repo,
    state: 'open',
    base: 'main',
    head: `${run.head_repository.owner.login}:${run.head_branch}`,
  });
  const candidates = pulls.filter((pr) => matchesPreview(pr, run));
  if (candidates.length === 0) {
    core.info('No open fork PR has this head commit; skipping the preview.');
    return;
  }
  if (candidates.length !== 1) {
    throw new Error('More than one PR matches the docs preview run');
  }

  const artifacts = await github.paginate(github.rest.actions.listWorkflowRunArtifacts, {
    ...context.repo,
    run_id: run.id,
  });
  const matches = artifacts.filter((a) => a.name === 'docs-fork-preview' && !a.expired);
  if (matches.length !== 1) {
    throw new Error('Expected one active docs-fork-preview artifact from the triggering run');
  }
  core.setOutput('pr', candidates[0].number);
  core.setOutput('artifact-id', matches[0].id);
  core.setOutput('preview-url', previewUrl(candidates[0].number));
}

export async function isCurrentPreview({ github, context }, number) {
  previewUrl(number);
  const run = previewRun(context);
  const { data: pr } = await github.rest.pulls.get({ ...context.repo, pull_number: number });
  return matchesPreview(pr, run);
}

export async function commentPreview({ github, context, core }, number) {
  if (!(await isCurrentPreview({ github, context }, number))) {
    core.info('The PR closed or changed during upload; skipping the preview comment.');
    return;
  }
  const body = `${marker}\nCloudflare documentation preview: ${previewUrl(number)}\n\nCommit: ${context.payload.workflow_run.head_sha}`;
  const comments = await github.paginate(github.rest.issues.listComments, {
    ...context.repo,
    issue_number: number,
  });
  const existing = comments.find(
    (comment) => comment.user?.login === 'github-actions[bot]' && comment.body?.startsWith(marker),
  );
  if (existing) {
    await github.rest.issues.updateComment({ ...context.repo, comment_id: existing.id, body });
  } else {
    await github.rest.issues.createComment({ ...context.repo, issue_number: number, body });
  }
}

async function validateAssetEntry(path) {
  const stat = await lstat(path);
  if (stat.isDirectory()) {
    for (const name of await readdir(path)) {
      await validateAssetEntry(join(path, name));
    }
  } else if (!stat.isFile()) {
    throw new Error(`Preview assets must be regular files: ${path}`);
  }
}

export async function validateAssets(directory) {
  // Never follow links from an untrusted artifact: a link could upload files
  // outside the artifact directory, including deployment credentials.
  await validateAssetEntry(directory);
  if (!(await lstat(join(directory, 'index.html'))).isFile()) {
    throw new Error('Preview assets must include index.html');
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  if (process.argv[2] !== 'validate-assets' || !process.argv[3]) {
    throw new Error('Usage: node docs-fork-preview.mjs validate-assets <directory>');
  }
  await validateAssets(process.argv[3]);
}
