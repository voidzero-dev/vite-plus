// Run with node --test; these workflow helpers need no workspace dependencies.
import assert from 'node:assert/strict';
import { mkdir, mkdtemp, readFile, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';

import {
  approvePreview,
  authorizePreview,
  commentPreview,
  isCurrentPreview,
  previewUrl,
  validateAssets,
} from '../docs-fork-preview.mjs';

const versionId = '11111111-1111-4111-8111-111111111111';
const versionUrl = 'https://11111111-viteplus-dev.voidzero-docs.workers.dev';

function grantApproval(state, sha = 'a'.repeat(40), number = 2684, runId = 987) {
  const status = {
    context: `docs-preview/pr-${number}`,
    state: 'success',
    creator: { login: 'github-actions[bot]' },
    target_url: `https://github.com/voidzero-dev/vite-plus/actions/runs/${runId}`,
  };
  const approval = {
    id: runId,
    repository: { full_name: 'voidzero-dev/vite-plus' },
    path: '.github/workflows/approve-docs-fork-preview.yml',
    event: 'pull_request_target',
    display_title: `Approve docs preview for PR #${number} at ${sha} (docs-preview)`,
    status: 'completed',
    conclusion: 'success',
  };
  state.statuses.set(sha, [status]);
  state.approvals.set(runId, approval);
  return { status, approval };
}

function uploadOutput(overrides = {}) {
  return `${JSON.stringify({
    type: 'version-upload',
    version: 1,
    worker_name: 'viteplus-dev',
    version_id: versionId,
    preview_url: versionUrl,
    preview_alias_url: previewUrl(2684),
    ...overrides,
  })}\n`;
}

function fixture() {
  const source = { id: 42, full_name: 'contributor/vite-plus', owner: { login: 'contributor' } };
  const context = {
    repo: { owner: 'voidzero-dev', repo: 'vite-plus' },
    payload: {
      workflow_run: {
        id: 123,
        path: '.github/workflows/build-docs-fork-preview.yml',
        event: 'pull_request',
        conclusion: 'success',
        head_sha: 'a'.repeat(40),
        head_branch: 'docs-update',
        head_repository: source,
        actor: { login: 'maintainer' },
        pull_requests: [],
      },
    },
  };
  const pr = {
    number: 2684,
    state: 'open',
    labels: [{ name: 'docs-preview' }],
    base: { ref: 'main', repo: { full_name: 'voidzero-dev/vite-plus' } },
    head: { sha: 'a'.repeat(40), ref: 'docs-update', repo: structuredClone(source) },
  };
  const state = {
    pulls: [pr],
    artifacts: [{ id: 456, name: 'docs-fork-preview', expired: false }],
    comments: [],
    outputs: {},
    writes: [],
    requests: [],
    permission: 'write',
    statuses: new Map(),
    approvals: new Map(),
    sleeps: [],
  };
  const github = {
    rest: {
      repos: {
        listCommitStatusesForRef() {},
        createCommitStatus: async (params) => state.writes.push({ method: 'status', ...params }),
        getCollaboratorPermissionLevel: async (params) => {
          state.requests.push(params);
          return { data: { permission: state.permission } };
        },
      },
      pulls: {
        list() {},
        get: async (params) => {
          state.requests.push(params);
          return { data: structuredClone(pr) };
        },
      },
      actions: {
        listWorkflowRunArtifacts() {},
        getWorkflowRun: async (params) => {
          state.requests.push(params);
          const approval = state.approvals.get(params.run_id);
          if (!approval) {
            throw new Error('Approval workflow run not found');
          }
          return { data: approval };
        },
      },
      issues: {
        listComments() {},
        createComment: async (params) => state.writes.push({ method: 'create', ...params }),
        updateComment: async (params) => state.writes.push({ method: 'update', ...params }),
      },
    },
    paginate: async (method, params) => {
      state.requests.push(params);
      if (method === github.rest.pulls.list) {
        return structuredClone(state.pulls);
      }
      if (method === github.rest.actions.listWorkflowRunArtifacts) {
        return state.artifacts;
      }
      if (method === github.rest.issues.listComments) {
        return state.comments;
      }
      if (method === github.rest.repos.listCommitStatusesForRef) {
        return state.statuses.get(params.ref) ?? [];
      }
      throw new Error('Unexpected GitHub request');
    },
  };
  const core = {
    info() {},
    setOutput: (key, value) => {
      state.outputs[key] = value;
    },
  };
  const sleep = async (ms) => state.sleeps.push(ms);
  return { github, context, core, pr, state, sleep, ...grantApproval(state) };
}

function approvalFixture() {
  const f = fixture();
  f.context.eventName = 'pull_request_target';
  f.context.actor = 'maintainer';
  f.context.runId = 987;
  f.context.payload = {
    action: 'labeled',
    label: { name: 'docs-preview' },
    pull_request: structuredClone(f.pr),
  };
  return f;
}

await test('records approval for the immutable label event commit', async () => {
  const f = approvalFixture();
  await approvePreview(f);
  assert.deepEqual(f.state.writes, [
    {
      method: 'status',
      owner: 'voidzero-dev',
      repo: 'vite-plus',
      sha: 'a'.repeat(40),
      context: 'docs-preview/pr-2684',
      state: 'success',
      description: 'Maintainer approved this commit for a docs preview',
      target_url: 'https://github.com/voidzero-dev/vite-plus/actions/runs/987',
    },
  ]);
});

for (const { name, mutate } of [
  { name: 'another repository', mutate: (c) => (c.repo.owner = 'contributor') },
  { name: 'an untrusted trigger', mutate: (c) => (c.eventName = 'pull_request') },
  { name: 'a push', mutate: (c) => (c.payload.action = 'synchronize') },
  { name: 'label removal', mutate: (c) => (c.payload.action = 'unlabeled') },
  { name: 'an unrelated label', mutate: (c) => (c.payload.label.name = 'bug') },
  { name: 'a missing label', mutate: (c) => (c.payload.label = undefined) },
  { name: 'a missing PR', mutate: (c) => (c.payload.pull_request = undefined) },
  { name: 'an invalid SHA', mutate: (c) => (c.payload.pull_request.head.sha = 'invalid') },
  { name: 'a deleted fork', mutate: (c) => (c.payload.pull_request.head.repo = null) },
  { name: 'a missing fork ID', mutate: (c) => (c.payload.pull_request.head.repo.id = undefined) },
  { name: 'a missing fork name', mutate: (c) => (c.payload.pull_request.head.repo.full_name = '') },
  { name: 'a missing branch', mutate: (c) => (c.payload.pull_request.head.ref = '') },
  { name: 'an invalid run ID', mutate: (c) => (c.runId = '987') },
  { name: 'a zero run ID', mutate: (c) => (c.runId = 0) },
  { name: 'an unsafe run ID', mutate: (c) => (c.runId = Number.MAX_SAFE_INTEGER + 1) },
  { name: 'an invalid PR number', mutate: (c) => (c.payload.pull_request.number = '2684') },
]) {
  await test(`does not record approval for ${name}`, async () => {
    const f = approvalFixture();
    mutate(f.context);
    await assert.rejects(approvePreview(f), /Invalid/);
    assert.deepEqual(f.state.writes, []);
  });
}

for (const { name, mutate } of [
  { name: 'a closed PR', mutate: (f) => (f.context.payload.pull_request.state = 'closed') },
  { name: 'a missing preview label', mutate: (f) => (f.context.payload.pull_request.labels = []) },
  {
    name: 'a same-repository PR',
    mutate: (f) => (f.context.payload.pull_request.head.repo.full_name = 'voidzero-dev/vite-plus'),
  },
  {
    name: 'another base branch',
    mutate: (f) => (f.context.payload.pull_request.base.ref = 'release'),
  },
  { name: 'a missing actor', mutate: (f) => (f.context.actor = undefined) },
  ...['read', 'triage', 'none', undefined].map((permission) => ({
    name: `${permission} permission`,
    mutate: (f) => (f.state.permission = permission),
  })),
]) {
  await test(`does not record approval with ${name}`, async () => {
    const f = approvalFixture();
    mutate(f);
    await assert.rejects(approvePreview(f), /A maintainer must apply/);
    assert.deepEqual(f.state.writes, []);
  });
}

for (const { name, mutate } of [
  { name: 'a new commit', mutate: (pr) => (pr.head.sha = 'b'.repeat(40)) },
  { name: 'label removal', mutate: (pr) => (pr.labels = []) },
  { name: 'PR closure', mutate: (pr) => (pr.state = 'closed') },
  { name: 'a base change', mutate: (pr) => (pr.base.ref = 'release') },
]) {
  await test(`does not record approval after ${name} during label handling`, async () => {
    const f = approvalFixture();
    mutate(f.pr);
    await assert.rejects(approvePreview(f), /The PR changed after the label event/);
    assert.deepEqual(f.state.writes, []);
  });
}

await test('never transfers approval to a commit pushed during the status write', async () => {
  const f = approvalFixture();
  const create = f.github.rest.repos.createCommitStatus;
  f.github.rest.repos.createCommitStatus = async (params) => {
    f.pr.head.sha = 'b'.repeat(40);
    await create(params);
  };
  await approvePreview(f);
  assert.equal(f.state.writes[0].sha, 'a'.repeat(40));
});

for (const [area, method] of [
  ['repos', 'getCollaboratorPermissionLevel'],
  ['pulls', 'get'],
  ['repos', 'createCommitStatus'],
]) {
  await test(`fails closed when approval ${method} fails`, async () => {
    const f = approvalFixture();
    f.github.rest[area][method] = async () => {
      throw new Error('GitHub API failed');
    };
    await assert.rejects(approvePreview(f), /GitHub API failed/);
    assert.deepEqual(f.state.writes, []);
  });
}

await test('keeps the trusted workflow run name aligned with the approval proof', async () => {
  const yaml = await readFile(
    new URL('../../workflows/approve-docs-fork-preview.yml', import.meta.url),
    'utf8',
  );
  const template = yaml.match(/^run-name: '(.+)'$/m)?.[1];
  assert.ok(template);
  const f = fixture();
  f.approval.display_title = template
    .replaceAll('${{ github.event.pull_request.number }}', '2684')
    .replaceAll('${{ github.event.pull_request.head.sha }}', f.pr.head.sha)
    .replaceAll('${{ github.event.label.name }}', 'docs-preview');
  await authorizePreview(f);
  assert.equal(f.state.outputs.pr, 2684);
});

await test('isolates build concurrency by PR and SHA, including delayed old runs', async () => {
  const yaml = await readFile(
    new URL('../../workflows/build-docs-fork-preview.yml', import.meta.url),
    'utf8',
  );
  const template = yaml.match(/concurrency:\n\s+group: (.+)\n\s+cancel-in-progress: true/)?.[1];
  assert.ok(template);
  const group = (number, sha) =>
    template
      .replaceAll('${{ github.event.pull_request.number }}', String(number))
      .replaceAll('${{ github.event.pull_request.head.sha }}', sha);
  const newer = group(2684, 'b'.repeat(40));
  const delayed = group(2684, 'a'.repeat(40));
  assert.notEqual(delayed, newer);
  assert.notEqual(group(2685, 'b'.repeat(40)), newer);
  assert.equal(group(2684, 'b'.repeat(40)), newer);
});

await test('queues pending deployments without replacing them when an old run arrives late', async () => {
  const yaml = await readFile(
    new URL('../../workflows/deploy-docs-fork-preview.yml', import.meta.url),
    'utf8',
  );
  assert.match(
    yaml,
    /concurrency:\n\s+group: deploy-docs-fork-preview-\$\{\{ needs\.authorize\.outputs\.pr \}\}\n\s+queue: max\n\s+cancel-in-progress: false/,
  );
});

await test('authorizes a fork with an empty workflow_run PR list and pins its artifact', async () => {
  const f = fixture();
  await authorizePreview(f);
  assert.deepEqual(f.state.requests, [
    {
      owner: 'voidzero-dev',
      repo: 'vite-plus',
      state: 'open',
      base: 'main',
      head: 'contributor:docs-update',
    },
    { owner: 'voidzero-dev', repo: 'vite-plus', username: 'maintainer' },
    { owner: 'voidzero-dev', repo: 'vite-plus', run_id: 123 },
    { owner: 'voidzero-dev', repo: 'vite-plus', ref: 'a'.repeat(40), per_page: 100 },
    { owner: 'voidzero-dev', repo: 'vite-plus', run_id: 987 },
    { owner: 'voidzero-dev', repo: 'vite-plus', pull_number: 2684 },
    { owner: 'voidzero-dev', repo: 'vite-plus', username: 'maintainer' },
    { owner: 'voidzero-dev', repo: 'vite-plus', ref: 'a'.repeat(40), per_page: 100 },
    { owner: 'voidzero-dev', repo: 'vite-plus', run_id: 987 },
  ]);
  assert.deepEqual(f.state.outputs, {
    pr: 2684,
    'artifact-id': 456,
    'preview-url': 'https://pr-2684-viteplus-dev.voidzero-docs.workers.dev',
  });
});

await test('does not approve a new commit when a maintainer applies an unrelated label', async () => {
  const f = fixture();
  await authorizePreview(f);
  assert.equal(f.state.outputs.pr, 2684);

  // The fork changes its workflow to build on any label. A maintainer applies
  // bug to B while docs-preview remains from A: actor and label checks pass.
  f.pr.head.sha = 'b'.repeat(40);
  f.context.payload.workflow_run.head_sha = f.pr.head.sha;
  f.state.outputs = {};
  await authorizePreview(f);
  assert.deepEqual(f.state.outputs, {});
  assert.equal(await isCurrentPreview(f, 2684), false);
  await commentPreview(f, 2684, uploadOutput());
  assert.deepEqual(f.state.writes, []);

  // Only a new trusted approval for B permits deployment.
  grantApproval(f.state, f.pr.head.sha, f.pr.number, 988);
  await authorizePreview(f);
  assert.equal(f.state.outputs.pr, 2684);
});

for (const { name, mutate } of [
  { name: 'missing status', mutate: (f) => f.state.statuses.clear() },
  { name: 'another PR status', mutate: (f) => (f.status.context = 'docs-preview/pr-2685') },
  { name: 'failed status', mutate: (f) => (f.status.state = 'failure') },
  { name: 'pending status', mutate: (f) => (f.status.state = 'pending') },
  { name: 'manual status', mutate: (f) => (f.status.creator.login = 'contributor') },
  { name: 'missing creator', mutate: (f) => (f.status.creator = null) },
  { name: 'missing URL', mutate: (f) => (f.status.target_url = null) },
  { name: 'external URL', mutate: (f) => (f.status.target_url = 'https://example.com/987') },
  {
    name: 'fork run URL',
    mutate: (f) =>
      (f.status.target_url = 'https://github.com/contributor/vite-plus/actions/runs/987'),
  },
  ...['0', '-987', '0987', '987/attempts/1', '987?other=true', '9007199254740992'].map((id) => ({
    name: `invalid run URL ${id}`,
    mutate: (f) =>
      (f.status.target_url = `https://github.com/voidzero-dev/vite-plus/actions/runs/${id}`),
  })),
  { name: 'mismatched proof ID', mutate: (f) => (f.approval.id = 988) },
  {
    name: 'fork proof',
    mutate: (f) => (f.approval.repository.full_name = 'contributor/vite-plus'),
  },
  { name: 'missing proof repository', mutate: (f) => (f.approval.repository = null) },
  { name: 'untrusted workflow', mutate: (f) => (f.approval.path = '.github/workflows/spoof.yml') },
  { name: 'PR-controlled proof', mutate: (f) => (f.approval.event = 'pull_request') },
  { name: 'manual proof', mutate: (f) => (f.approval.event = 'workflow_dispatch') },
  {
    name: 'proof for another PR',
    mutate: (f) => (f.approval.display_title = f.approval.display_title.replace('#2684', '#2685')),
  },
  {
    name: 'proof for another SHA',
    mutate: (f) =>
      (f.approval.display_title = f.approval.display_title.replace('a'.repeat(40), 'b'.repeat(40))),
  },
  {
    name: 'proof for another label',
    mutate: (f) =>
      (f.approval.display_title = f.approval.display_title.replace('(docs-preview)', '(bug)')),
  },
  { name: 'missing proof title', mutate: (f) => (f.approval.display_title = undefined) },
  ...['failure', 'cancelled', 'skipped', null].map((conclusion) => ({
    name: `${conclusion} proof`,
    mutate: (f) => (f.approval.conclusion = conclusion),
  })),
]) {
  await test(`rejects ${name} during authorization and rechecks`, async () => {
    const f = fixture();
    mutate(f);
    await authorizePreview(f);
    assert.deepEqual(f.state.outputs, {});
    assert.equal(await isCurrentPreview(f, 2684), false);
    await commentPreview(f, 2684, uploadOutput());
    assert.deepEqual(f.state.writes, []);
  });
}

await test('does not accept an old proof URL copied into a new commit status', async () => {
  const f = fixture();
  f.pr.head.sha = 'b'.repeat(40);
  f.context.payload.workflow_run.head_sha = f.pr.head.sha;
  f.state.statuses.set(f.pr.head.sha, [structuredClone(f.status)]);
  await authorizePreview(f);
  assert.deepEqual(f.state.outputs, {});
  assert.equal(await isCurrentPreview(f, 2684), false);
});

await test('uses the latest status instead of an older successful approval', async () => {
  const f = fixture();
  f.state.statuses.get(f.pr.head.sha).unshift({ ...f.status, state: 'failure' });
  await authorizePreview(f);
  assert.deepEqual(f.state.outputs, {});
  assert.deepEqual(f.state.sleeps, []);
});

for (const pending of ['missing status', 'running workflow']) {
  await test(`waits for the trusted approval when there is a ${pending}`, async () => {
    const f = fixture();
    if (pending === 'missing status') {
      f.state.statuses.clear();
    } else {
      f.approval.status = 'in_progress';
      f.approval.conclusion = null;
    }
    f.sleep = async (ms) => {
      f.state.sleeps.push(ms);
      grantApproval(f.state);
    };
    await authorizePreview(f);
    assert.equal(f.state.outputs.pr, 2684);
    assert.deepEqual(f.state.sleeps, [5000]);
  });

  await test(`stops waiting after a bounded interval for a ${pending}`, async () => {
    const f = fixture();
    if (pending === 'missing status') {
      f.state.statuses.clear();
    } else {
      f.approval.status = 'in_progress';
      f.approval.conclusion = null;
    }
    await authorizePreview(f);
    assert.deepEqual(f.state.outputs, {});
    assert.deepEqual(f.state.sleeps, Array(23).fill(5000));
    assert.equal(await isCurrentPreview(f, 2684), false);
  });
}

await test('does not enqueue an older authorization that finishes after a newer preview', async () => {
  const older = fixture();
  older.approval.status = 'in_progress';
  older.approval.conclusion = null;
  const newerOutputs = {};
  const newer = {
    ...older,
    context: structuredClone(older.context),
    core: {
      info() {},
      setOutput: (key, value) => (newerOutputs[key] = value),
    },
  };
  newer.context.payload.workflow_run.id = 124;
  newer.context.payload.workflow_run.head_sha = 'b'.repeat(40);
  older.sleep = async (ms) => {
    older.state.sleeps.push(ms);
    older.pr.head.sha = newer.context.payload.workflow_run.head_sha;
    grantApproval(older.state, older.pr.head.sha, older.pr.number, 988);
    await authorizePreview(newer);
    older.approval.status = 'completed';
    older.approval.conclusion = 'success';
  };

  await authorizePreview(older);

  assert.equal(newerOutputs.pr, 2684);
  assert.deepEqual(older.state.sleeps, [5000]);
  assert.deepEqual(older.state.outputs, {});
  assert.equal(await isCurrentPreview(newer, 2684), true);
});

for (const { name, mutate } of [
  { name: 'label removal', mutate: (f) => (f.pr.labels = []) },
  { name: 'PR closure', mutate: (f) => (f.pr.state = 'closed') },
  { name: 'a base change', mutate: (f) => (f.pr.base.ref = 'release') },
  { name: 'requester permission removal', mutate: (f) => (f.state.permission = 'read') },
]) {
  await test(`rechecks ${name} after approval polling and before queueing`, async () => {
    const f = fixture();
    f.approval.status = 'in_progress';
    f.approval.conclusion = null;
    f.sleep = async () => {
      grantApproval(f.state);
      mutate(f);
    };
    await authorizePreview(f);
    assert.deepEqual(f.state.outputs, {});
  });
}

await test('fails closed when the PR recheck before queueing fails', async () => {
  const f = fixture();
  f.github.rest.pulls.get = async () => {
    throw new Error('GitHub PR lookup failed');
  };
  await assert.rejects(authorizePreview(f), /GitHub PR lookup failed/);
  assert.deepEqual(f.state.outputs, {});
});

for (const api of ['statuses', 'workflow proof']) {
  await test(`fails closed when the ${api} lookup fails`, async () => {
    const f = fixture();
    if (api === 'statuses') {
      const paginate = f.github.paginate;
      f.github.paginate = async (method, params) => {
        if (method === f.github.rest.repos.listCommitStatusesForRef) {
          throw new Error('GitHub API failed');
        }
        return paginate(method, params);
      };
    } else {
      f.github.rest.actions.getWorkflowRun = async () => {
        throw new Error('GitHub API failed');
      };
    }
    await assert.rejects(authorizePreview(f), /GitHub API failed/);
    assert.deepEqual(f.state.outputs, {});
    await assert.rejects(isCurrentPreview(f, 2684), /GitHub API failed/);
  });
}

await test('rechecks approval revocation after authorization', async () => {
  const f = fixture();
  await authorizePreview(f);
  assert.equal(f.state.outputs.pr, 2684);
  f.status.state = 'failure';
  assert.equal(await isCurrentPreview(f, 2684), false);
  await commentPreview(f, 2684, uploadOutput());
  assert.deepEqual(f.state.writes, []);
});

for (const [field, value] of [
  ['path', '.github/workflows/spoof.yml'],
  ['event', 'push'],
  ['conclusion', 'failure'],
  ['head_sha', 'invalid'],
  ['head_repository', null],
  ['head_branch', ''],
]) {
  await test(`rejects a run with invalid ${field}`, async () => {
    const f = fixture();
    f.context.payload.workflow_run[field] = value;
    await assert.rejects(authorizePreview(f), /Invalid docs preview workflow run/);
    assert.deepEqual(f.state.outputs, {});
  });
}

await test('rejects a workflow running in another repository', async () => {
  const f = fixture();
  f.context.repo.owner = 'contributor';
  await assert.rejects(authorizePreview(f), /Invalid docs preview workflow run/);
});

await test('leaves same-repository previews to the existing integration', async () => {
  const f = fixture();
  f.context.payload.workflow_run.head_repository.full_name = 'voidzero-dev/vite-plus';
  await authorizePreview(f);
  assert.deepEqual(f.state.requests, []);
  assert.deepEqual(f.state.outputs, {});
});

for (const { name, mutate } of [
  { name: 'closed', mutate: (pr) => (pr.state = 'closed') },
  { name: 'no preview label', mutate: (pr) => (pr.labels = []) },
  { name: 'missing labels', mutate: (pr) => (pr.labels = undefined) },
  { name: 'unrelated label', mutate: (pr) => (pr.labels = [{ name: 'preview-build' }]) },
  { name: 'stale commit', mutate: (pr) => (pr.head.sha = 'b'.repeat(40)) },
  { name: 'other source repository', mutate: (pr) => (pr.head.repo.id = 99) },
  { name: 'renamed source repository', mutate: (pr) => (pr.head.repo.full_name = 'someone/other') },
  { name: 'other source branch', mutate: (pr) => (pr.head.ref = 'other') },
  { name: 'other base branch', mutate: (pr) => (pr.base.ref = 'release') },
  { name: 'other base repository', mutate: (pr) => (pr.base.repo.full_name = 'someone/other') },
  { name: 'deleted fork', mutate: (pr) => (pr.head.repo = null) },
]) {
  await test(`skips a PR with ${name}, including the check immediately before upload`, async () => {
    const f = fixture();
    mutate(f.pr);
    await authorizePreview(f);
    assert.deepEqual(f.state.outputs, {});
    assert.equal(await isCurrentPreview(f, 2684), false);
  });
}

await test('rejects ambiguous PR matches', async () => {
  const f = fixture();
  f.state.pulls.push({ ...f.pr, number: 2685 });
  await assert.rejects(authorizePreview(f), /More than one PR/);
});

for (const permission of ['admin', 'maintain', 'write']) {
  await test(`accepts a labeled preview requested with ${permission} permission`, async () => {
    const f = fixture();
    f.state.permission = permission;
    await authorizePreview(f);
    assert.equal(f.state.outputs.pr, 2684);
    assert.equal(await isCurrentPreview(f, 2684), true);
  });
}

for (const permission of ['read', 'triage', 'none', undefined]) {
  await test(`rejects an original run actor with ${permission} permission`, async () => {
    const f = fixture();
    f.state.permission = permission;
    await authorizePreview(f);
    assert.deepEqual(f.state.outputs, {});
    assert.equal(await isCurrentPreview(f, 2684), false);
    assert.ok(f.state.requests.every((request) => !('run_id' in request)));
  });
}

await test('rejects a missing original run actor', async () => {
  const f = fixture();
  delete f.context.payload.workflow_run.actor;
  await authorizePreview(f);
  assert.deepEqual(f.state.outputs, {});
  assert.equal(await isCurrentPreview(f, 2684), false);
});

await test('does not authorize an outsider run when a maintainer reruns it', async () => {
  const f = fixture();
  f.context.payload.workflow_run.actor = { login: 'contributor' };
  f.context.payload.workflow_run.triggering_actor = { login: 'maintainer' };
  f.state.permission = 'read';
  await authorizePreview(f);
  assert.deepEqual(f.state.outputs, {});
  assert.ok(f.state.requests.some((request) => request.username === 'contributor'));
  assert.ok(f.state.requests.every((request) => request.username !== 'maintainer'));
});

await test('fails closed when the requester permission check fails', async () => {
  const f = fixture();
  f.github.rest.repos.getCollaboratorPermissionLevel = async () => {
    throw new Error('GitHub permission check failed');
  };
  await assert.rejects(authorizePreview(f), /GitHub permission check failed/);
  assert.deepEqual(f.state.outputs, {});
  await assert.rejects(isCurrentPreview(f, 2684), /GitHub permission check failed/);
});

await test('rechecks label removal after authorization and before commenting', async () => {
  const f = fixture();
  await authorizePreview(f);
  assert.equal(f.state.outputs.pr, 2684);
  f.pr.labels = [];
  assert.equal(await isCurrentPreview(f, 2684), false);
  await commentPreview(f, 2684, uploadOutput());
  assert.deepEqual(f.state.writes, []);
});

await test('rechecks revoked requester permission after authorization', async () => {
  const f = fixture();
  await authorizePreview(f);
  assert.equal(f.state.outputs.pr, 2684);
  f.state.permission = 'read';
  assert.equal(await isCurrentPreview(f, 2684), false);
  await commentPreview(f, 2684, uploadOutput());
  assert.deepEqual(f.state.writes, []);
});

await test('does not reuse an approved run for a new commit while the label remains', async () => {
  const f = fixture();
  await authorizePreview(f);
  f.pr.head.sha = 'b'.repeat(40);
  assert.equal(await isCurrentPreview(f, 2684), false);

  // Even if a fork changes its workflow to run on pushes, the new run does
  // not inherit permission from the actor of the earlier labeled run.
  f.context.payload.workflow_run.head_sha = f.pr.head.sha;
  f.context.payload.workflow_run.actor = { login: 'contributor' };
  f.state.permission = 'read';
  f.state.outputs = {};
  await authorizePreview(f);
  assert.deepEqual(f.state.outputs, {});
});

await test('skips successful helper-only or unrelated label runs without artifacts', async () => {
  const f = fixture();
  f.state.artifacts = [];
  await authorizePreview(f);
  assert.deepEqual(f.state.outputs, {});
});

for (const artifacts of [
  [{ id: 456, name: 'docs-fork-preview', expired: true }],
  [{ id: 456, name: 'other', expired: false }],
  [
    { id: 456, name: 'docs-fork-preview', expired: false },
    { id: 789, name: 'docs-fork-preview', expired: false },
  ],
]) {
  await test(`rejects missing, expired, or ambiguous artifacts: ${JSON.stringify(artifacts)}`, async () => {
    const f = fixture();
    f.state.artifacts = artifacts;
    await assert.rejects(authorizePreview(f), /Expected one active/);
    assert.deepEqual(f.state.outputs, {});
  });
}

await test('ignores a contributor comment that copies the bot marker', async () => {
  const f = fixture();
  f.state.comments.push({
    id: 100,
    user: { login: 'contributor' },
    body: '<!-- cloudflare-docs-fork-preview -->',
  });
  await commentPreview(f, 2684, uploadOutput());
  assert.equal(f.state.writes[0].method, 'create');
  assert.equal(f.state.writes[0].issue_number, 2684);
  assert.equal(
    f.state.writes[0].body,
    `<!-- cloudflare-docs-fork-preview -->\nCloudflare documentation preview: ${versionUrl}\n\nCommit: ${'a'.repeat(40)}\n\nLatest uploaded preview (may show another commit): ${previewUrl(2684)}`,
  );
});

await test('updates the existing bot comment', async () => {
  const f = fixture();
  f.state.comments.push({
    id: 101,
    user: { login: 'github-actions[bot]' },
    body: '<!-- cloudflare-docs-fork-preview -->\nPrevious preview',
  });
  await commentPreview(f, 2684, uploadOutput());
  assert.equal(f.state.writes[0].method, 'update');
  assert.equal(f.state.writes[0].comment_id, 101);
  assert.ok(f.state.writes[0].body.includes(versionUrl));
});

await test('does not comment if the PR changes during upload', async () => {
  const f = fixture();
  f.pr.head.sha = 'b'.repeat(40);
  await commentPreview(f, 2684, uploadOutput());
  assert.deepEqual(f.state.writes, []);
});

await test('keeps the previous comment tied to its version when the PR changes during upload', async () => {
  const f = fixture();
  await commentPreview(f, 2684, uploadOutput());
  const previousBody = f.state.writes[0].body;
  f.state.comments.push({
    id: 101,
    user: { login: 'github-actions[bot]' },
    body: previousBody,
  });
  f.state.writes = [];

  // B passes the pre-upload check, then C arrives while B moves the PR alias.
  f.context.payload.workflow_run.head_sha = 'b'.repeat(40);
  f.pr.head.sha = 'b'.repeat(40);
  grantApproval(f.state, f.pr.head.sha, f.pr.number, 988);
  assert.equal(await isCurrentPreview(f, 2684), true);
  f.pr.head.sha = 'c'.repeat(40);
  await commentPreview(
    f,
    2684,
    uploadOutput({
      version_id: '22222222-2222-4222-8222-222222222222',
      preview_url: 'https://22222222-viteplus-dev.voidzero-docs.workers.dev',
    }),
  );

  assert.deepEqual(f.state.writes, []);
  assert.equal(f.state.comments[0].body, previousBody);
  assert.ok(previousBody.includes(`Cloudflare documentation preview: ${versionUrl}`));
  assert.ok(previousBody.includes(`Commit: ${'a'.repeat(40)}`));
});

await test('reads the version URL from Wrangler JSONL with other records and blank lines', async () => {
  const f = fixture();
  await commentPreview(f, 2684, `\n${JSON.stringify({ type: 'other' })}\n${uploadOutput()}\n`);
  assert.ok(f.state.writes[0].body.includes(versionUrl));
});

for (const [name, output] of [
  ['missing upload', ''],
  ['duplicate uploads', uploadOutput() + uploadOutput()],
  ['invalid JSON', '{'],
  ['unsupported output version', uploadOutput({ version: 2 })],
  ['another Worker', uploadOutput({ worker_name: 'other' })],
  ['invalid version ID', uploadOutput({ version_id: 'invalid' })],
  ['disabled preview URLs', uploadOutput({ preview_url: undefined })],
  ['moving alias', uploadOutput({ preview_url: previewUrl(2684) })],
  [
    'another version URL',
    uploadOutput({ preview_url: versionUrl.replace('11111111', '22222222') }),
  ],
  ['another host', uploadOutput({ preview_url: 'https://example.com' })],
]) {
  await test(`does not comment for Wrangler output with ${name}`, async () => {
    const f = fixture();
    await assert.rejects(commentPreview(f, 2684, output));
    assert.deepEqual(f.state.writes, []);
  });
}

await test('rejects invalid PR numbers before using them in URLs or requests', async () => {
  for (const number of [0, -1, 1.5, NaN, '2684', '2684\nother-output=true']) {
    assert.throws(() => previewUrl(number), /Invalid pull request number/);
    await assert.rejects(isCurrentPreview(fixture(), number), /Invalid pull request number/);
  }
});

await test('accepts a static site and rejects links outside the artifact', async (t) => {
  const directory = await mkdtemp(join(tmpdir(), 'docs-preview-test-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  await writeFile(join(directory, 'index.html'), '<!doctype html><title>Preview</title>');
  await mkdir(join(directory, 'assets'));
  await writeFile(join(directory, 'assets', 'app.js'), 'window.preview = true;');
  await validateAssets(directory);
  await symlink(join(directory, 'index.html'), join(directory, 'assets', 'link'));
  await assert.rejects(validateAssets(directory), /must be regular files/);
});

await test('rejects an artifact without a site index', async (t) => {
  const directory = await mkdtemp(join(tmpdir(), 'docs-preview-test-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  await assert.rejects(validateAssets(directory), /ENOENT/);
});
