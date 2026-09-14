import { lstat, readdir } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const repository = 'voidzero-dev/vite-plus';
const buildWorkflow = '.github/workflows/build-docs-fork-preview.yml';
const marker = '<!-- cloudflare-docs-fork-preview -->';
const previewLabel = 'docs-preview';

function validatePrNumber(number) {
  if (!Number.isSafeInteger(number) || number <= 0) {
    throw new Error('Invalid pull request number');
  }
}

export function previewAlias(number) {
  validatePrNumber(number);
  return `pr-${number}`;
}

export function previewUrl(number) {
  return `https://${previewAlias(number)}-viteplus-dev.voidzero-docs.workers.dev`;
}

function previewRun(context) {
  const run = context.payload.workflow_run;
  if (
    `${context.repo.owner}/${context.repo.repo}` !== repository ||
    context.eventName !== 'workflow_run' ||
    !Number.isSafeInteger(context.runId) ||
    context.runId <= 0 ||
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
  if (![run.id, run.run_attempt].every((value) => Number.isSafeInteger(value) && value > 0)) {
    throw new Error('Invalid docs preview build identity');
  }
  return run;
}

function matchesPreview(pr, run) {
  return (
    pr.state === 'open' &&
    pr.labels?.some((label) => label.name === previewLabel) === true &&
    pr.base.repo.full_name === repository &&
    pr.base.ref === 'main' &&
    pr.head.repo?.full_name !== repository &&
    pr.head.repo?.id === run.head_repository.id &&
    pr.head.repo?.full_name === run.head_repository.full_name &&
    pr.head.ref === run.head_branch &&
    pr.head.sha === run.head_sha
  );
}

async function hasWritePermission({ github, context }, login) {
  if (!login) {
    return false;
  }
  const { data } = await github.rest.repos.getCollaboratorPermissionLevel({
    ...context.repo,
    username: login,
  });
  return ['admin', 'maintain', 'write'].includes(data.permission);
}

async function hasDeploymentApproval({ github, context }) {
  previewRun(context);
  // Use the current trusted deployment run, never an ID from the fork's
  // artifact or the build run. This run's immutable event identifies the build.
  // Checking the history also fails closed if the environment has no rules.
  const { data } = await github.rest.actions.getReviewsForRun({
    ...context.repo,
    run_id: context.runId,
  });
  const reviews = data.filter((review) =>
    review.environments?.some((environment) => environment.name === previewLabel),
  );
  // The API does not document history order. Refuse mixed decisions instead
  // of guessing which one is newest; reapply the label to request a new run.
  if (reviews.length === 0 || reviews.some((review) => review.state !== 'approved')) {
    return false;
  }
  for (const review of reviews) {
    if (await hasWritePermission({ github, context }, review.user?.login)) {
      return true;
    }
  }
  return false;
}

export async function requireDeploymentApproval({ github, context }) {
  if (!(await hasDeploymentApproval({ github, context }))) {
    throw new Error(
      'This run needs maintainer approval for docs-preview. Configure required reviewers in the environment, then reapply the label and approve the new deployment.',
    );
  }
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
    core.info('No open fork PR with docs-preview has this head commit; skipping the preview.');
    return;
  }
  if (candidates.length !== 1) {
    throw new Error('More than one PR matches the docs preview run');
  }
  // Use actor, not triggering_actor: rerunning an outsider's build must not
  // turn it into a maintainer's preview request.
  if (!(await hasWritePermission({ github, context }, run.actor?.login))) {
    core.info('The original run actor does not have write permission; skipping the preview.');
    return;
  }

  const artifacts = await github.paginate(github.rest.actions.listWorkflowRunArtifacts, {
    ...context.repo,
    run_id: run.id,
  });
  // Helper-only and unrelated label runs can succeed without building docs.
  if (artifacts.length === 0) {
    core.info('The workflow produced no artifacts; skipping the preview.');
    return;
  }
  // A rerun has its own artifact. Deploy only the triggering attempt's output,
  // even when a delayed deployment lists artifacts from newer attempts.
  const artifactName = `docs-fork-preview-${run.run_attempt}`;
  const matches = artifacts.filter((a) => a.name === artifactName && !a.expired);
  if (matches.length !== 1) {
    throw new Error(`Expected one active ${artifactName} artifact from the triggering run`);
  }
  // Environment approval happens after this job. Recheck the PR before
  // requesting it, then recheck again after the deployment job's approval wait.
  const { data: current } = await github.rest.pulls.get({
    ...context.repo,
    pull_number: candidates[0].number,
  });
  if (
    !matchesPreview(current, run) ||
    !(await hasWritePermission({ github, context }, run.actor?.login))
  ) {
    core.info('The PR changed or preview permission was revoked; skipping the deployment queue.');
    return;
  }
  validatePrNumber(candidates[0].number);
  core.setOutput('pr', candidates[0].number);
  core.setOutput('artifact-id', matches[0].id);
  core.setOutput('preview-alias', previewAlias(candidates[0].number));
  core.setOutput('preview-url', previewUrl(candidates[0].number));
}

export async function isCurrentPreview({ github, context }, number) {
  validatePrNumber(number);
  const run = previewRun(context);
  const { data: pr } = await github.rest.pulls.get({ ...context.repo, pull_number: number });
  return (
    matchesPreview(pr, run) &&
    (await hasWritePermission({ github, context }, run.actor?.login)) &&
    (await hasDeploymentApproval({ github, context }))
  );
}

function uploadedPreviewUrl(output, expectedAliasUrl) {
  // WRANGLER_OUTPUT_FILE_PATH contains JSONL, not console output. Require one
  // upload from this job, its version URL, and the alias used by its installers.
  const uploads = output
    .split('\n')
    .filter((line) => line.trim() !== '')
    .map((line) => JSON.parse(line))
    .filter((entry) => entry?.type === 'version-upload');
  if (uploads.length !== 1) {
    throw new Error('Expected one version-upload record from Wrangler');
  }
  const [upload] = uploads;
  if (
    upload.version !== 1 ||
    upload.worker_name !== 'viteplus-dev' ||
    !/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/.test(upload.version_id) ||
    upload.preview_url !==
      `https://${upload.version_id.slice(0, 8)}-viteplus-dev.voidzero-docs.workers.dev` ||
    upload.preview_alias_url !== expectedAliasUrl
  ) {
    throw new Error(
      'Invalid preview URLs from Wrangler; check that Preview URLs and the PR alias are configured',
    );
  }
  return upload.preview_alias_url;
}

export async function commentPreview({ github, context, core }, number, output) {
  if (!(await isCurrentPreview({ github, context }, number))) {
    core.info('The PR changed or preview permission was revoked; skipping the preview comment.');
    return;
  }
  const run = previewRun(context);
  const body = `${marker}\nCloudflare documentation preview: ${uploadedPreviewUrl(output, previewUrl(number))}\n\nCommit: ${run.head_sha}`;
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
