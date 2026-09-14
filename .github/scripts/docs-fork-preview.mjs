import { lstat, readdir } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { setTimeout } from 'node:timers/promises';
import { pathToFileURL } from 'node:url';

const repository = 'voidzero-dev/vite-plus';
const buildWorkflow = '.github/workflows/build-docs-fork-preview.yml';
const approvalWorkflow = '.github/workflows/approve-docs-fork-preview.yml';
const marker = '<!-- cloudflare-docs-fork-preview -->';
const previewLabel = 'docs-preview';
const approvalRunPrefix = `https://github.com/${repository}/actions/runs/`;

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

async function requestedByMaintainer({ github, context }, run) {
  // Use the original actor, not triggering_actor: a maintainer re-running an
  // outsider's workflow must not grant that run deployment permission.
  if (!run.actor?.login) {
    return false;
  }
  const { data } = await github.rest.repos.getCollaboratorPermissionLevel({
    ...context.repo,
    username: run.actor.login,
  });
  return ['admin', 'maintain', 'write'].includes(data.permission);
}

export async function approvePreview({ github, context, core }) {
  const snapshot = context.payload.pull_request;
  if (
    `${context.repo.owner}/${context.repo.repo}` !== repository ||
    context.eventName !== 'pull_request_target' ||
    context.payload.action !== 'labeled' ||
    context.payload.label?.name !== previewLabel ||
    !/^[a-f0-9]{40}$/.test(snapshot?.head?.sha) ||
    !snapshot.head.repo?.id ||
    !snapshot.head.repo.full_name ||
    !snapshot.head.ref ||
    !Number.isSafeInteger(context.runId) ||
    context.runId <= 0
  ) {
    throw new Error('Invalid docs preview approval event');
  }
  previewUrl(snapshot.number);
  const run = {
    head_sha: snapshot.head.sha,
    head_branch: snapshot.head.ref,
    head_repository: snapshot.head.repo,
    actor: { login: context.actor },
  };
  if (!matchesPreview(snapshot, run) || !(await requestedByMaintainer({ github, context }, run))) {
    throw new Error('A maintainer must apply docs-preview to an open fork PR');
  }
  const { data: current } = await github.rest.pulls.get({
    ...context.repo,
    pull_number: snapshot.number,
  });
  if (!matchesPreview(current, run)) {
    throw new Error('The PR changed after the label event; review it and reapply docs-preview');
  }
  await github.rest.repos.createCommitStatus({
    ...context.repo,
    sha: snapshot.head.sha,
    context: `docs-preview/pr-${snapshot.number}`,
    state: 'success',
    description: 'Maintainer approved this commit for a docs preview',
    target_url: `${approvalRunPrefix}${context.runId}`,
  });
  core.info(`Recorded docs preview approval for PR #${snapshot.number} at ${snapshot.head.sha}.`);
}

async function previewApprovalState({ github, context }, number, sha) {
  const statuses = await github.paginate(github.rest.repos.listCommitStatusesForRef, {
    ...context.repo,
    ref: sha,
    per_page: 100,
  });
  // GitHub returns statuses newest first. A label on an older commit is not
  // permission to publish this one, even when a maintainer triggers its run.
  const status = statuses.find((entry) => entry.context === `docs-preview/pr-${number}`);
  if (!status) {
    return 'pending';
  }
  const target = status.target_url;
  if (
    status.state !== 'success' ||
    status.creator?.login !== 'github-actions[bot]' ||
    !target?.startsWith(approvalRunPrefix)
  ) {
    return 'denied';
  }
  const id = target.slice(approvalRunPrefix.length);
  if (!/^[1-9]\d*$/.test(id) || !Number.isSafeInteger(Number(id))) {
    return 'denied';
  }
  const { data: approval } = await github.rest.actions.getWorkflowRun({
    ...context.repo,
    run_id: Number(id),
  });
  // The status is only an index. Verify its proof against a trusted workflow
  // run so copying a status or a run URL cannot approve a different PR/SHA.
  if (
    approval.id !== Number(id) ||
    approval.repository?.full_name !== repository ||
    approval.path !== approvalWorkflow ||
    approval.event !== 'pull_request_target' ||
    approval.display_title !== `Approve docs preview for PR #${number} at ${sha} (${previewLabel})`
  ) {
    return 'denied';
  }
  if (approval.status !== 'completed') {
    return 'pending';
  }
  return approval.conclusion === 'success' ? 'approved' : 'denied';
}

export async function authorizePreview({ github, context, core, sleep = setTimeout }) {
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
  if (!(await requestedByMaintainer({ github, context }, run))) {
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
  const matches = artifacts.filter((a) => a.name === 'docs-fork-preview' && !a.expired);
  if (matches.length !== 1) {
    throw new Error('Expected one active docs-fork-preview artifact from the triggering run');
  }
  // The build and trusted label handler start independently. Allow the small
  // metadata-only approval job to finish, but never deploy without its proof.
  let approval;
  for (let attempt = 0; attempt < 24; attempt++) {
    approval = await previewApprovalState({ github, context }, candidates[0].number, run.head_sha);
    if (approval !== 'pending') {
      break;
    }
    if (attempt < 23) {
      await sleep(5000);
    }
  }
  if (approval !== 'approved') {
    core.info(
      'No successful trusted approval for this PR commit; remove and reapply docs-preview.',
    );
    return;
  }
  // Approval polling can outlive the PR state checked above. Do not send a
  // stale or revoked request to the deployment queue.
  if (!(await isCurrentPreview({ github, context }, candidates[0].number))) {
    core.info('The PR changed or preview permission was revoked; skipping the deployment queue.');
    return;
  }
  core.setOutput('pr', candidates[0].number);
  core.setOutput('artifact-id', matches[0].id);
  core.setOutput('preview-url', previewUrl(candidates[0].number));
}

export async function isCurrentPreview({ github, context }, number) {
  previewUrl(number);
  const run = previewRun(context);
  const { data: pr } = await github.rest.pulls.get({ ...context.repo, pull_number: number });
  return (
    matchesPreview(pr, run) &&
    (await requestedByMaintainer({ github, context }, run)) &&
    (await previewApprovalState({ github, context }, number, run.head_sha)) === 'approved'
  );
}

function uploadedPreviewUrl(output) {
  // WRANGLER_OUTPUT_FILE_PATH contains JSONL, not console output. Require one
  // upload from this job and its version URL; never substitute the moving alias.
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
      `https://${upload.version_id.slice(0, 8)}-viteplus-dev.voidzero-docs.workers.dev`
  ) {
    throw new Error(
      'Invalid version preview URL from Wrangler; check that Preview URLs are enabled',
    );
  }
  return upload.preview_url;
}

export async function commentPreview({ github, context, core }, number, output) {
  if (!(await isCurrentPreview({ github, context }, number))) {
    core.info('The PR changed or preview permission was revoked; skipping the preview comment.');
    return;
  }
  const body = `${marker}\nCloudflare documentation preview: ${uploadedPreviewUrl(output)}\n\nCommit: ${context.payload.workflow_run.head_sha}\n\nLatest uploaded preview (may show another commit): ${previewUrl(number)}`;
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
