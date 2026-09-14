// Run with node --test; these workflow helpers need no workspace dependencies.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { copyFile, mkdir, mkdtemp, readFile, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';

import {
  authorizePreview,
  commentPreview,
  isCurrentPreview,
  previewAlias,
  previewUrl,
  requireDeploymentApproval,
  validateAssets,
} from '../docs-fork-preview.mjs';

const versionId = '11111111-1111-4111-8111-111111111111';
const versionUrl = 'https://11111111-viteplus-dev.voidzero-docs.workers.dev';

function grantApproval(state, runId = 987) {
  const approval = {
    environments: [{ id: 10, name: 'docs-preview' }],
    state: 'approved',
    user: { login: 'reviewer' },
  };
  state.approvals.set(runId, [approval]);
  return { approval };
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
    eventName: 'workflow_run',
    runId: 987,
    repo: { owner: 'voidzero-dev', repo: 'vite-plus' },
    payload: {
      workflow_run: {
        id: 123,
        run_attempt: 1,
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
    artifacts: [{ id: 456, name: 'docs-fork-preview-1', expired: false }],
    comments: [],
    outputs: {},
    writes: [],
    requests: [],
    permission: 'write',
    reviewerPermission: 'write',
    approvals: new Map(),
  };
  const github = {
    rest: {
      repos: {
        getCollaboratorPermissionLevel: async (params) => {
          state.requests.push(params);
          return {
            data: {
              permission:
                params.username === 'reviewer' ? state.reviewerPermission : state.permission,
            },
          };
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
        getReviewsForRun: async (params) => {
          state.requests.push(params);
          return { data: state.approvals.get(params.run_id) ?? [] };
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
      throw new Error('Unexpected GitHub request');
    },
  };
  const core = {
    info() {},
    setOutput: (key, value) => {
      state.outputs[key] = value;
    },
  };
  return { github, context, core, pr, state, ...grantApproval(state) };
}

await test('uses the package preview build and protected deployment pattern', async () => {
  const build = await readFile(
    new URL('../../workflows/build-docs-fork-preview.yml', import.meta.url),
    'utf8',
  );
  const deploy = await readFile(
    new URL('../../workflows/deploy-docs-fork-preview.yml', import.meta.url),
    'utf8',
  );
  const helper = await readFile(new URL('../docs-fork-preview.mjs', import.meta.url), 'utf8');
  assert.doesNotMatch(
    build + deploy + helper,
    /pull_request_target|statuses:|createCommitStatus|previewApprovalState/,
  );
  assert.match(build, /github\.event\.action == 'labeled' &&/);
  assert.match(build, /github\.event\.label\.name == 'docs-preview'/);
  assert.match(build, /ref: \$\{\{ github\.event\.pull_request\.head\.sha \}\}/);
  assert.match(build, /cache: false/);
  assert.doesNotMatch(build, /secrets\.|environment:|id-token:|: write/);
  assert.match(
    deploy,
    /workflow_run:\n\s+workflows: \['Build Docs Fork Preview'\]\n\s+types: \[completed\]/,
  );
  assert.doesNotMatch(deploy, /\n  approve:/);
  const authorize = deploy.split('\n  authorize:\n')[1].split('\n  deploy:\n')[0];
  assert.doesNotMatch(authorize, /environment:|secrets\.|: write/);
  const job = deploy.split('\n  deploy:\n')[1];
  assert.match(job, /environment:\n\s+name: docs-preview/);
  assert.match(
    job,
    /name: 'Deploy PR #\$\{\{ needs\.authorize\.outputs\.pr \}\} at \$\{\{ github\.event\.workflow_run\.head_sha \}\}'/,
  );
  assert.match(job, /ref: \$\{\{ github\.sha \}\}/);
  const approval = job.indexOf('await requireDeploymentApproval({ github, context });');
  assert.ok(approval >= 0);
  assert.ok(approval < job.indexOf('- name: Install Wrangler'));
  assert.ok(approval < job.indexOf('- uses: actions/download-artifact@'));
  assert.match(job, /artifact-ids: \$\{\{ needs\.authorize\.outputs\.artifact-id \}\}/);
  assert.match(job, /run-id: \$\{\{ github\.event\.workflow_run\.id \}\}/);
  assert.ok(job.indexOf('await isCurrentPreview(') < job.indexOf('- name: Upload preview version'));
  assert.match(job, /if: steps\.current\.outputs\.current == 'true'/);
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

await test('reuses the PR origin across commits, builds, and reruns while pinning artifacts', async () => {
  const build = await readFile(
    new URL('../../workflows/build-docs-fork-preview.yml', import.meta.url),
    'utf8',
  );
  const deploy = await readFile(
    new URL('../../workflows/deploy-docs-fork-preview.yml', import.meta.url),
    'utf8',
  );
  const originTemplate = build.match(/DOCS_SITE_ORIGIN: (.+)/)?.[1];
  const artifactTemplate = build.match(/name: (docs-fork-preview-.+)/)?.[1];
  assert.ok(originTemplate);
  assert.ok(artifactTemplate);
  for (const [number, runId, attempt, sha] of [
    [2684, 123, 1, 'a'.repeat(40)],
    [2684, 124, 1, 'b'.repeat(40)],
    [2684, 124, 2, 'b'.repeat(40)],
    [2685, 125, 1, 'a'.repeat(40)],
  ]) {
    const f = fixture();
    f.pr.number = number;
    f.pr.head.sha = sha;
    f.context.payload.workflow_run.id = runId;
    f.context.payload.workflow_run.run_attempt = attempt;
    f.context.payload.workflow_run.head_sha = sha;
    f.state.artifacts[0].name = artifactTemplate.replace(
      '${{ github.run_attempt }}',
      String(attempt),
    );
    await authorizePreview(f);
    const origin = originTemplate.replace(
      '${{ github.event.pull_request.number }}',
      String(number),
    );
    assert.equal(origin, `https://pr-${number}-viteplus-dev.voidzero-docs.workers.dev`);
    assert.equal(origin, f.state.outputs['preview-url']);
    assert.equal(f.state.outputs['preview-alias'], `pr-${number}`);
    assert.equal(f.state.outputs['artifact-id'], 456);
  }
  assert.match(deploy, /preview-alias: \$\{\{ steps\.preview\.outputs\.preview-alias \}\}/);
  assert.match(deploy, /PREVIEW_ALIAS: \$\{\{ needs\.authorize\.outputs\.preview-alias \}\}/);
  assert.match(deploy, /--preview-alias "\$PREVIEW_ALIAS"/);
});

await test('uses each PR origin for shell and PowerShell installer links', async (t) => {
  const directory = await mkdtemp(join(tmpdir(), 'docs-preview-installers-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const snapshots = [];
  for (const number of [2684, 2685]) {
    const root = join(directory, String(number));
    const scripts = join(root, 'docs', '.vitepress', 'scripts');
    const output = join(root, 'docs', 'public');
    const installers = join(root, 'packages', 'cli');
    for (const path of [scripts, output, installers]) {
      await mkdir(path, { recursive: true });
    }
    const script = join(scripts, 'copy-installers.mjs');
    await copyFile(
      new URL('../../../docs/.vitepress/scripts/copy-installers.mjs', import.meta.url),
      script,
    );
    for (const name of ['install.sh', 'install.ps1', 'install-legacy.sh', 'install-legacy.ps1']) {
      await copyFile(
        new URL(`../../../packages/cli/${name}`, import.meta.url),
        join(installers, name),
      );
    }
    const origin = previewUrl(number);
    execFileSync(process.execPath, [script], { env: { ...process.env, DOCS_SITE_ORIGIN: origin } });
    const shell = await readFile(join(output, 'install.sh'), 'utf8');
    const powershell = await readFile(join(output, 'install.ps1'), 'utf8');
    assert.ok(shell.includes(`${origin}/install-legacy.sh`));
    assert.ok(powershell.includes(`${origin}/install-legacy.ps1`));
    snapshots.push({ origin, shell, powershell });
  }
  assert.equal(new Set(snapshots.map((snapshot) => snapshot.origin)).size, 2);
  for (const snapshot of snapshots) {
    for (const other of snapshots) {
      if (other.origin !== snapshot.origin) {
        assert.ok(!snapshot.shell.includes(other.origin));
        assert.ok(!snapshot.powershell.includes(other.origin));
      }
    }
  }
});

await test('pins an artifact to the triggering build attempt', async () => {
  const f = fixture();
  f.state.artifacts.push({ id: 789, name: 'docs-fork-preview-2', expired: false });
  await authorizePreview(f);
  assert.equal(f.state.outputs['artifact-id'], 456);
  assert.equal(f.state.outputs['preview-alias'], 'pr-2684');
  f.context.payload.workflow_run.run_attempt = 2;
  await authorizePreview(f);
  assert.equal(f.state.outputs['artifact-id'], 789);
  assert.equal(f.state.outputs['preview-alias'], 'pr-2684');
  f.context.payload.workflow_run.run_attempt = 3;
  await assert.rejects(authorizePreview(f), /Expected one active docs-fork-preview-3/);
});

await test('validates build identities before making requests', async () => {
  for (const value of [undefined, 0, -1, 1.5, NaN, '123', Number.MAX_SAFE_INTEGER + 1]) {
    for (const [runId, attempt] of [
      [value, 1],
      [123, value],
    ]) {
      const f = fixture();
      f.context.payload.workflow_run.id = runId;
      f.context.payload.workflow_run.run_attempt = attempt;
      await assert.rejects(authorizePreview(f), /Invalid docs preview build identity/);
      assert.deepEqual(f.state.requests, []);
    }
  }
});

await test('keeps PR aliases within DNS limits', () => {
  const alias = previewAlias(Number.MAX_SAFE_INTEGER);
  assert.match(alias, /^[a-z][a-z0-9-]*$/);
  assert.ok(`${alias}-viteplus-dev`.length <= 63);
});

await test('serializes deployments per PR with the default queue and lets running uploads finish', async () => {
  const yaml = await readFile(
    new URL('../../workflows/deploy-docs-fork-preview.yml', import.meta.url),
    'utf8',
  );
  assert.match(
    yaml,
    /concurrency:\n\s+group: deploy-docs-fork-preview-\$\{\{ needs\.authorize\.outputs\.pr \}\}\n\s+cancel-in-progress: false/,
  );
});

await test('authorizes a fork with an empty workflow_run PR list and pins its artifact', async () => {
  const f = fixture();
  // The authorize job must be able to request review before approval exists.
  f.state.approvals.clear();
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
    { owner: 'voidzero-dev', repo: 'vite-plus', pull_number: 2684 },
    { owner: 'voidzero-dev', repo: 'vite-plus', username: 'maintainer' },
  ]);
  assert.deepEqual(f.state.outputs, {
    pr: 2684,
    'artifact-id': 456,
    'preview-alias': 'pr-2684',
    'preview-url': 'https://pr-2684-viteplus-dev.voidzero-docs.workers.dev',
  });
  await assert.rejects(requireDeploymentApproval(f), /needs maintainer approval/);
  assert.equal(await isCurrentPreview(f, 2684), false);
});

await test('checks approval for the current deployment run, not the build run', async () => {
  const f = fixture();
  await requireDeploymentApproval(f);
  assert.deepEqual(f.state.requests, [
    { owner: 'voidzero-dev', repo: 'vite-plus', run_id: 987 },
    { owner: 'voidzero-dev', repo: 'vite-plus', username: 'reviewer' },
  ]);
});

await test('does not approve a new commit when a maintainer applies an unrelated label', async () => {
  const f = fixture();
  assert.equal(await isCurrentPreview(f, 2684), true);
  // A fork can change its build triggers. The persistent label and original
  // actor cannot approve B; its new deployment run needs its own review.
  f.pr.head.sha = 'b'.repeat(40);
  f.context.payload.workflow_run.head_sha = f.pr.head.sha;
  f.context.payload.workflow_run.id = 124;
  f.context.runId = 988;
  await authorizePreview(f);
  assert.equal(f.state.outputs.pr, 2684);
  await assert.rejects(requireDeploymentApproval(f), /needs maintainer approval/);
  assert.equal(await isCurrentPreview(f, 2684), false);
  await commentPreview(f, 2684, uploadOutput());
  assert.deepEqual(f.state.writes, []);
  grantApproval(f.state, f.context.runId);
  await requireDeploymentApproval(f);
  assert.equal(await isCurrentPreview(f, 2684), true);
});

for (const [name, mutate] of [
  ['no review or an unprotected environment', (f) => f.state.approvals.clear()],
  [
    'a build-run review',
    (f) => {
      f.state.approvals.clear();
      grantApproval(f.state, 123);
    },
  ],
  [
    'another deployment review',
    (f) => {
      f.state.approvals.clear();
      grantApproval(f.state, 986);
    },
  ],
  ['another environment', (f) => (f.approval.environments[0].name = 'release')],
  ['missing environments', (f) => (f.approval.environments = undefined)],
  ['no environments', (f) => (f.approval.environments = [])],
  ['a missing reviewer', (f) => (f.approval.user = undefined)],
  ...['pending', 'rejected', undefined].map((state) => [
    state + ' review',
    (f) => (f.approval.state = state),
  ]),
  ...['read', 'triage', 'none', undefined].map((permission) => [
    permission + ' reviewer permission',
    (f) => (f.state.reviewerPermission = permission),
  ]),
]) {
  await test('refuses deployment and commenting with ' + name, async () => {
    const f = fixture();
    mutate(f);
    await assert.rejects(requireDeploymentApproval(f), /needs maintainer approval/);
    assert.equal(await isCurrentPreview(f, 2684), false);
    await commentPreview(f, 2684, uploadOutput());
    assert.deepEqual(f.state.writes, []);
  });
}

for (const permission of ['admin', 'maintain', 'write']) {
  await test('accepts an environment reviewer with ' + permission + ' permission', async () => {
    const f = fixture();
    f.state.reviewerPermission = permission;
    await requireDeploymentApproval(f);
  });
}

for (const state of ['pending', 'rejected']) {
  for (const first of [true, false]) {
    await test(
      'rejects mixed ' + state + ' and approved reviews regardless of history order: ' + first,
      async () => {
        const f = fixture();
        const review = { ...f.approval, state };
        f.state.approvals.set(987, first ? [review, f.approval] : [f.approval, review]);
        await assert.rejects(requireDeploymentApproval(f), /needs maintainer approval/);
      },
    );
  }
}

await test('ignores reviews for unrelated environments', async () => {
  const f = fixture();
  f.state.approvals.get(987).unshift({ state: 'rejected', environments: [{ name: 'release' }] });
  await requireDeploymentApproval(f);
});

await test('rechecks approval revocation before upload and commenting', async () => {
  const f = fixture();
  await requireDeploymentApproval(f);
  f.approval.state = 'rejected';
  assert.equal(await isCurrentPreview(f, 2684), false);
  await commentPreview(f, 2684, uploadOutput());
  assert.deepEqual(f.state.writes, []);
});

for (const [area, method] of [
  ['actions', 'getReviewsForRun'],
  ['repos', 'getCollaboratorPermissionLevel'],
]) {
  await test('fails closed when the deployment approval ' + method + ' lookup fails', async () => {
    const f = fixture();
    f.github.rest[area][method] = async () => {
      throw new Error('GitHub API failed');
    };
    await assert.rejects(requireDeploymentApproval(f), /GitHub API failed/);
    await assert.rejects(isCurrentPreview(f, 2684), /GitHub API failed/);
    await assert.rejects(commentPreview(f, 2684, uploadOutput()), /GitHub API failed/);
    assert.deepEqual(f.state.writes, []);
  });
}

for (const runId of [undefined, 0, -1, '987', Number.MAX_SAFE_INTEGER + 1]) {
  await test('rejects an invalid trusted deployment run ID: ' + runId, async () => {
    const f = fixture();
    f.context.runId = runId;
    await assert.rejects(requireDeploymentApproval(f), /Invalid docs preview workflow run/);
    assert.deepEqual(f.state.requests, []);
  });
}

for (const [name, mutate] of [
  ['new commit', (f) => (f.pr.head.sha = 'b'.repeat(40))],
  ['label removal', (f) => (f.pr.labels = [])],
  ['PR closure', (f) => (f.pr.state = 'closed')],
  ['base change', (f) => (f.pr.base.ref = 'release')],
  ['requester permission removal', (f) => (f.state.permission = 'read')],
]) {
  await test('rechecks ' + name + ' before requesting environment review', async () => {
    const f = fixture();
    const paginate = f.github.paginate;
    f.github.paginate = async (method, params) => {
      const result = await paginate(method, params);
      if (method === f.github.rest.actions.listWorkflowRunArtifacts) {
        mutate(f);
      }
      return result;
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

await test('does not authorize or comment on a label event', async () => {
  const f = fixture();
  f.context.eventName = 'pull_request_target';
  await assert.rejects(authorizePreview(f), /Invalid docs preview workflow run/);
  await assert.rejects(
    commentPreview(f, 2684, uploadOutput()),
    /Invalid docs preview workflow run/,
  );
  assert.deepEqual(f.state.outputs, {});
  assert.deepEqual(f.state.writes, []);
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
  [{ id: 456, name: 'docs-fork-preview-1', expired: true }],
  [{ id: 456, name: 'other', expired: false }],
  [
    { id: 456, name: 'docs-fork-preview-1', expired: false },
    { id: 789, name: 'docs-fork-preview-1', expired: false },
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
    `<!-- cloudflare-docs-fork-preview -->\nCloudflare documentation preview: ${previewUrl(2684)}\n\nCommit: ${'a'.repeat(40)}`,
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
  assert.ok(f.state.writes[0].body.includes(previewUrl(2684)));
});

await test('does not comment if the PR changes during upload', async () => {
  const f = fixture();
  f.pr.head.sha = 'b'.repeat(40);
  await commentPreview(f, 2684, uploadOutput());
  assert.deepEqual(f.state.writes, []);
});

await test('updates one preview comment with the same PR URL across commits and reruns', async () => {
  const f = fixture();
  await commentPreview(f, 2684, uploadOutput());
  const previousBody = f.state.writes[0].body;
  f.state.comments.push({
    id: 101,
    user: { login: 'github-actions[bot]' },
    body: previousBody,
  });
  f.state.writes = [];

  for (const attempt of [1, 2]) {
    f.context.payload.workflow_run.id = 124;
    f.context.payload.workflow_run.run_attempt = attempt;
    f.context.payload.workflow_run.head_sha = 'b'.repeat(40);
    f.pr.head.sha = 'b'.repeat(40);
    f.context.runId = 987 + attempt;
    grantApproval(f.state, f.context.runId);
    await commentPreview(
      f,
      2684,
      uploadOutput({
        version_id: '22222222-2222-4222-8222-222222222222',
        preview_url: 'https://22222222-viteplus-dev.voidzero-docs.workers.dev',
      }),
    );
  }

  assert.equal(f.state.writes.length, 2);
  for (const comment of f.state.writes) {
    assert.equal(comment.method, 'update');
    assert.equal(comment.comment_id, 101);
    assert.equal(comment.body, previousBody.replace('a'.repeat(40), 'b'.repeat(40)));
    assert.deepEqual(comment.body.match(/https:\/\/\S+/g), [previewUrl(2684)]);
  }
});

await test('reads the PR alias from Wrangler JSONL with other records and blank lines', async () => {
  const f = fixture();
  await commentPreview(f, 2684, `\n${JSON.stringify({ type: 'other' })}\n${uploadOutput()}\n`);
  assert.ok(f.state.writes[0].body.includes(previewUrl(2684)));
  assert.ok(!f.state.writes[0].body.includes(versionUrl));
});

for (const [name, output] of [
  ['missing upload', ''],
  ['duplicate uploads', uploadOutput() + uploadOutput()],
  ['invalid JSON', '{'],
  ['unsupported output version', uploadOutput({ version: 2 })],
  ['another Worker', uploadOutput({ worker_name: 'other' })],
  ['invalid version ID', uploadOutput({ version_id: 'invalid' })],
  ['disabled preview URLs', uploadOutput({ preview_url: undefined })],
  ['alias instead of version URL', uploadOutput({ preview_url: previewUrl(2684) })],
  ['missing PR alias', uploadOutput({ preview_alias_url: undefined })],
  ['another PR alias', uploadOutput({ preview_alias_url: previewUrl(2685) })],
  [
    'build attempt alias',
    uploadOutput({
      preview_alias_url: 'https://build-123-1-viteplus-dev.voidzero-docs.workers.dev',
    }),
  ],
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
  for (const number of [
    undefined,
    0,
    -1,
    1.5,
    NaN,
    Number.MAX_SAFE_INTEGER + 1,
    '2684',
    '2684\nother-output=true',
  ]) {
    assert.throws(() => previewAlias(number), /Invalid pull request number/);
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
