import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { test } from 'node:test';

import {
  binaryArtifacts,
  comparePreviewBinaries,
  createReleaseLoader,
  measureBinary,
  postSizeComment,
  renderSizeReport,
  targets,
} from '../preview-binary-size.mjs';

const release = { tag_name: 'v1.3.0', draft: false, prerelease: false, assets: [] };
const headSha = 'a'.repeat(40);
const runUrl = 'https://github.com/voidzero-dev/vite-plus/actions/runs/123';

async function temporaryDirectory(t) {
  const directory = await mkdtemp(join(tmpdir(), 'preview-size-test-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  return directory;
}

test('compares every shipped binary without running preview content', async (t) => {
  const artifactDirectory = await temporaryDirectory(t);
  const payload = Buffer.from('This is not executable code.\n'.repeat(80));
  const artifacts = binaryArtifacts();
  const matrix = await readFile(
    new URL('../../workflows/reusable-release-build.yml', import.meta.url),
    'utf8',
  );
  assert.deepEqual(
    targets.map(([target]) => target).sort(),
    [...matrix.matchAll(/- target: (\S+)/g)].map((match) => match[1]).sort(),
  );
  assert.equal(artifacts.length, 20);
  for (const artifact of artifacts) {
    const path = join(artifactDirectory, artifact.path);
    await mkdir(dirname(path), { recursive: true });
    await writeFile(path, payload);
  }
  const requested = [];
  const body = await comparePreviewBinaries({
    release,
    artifactDirectory,
    headSha,
    runUrl,
    loadRelease: async (artifact) => {
      requested.push(artifact.path);
      return Buffer.concat([payload, payload]);
    },
  });
  assert.equal(requested.length, 20);
  assert.equal((body.match(/-50\.00%/g) ?? []).length, 20);
  assert.match(body, /gzip -9 sizes/);
  assert.match(body, /v1\.3\.0/);
  for (const artifact of artifacts) assert.ok(body.includes(artifact.name));
});

test('reports increases, unchanged sizes, and unavailable release binaries', () => {
  const body = renderSizeReport({
    release,
    headSha,
    runUrl,
    rows: [
      { name: 'Increased', baseline: { raw: 100, gzip: 50 }, preview: { raw: 150, gzip: 75 } },
      { name: 'Unchanged', baseline: { raw: 100, gzip: 50 }, preview: { raw: 100, gzip: 50 } },
      { name: 'New', baseline: null, preview: { raw: 100, gzip: 50 } },
    ],
  });
  assert.match(body, /\+50 B \(\+50\.00%\)/);
  assert.match(body, /0 B \(0\.00%\)/);
  assert.match(body, /New \| — \| 100 B \| Not in release/);
  assert.throws(() => measureBinary(Buffer.alloc(0)), /empty/);
  assert.throws(() => renderSizeReport({ release: { ...release, prerelease: true } }), /stable/);
  assert.throws(() => renderSizeReport({ release, headSha: 'invalid' }), /commit/);
});

test('fails on missing preview binaries and refuses symbolic links', async (t) => {
  const artifactDirectory = await temporaryDirectory(t);
  const options = { release, artifactDirectory, headSha, runUrl, loadRelease: async () => null };
  await assert.rejects(comparePreviewBinaries(options), { code: 'ENOENT' });
  const first = join(artifactDirectory, binaryArtifacts()[0].path);
  await mkdir(dirname(first), { recursive: true });
  const other = join(artifactDirectory, 'other');
  await writeFile(other, 'unrelated file');
  await symlink(other, first);
  await assert.rejects(comparePreviewBinaries(options), /regular preview binary/);
});

test('loads matching release archives and exact-version npm binaries without installation', async (t) => {
  const directory = await temporaryDirectory(t);
  const payload = Buffer.from('native binary bytes');
  await mkdir(join(directory, 'package'));
  await writeFile(join(directory, 'package/vite-plus.darwin-arm64.node'), payload);
  await writeFile(join(directory, 'vp.exe'), payload);
  await writeFile(join(directory, 'vp-shim.exe'), payload);
  const tgz = join(directory, 'napi.tgz');
  const zip = join(directory, 'cli.zip');
  execFileSync('tar', ['-czf', tgz, '-C', directory, 'package']);
  execFileSync('zip', ['-q', zip, 'vp.exe', 'vp-shim.exe'], { cwd: directory });
  const responses = new Map([
    [
      'https://registry.npmjs.org/%40voidzero-dev%2Fvite-plus-darwin-arm64/1.3.0',
      JSON.stringify({
        name: '@voidzero-dev/vite-plus-darwin-arm64',
        version: '1.3.0',
        dist: { tarball: 'https://registry.npmjs.org/native.tgz' },
      }),
    ],
    ['https://registry.npmjs.org/native.tgz', await readFile(tgz)],
    [
      `https://github.com/voidzero-dev/vite-plus/releases/download/v1.3.0/vp-x86_64-pc-windows-msvc.zip`,
      await readFile(zip),
    ],
  ]);
  const requests = [];
  const originalFetch = globalThis.fetch;
  t.after(() => {
    globalThis.fetch = originalFetch;
  });
  globalThis.fetch = async (url) => {
    requests.push(url);
    return responses.has(url)
      ? new Response(responses.get(url))
      : new Response('', { status: 404 });
  };
  const loader = createReleaseLoader(
    {
      ...release,
      assets: [{ name: 'vp-x86_64-pc-windows-msvc.zip' }],
    },
    directory,
  );
  const artifacts = binaryArtifacts();
  for (const path of [
    'vite-plus-native-aarch64-apple-darwin/vite-plus.darwin-arm64.node',
    'vp-global-cli-x86_64-pc-windows-msvc/vp.exe',
    'vp-global-cli-x86_64-pc-windows-msvc/vp-shim.exe',
  ]) {
    assert.deepEqual(await loader(artifacts.find((artifact) => artifact.path === path)), payload);
  }
  assert.equal(requests.filter((url) => url.endsWith('.zip')).length, 1);
  assert.equal(
    await loader(artifacts.find((artifact) => artifact.package?.endsWith('darwin-x64'))),
    null,
  );
  assert.equal(
    await loader(artifacts.find((artifact) => artifact.asset?.startsWith('vp-setup-'))),
    null,
  );
  responses.set(
    'https://registry.npmjs.org/%40voidzero-dev%2Fvite-plus-darwin-arm64/1.3.0',
    JSON.stringify({ name: 'wrong', version: '1.3.0' }),
  );
  await assert.rejects(loader(artifacts[1]), /Unexpected npm package/);
});

function commentFixture() {
  const writes = [];
  const pr = { state: 'open', head: { sha: headSha }, labels: [{ name: 'preview-build' }] };
  const comments = [];
  return {
    pr,
    comments,
    writes,
    args: {
      github: {
        rest: {
          pulls: { get: async () => ({ data: pr }) },
          issues: {
            listComments: () => {},
            createComment: async (request) => writes.push({ action: 'create', ...request }),
            updateComment: async (request) => writes.push({ action: 'update', ...request }),
          },
        },
        paginate: async () => comments,
      },
      context: {
        repo: { owner: 'voidzero-dev', repo: 'vite-plus' },
        payload: { workflow_run: { head_sha: headSha } },
      },
      core: { info: () => {} },
      prNumber: 123,
      body: '<!-- preview-binary-size -->\nreport',
    },
  };
}

test('creates one report and updates only the bot-owned size comment', async () => {
  const { args, comments, writes } = commentFixture();
  comments.push({ id: 1, user: { login: 'contributor' }, body: args.body });
  await postSizeComment(args);
  assert.equal(writes[0].action, 'create');
  assert.equal(writes[0].issue_number, 123);
  comments.push({ id: 2, user: { login: 'github-actions[bot]' }, body: args.body });
  await postSizeComment(args);
  assert.equal(writes[1].action, 'update');
  assert.equal(writes[1].comment_id, 2);
});

test('does not post if the PR advances, closes, or loses its preview label', async () => {
  for (const change of [
    (pr) => {
      pr.head.sha = 'b'.repeat(40);
    },
    (pr) => {
      pr.state = 'closed';
    },
    (pr) => {
      pr.labels = [];
    },
  ]) {
    const { args, pr, writes } = commentFixture();
    change(pr);
    await postSizeComment(args);
    assert.deepEqual(writes, []);
  }
});
