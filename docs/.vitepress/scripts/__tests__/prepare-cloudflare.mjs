import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import { getWorkerBundleDir, readBuildOutput } from '@cloudflare/build-output-utils';

import { prepareCloudflare } from '../prepare-cloudflare.mjs';

await test('packages only static assets and removes stale Worker code', async (t) => {
  const directory = await mkdtemp(join(tmpdir(), 'docs-cloudflare-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const sourceDirectory = join(directory, 'assets');
  const root = join(directory, 'deploy');
  await mkdir(sourceDirectory);
  await writeFile(join(sourceDirectory, 'index.html'), '<!doctype html><title>Docs</title>');
  await writeFile(join(sourceDirectory, '404.html'), '<!doctype html><title>Not found</title>');
  // Fork content with config-like names must remain inert asset bytes.
  await writeFile(join(sourceDirectory, 'cloudflare.config.ts'), 'throw new Error("fork config");');
  await writeFile(join(sourceDirectory, '.env'), 'CLOUDFLARE_API_TOKEN=untrusted');
  await writeFile(
    join(sourceDirectory, 'package.json'),
    JSON.stringify({ scripts: { build: 'exit 1' } }),
  );
  await mkdir(getWorkerBundleDir(root), { recursive: true });
  await writeFile(join(getWorkerBundleDir(root), 'index.js'), 'throw new Error("stale Worker");');

  await prepareCloudflare(sourceDirectory, root);

  const output = await readBuildOutput(root);
  assert.deepEqual(output.rootConfig.buildContext, { isPreview: false });
  assert.deepEqual(Object.keys(output.workers), ['default']);
  assert.deepEqual(output.containers, []);
  const worker = output.workers.default;
  assert.equal(worker.bundleDir, undefined);
  assert.equal(worker.config.manifest, undefined);
  assert.equal(worker.config.name, 'viteplus-dev');
  assert.equal(worker.config.compatibilityDate, '2026-09-02');
  assert.equal(worker.config.assets.htmlHandling, 'auto-trailing-slash');
  assert.equal(worker.config.assets.notFoundHandling, '404-page');
  assert.equal(
    await readFile(join(worker.assetsDir, 'cloudflare.config.ts'), 'utf8'),
    'throw new Error("fork config");',
  );
  assert.equal(
    await readFile(join(worker.assetsDir, '404.html'), 'utf8'),
    '<!doctype html><title>Not found</title>',
  );

  const outputPath = join(directory, 'upload.jsonl');
  const cf = join(dirname(fileURLToPath(import.meta.resolve('cf/package.json'))), 'bin/cf');
  await promisify(execFile)(
    process.execPath,
    [cf, 'workers', 'versions', 'create', '--prebuilt', '--dry-run', '--preview-alias', 'pr-2684'],
    {
      cwd: root,
      env: {
        ...process.env,
        CF_SEND_TELEMETRY: 'false',
        WRANGLER_OUTPUT_FILE_PATH: outputPath,
      },
    },
  );
  // cf beta still emits the JSONL record consumed by the preview comment helper.
  const records = (await readFile(outputPath, 'utf8')).trim().split('\n').map(JSON.parse);
  const uploads = records.filter((record) => record.type === 'version-upload');
  assert.equal(uploads.length, 1);
  assert.equal(uploads[0].version, 1);
  assert.equal(uploads[0].worker_name, 'viteplus-dev');
  assert.equal(uploads[0].version_id, null);
});
