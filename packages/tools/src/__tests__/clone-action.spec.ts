import { execFileSync, spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { delimiter, dirname, join } from 'node:path';

import { expect, onTestFinished, test } from 'vite-plus/test';
import { parse } from 'yaml';

interface Step {
  name?: string;
  uses?: string;
  if?: string;
  run?: string;
  with?: Record<string, unknown>;
}

const repoRoot = new URL('../../../../', import.meta.url);
const { inputs, runs } = parse(
  readFileSync(new URL('.github/actions/clone/action.yml', repoRoot), 'utf8'),
) as {
  inputs: Record<string, { default: string }>;
  runs: { steps: Step[] };
};

test('sets up native TypeScript support before the bootstrap scripts', () => {
  const setupIndex = runs.steps.findIndex((step) => step.uses?.startsWith('actions/setup-node@'));
  expect(setupIndex).toBeGreaterThanOrEqual(0);
  expect(inputs['setup-node'].default).toBe('true');
  expect(runs.steps[setupIndex].if).toBe("${{ inputs.setup-node != 'false' }}");
  expect(runs.steps[setupIndex].with).toMatchObject({
    'node-version-file': '.node-version',
    'package-manager-cache': false,
  });
  const validationIndex = runs.steps.findIndex(
    (step) => step.name === 'Validate bootstrap Node.js',
  );
  expect(validationIndex).toBeGreaterThan(setupIndex);
  expect(runs.steps[validationIndex].if).toBeUndefined();
  for (const script of ['vendored-vitest.ts', 'ecosystem-ci/clone.ts']) {
    const scriptIndex = runs.steps.findIndex((step) => step.run?.includes(script));
    expect(scriptIndex).toBeGreaterThan(validationIndex);
  }
});

test('rejects a caller-provided Node.js without native TypeScript support', () => {
  const step = runs.steps.find((entry) => entry.name === 'Validate bootstrap Node.js');
  expect(step?.run).toBeDefined();
  const result = spawnSync('bash', ['--noprofile', '--norc', '-e', '-c', step!.run!], {
    encoding: 'utf8',
    env: {
      ...process.env,
      PATH: `${dirname(process.execPath)}${delimiter}${process.env.PATH}`,
      NODE_OPTIONS: '--no-experimental-strip-types',
    },
  });
  expect(result.status).toBe(1);
  expect(result.stderr).toContain('Clone bootstrap requires native TypeScript support');
});

test('runs both bootstrap scripts without installed dependencies or registry access', () => {
  const root = mkdtempSync(join(tmpdir(), 'vp-clone-bootstrap-'));
  onTestFinished(() => rmSync(root, { recursive: true, force: true }));
  for (const file of [
    'package.json',
    'pnpm-lock.yaml',
    'pnpm-workspace.yaml',
    'packages/tools/src/vendored-vitest.ts',
    'packages/cli/src/utils/constants.ts',
    'ecosystem-ci/clone.ts',
    'ecosystem-ci/paths.ts',
  ]) {
    mkdirSync(dirname(join(root, file)), { recursive: true });
    copyFileSync(new URL(file, repoRoot), join(root, file));
  }
  const viteDir = join(root, 'vite/packages/vite');
  mkdirSync(viteDir, { recursive: true });
  writeFileSync(join(viteDir, 'package.json'), '{"devDependencies":{"vitest":"4.1.10"}}');

  const projectDir = join(root, 'vite-plus-ecosystem-ci/fixture');
  mkdirSync(projectDir, { recursive: true });
  const git = (...args: string[]) =>
    execFileSync('git', args, { cwd: projectDir, encoding: 'utf8', stdio: 'pipe' }).trim();
  git('init');
  git('config', 'core.hooksPath', join(root, 'no-hooks'));
  git(
    '-c',
    'user.name=Test',
    '-c',
    'user.email=test@example.com',
    '-c',
    'commit.gpgsign=false',
    'commit',
    '--allow-empty',
    '-m',
    'fixture',
  );
  const repository = 'https://example.invalid/fixture.git';
  git('remote', 'add', 'origin', repository);
  writeFileSync(
    join(root, 'ecosystem-ci/repo.json'),
    JSON.stringify({ fixture: { repository, hash: git('rev-parse', 'HEAD') } }),
  );

  for (const stepName of [
    'Validate bootstrap Node.js',
    "Align vendored Vite's Vitest dependencies",
    'Clone ecosystem ci project',
  ]) {
    const step = runs.steps.find((entry) => entry.name === stepName);
    expect(step?.run).toBeDefined();
    const result = spawnSync('bash', ['--noprofile', '--norc', '-e', '-c', step!.run!], {
      cwd: root,
      encoding: 'utf8',
      timeout: 15_000,
      env: {
        ...process.env,
        PATH: `${dirname(process.execPath)}${delimiter}${process.env.PATH}`,
        CI: 'true',
        RUNNER_TEMP: root,
        INPUTS_ECOSYSTEM_CI_PROJECT: 'fixture',
        npm_config_cache: join(root, 'npm-cache'),
        npm_config_offline: 'true',
        npm_config_registry: 'https://registry.invalid',
      },
    });
    expect(result.error).toBeUndefined();
    expect(result.status, result.stderr).toBe(0);
    if (stepName === 'Clone ecosystem ci project') {
      expect(result.stdout).toContain('Already at correct commit');
    }
  }
  const manifest = JSON.parse(readFileSync(join(viteDir, 'package.json'), 'utf8'));
  const workspace = parse(readFileSync(join(root, 'pnpm-workspace.yaml'), 'utf8'));
  expect(manifest.devDependencies.vitest).toBe(workspace.catalog.vitest);
});
