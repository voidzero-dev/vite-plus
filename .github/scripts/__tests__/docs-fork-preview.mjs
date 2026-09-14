// Run with node --test; these workflow helpers need no workspace dependencies.
import assert from 'node:assert/strict';
import { mkdir, mkdtemp, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';

import {
  authorizePreview,
  commentPreview,
  isCurrentPreview,
  previewUrl,
  validateAssets,
} from '../docs-fork-preview.mjs';

const versionId = '11111111-1111-4111-8111-111111111111';
const versionUrl = 'https://11111111-viteplus-dev.voidzero-docs.workers.dev';

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
        pull_requests: [],
      },
    },
  };
  const pr = {
    number: 2684,
    state: 'open',
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
  };
  const github = {
    rest: {
      pulls: { list() {}, get: async () => ({ data: pr }) },
      actions: { listWorkflowRunArtifacts() {} },
      issues: {
        listComments() {},
        createComment: async (params) => state.writes.push({ method: 'create', ...params }),
        updateComment: async (params) => state.writes.push({ method: 'update', ...params }),
      },
    },
    paginate: async (method, params) => {
      state.requests.push(params);
      if (method === github.rest.pulls.list) {
        return state.pulls;
      }
      if (method === github.rest.actions.listWorkflowRunArtifacts) {
        return state.artifacts;
      }
      if (method === github.rest.issues.listComments) {
        return state.comments;
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
  return { github, context, core, pr, state };
}

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
    { owner: 'voidzero-dev', repo: 'vite-plus', run_id: 123 },
  ]);
  assert.deepEqual(f.state.outputs, {
    pr: 2684,
    'artifact-id': 456,
    'preview-url': 'https://pr-2684-viteplus-dev.voidzero-docs.workers.dev',
  });
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

for (const artifacts of [
  [],
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
