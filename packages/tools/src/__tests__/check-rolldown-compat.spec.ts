import { spawnSync } from 'node:child_process';
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';

import { expect, onTestFinished, test } from 'vite-plus/test';
import { parse } from 'yaml';

import { checkRolldownCompatibility } from '../check-rolldown-compat.ts';

const repoRoot = resolve(import.meta.dirname, '../../../..');
const script = join(repoRoot, 'packages/tools/src/check-rolldown-compat.ts');

function writeJson(file: string, value: unknown) {
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, JSON.stringify(value));
}

function fixture(
  overrides: { version?: unknown; viteRange?: unknown; tsdownRange?: unknown } = {},
) {
  const { version, viteRange, tsdownRange } = {
    version: '1.2.16',
    viteRange: '~1.2.16',
    tsdownRange: '~1.2.15',
    ...overrides,
  };
  const root = mkdtempSync(join(tmpdir(), 'vp-rolldown-compat-'));
  onTestFinished(() => rmSync(root, { recursive: true, force: true }));
  const rolldownPath = join(root, 'rolldown/packages/rolldown/package.json');
  const tsdownPath = join(root, 'store/tsdown/package.json');
  writeJson(rolldownPath, { name: 'rolldown', version });
  writeJson(join(root, 'vite/packages/vite/package.json'), {
    dependencies: { rolldown: viteRange },
  });
  writeJson(join(root, 'packages/core/package.json'), {});
  writeJson(tsdownPath, { dependencies: { rolldown: tsdownRange } });
  mkdirSync(join(root, 'packages/core/node_modules'), { recursive: true });
  // Match pnpm's package layout, including an override that hides version skew.
  symlinkSync(dirname(tsdownPath), join(root, 'packages/core/node_modules/tsdown'), 'junction');
  mkdirSync(join(root, 'store/tsdown/node_modules'), { recursive: true });
  symlinkSync(dirname(rolldownPath), join(root, 'store/tsdown/node_modules/rolldown'), 'junction');
  return { root, rolldownPath, tsdownPath };
}

test.each([
  { version: '1.2.16', viteRange: '~1.2.16', tsdownRange: '~1.2.15' },
  { version: '1.2.17', viteRange: '^1.2.0', tsdownRange: '>=1.2.15 <1.3.0' },
  { version: '1.3.0', viteRange: '^1.3.0', tsdownRange: '~1.3.0' },
])('accepts Rolldown $version with $viteRange and $tsdownRange', (versions) => {
  expect(() => checkRolldownCompatibility(fixture(versions).root)).not.toThrow();
});

test.each([
  { version: '1.3.0', viteRange: '~1.2.16', tsdownRange: '~1.3.0', error: "vite's 1.2.x" },
  { version: '1.2.16', viteRange: '~1.2.16', tsdownRange: '~1.3.0', error: "tsdown's 1.3.x" },
  { version: '1.3.0', viteRange: '^1.2.16', tsdownRange: '^1.3.0', error: "vite's 1.2.x" },
  { version: '1.3.0', viteRange: '^1.2.16', tsdownRange: '^1.2.15', error: "vite's 1.2.x" },
  { version: '2.0.0', viteRange: '^1.2.16', tsdownRange: '^2.0.0', error: "vite's 1.2.x" },
  {
    version: '1.2.15',
    viteRange: '~1.2.16',
    tsdownRange: '~1.2.15',
    error: "does not satisfy vite's",
  },
  {
    version: '1.2.16',
    viteRange: '~1.2.15',
    tsdownRange: '~1.2.17',
    error: "does not satisfy tsdown's",
  },
  {
    version: '1.2.17',
    viteRange: '>=1.2.16 <1.2.17',
    tsdownRange: '~1.2.15',
    error: "does not satisfy vite's",
  },
])('rejects Rolldown $version with $viteRange and $tsdownRange', ({ error, ...versions }) => {
  expect(() => checkRolldownCompatibility(fixture(versions).root)).toThrow(error);
});

test.each(['viteRange', 'tsdownRange'] as const)('rejects missing or invalid %s', (key) => {
  for (const range of [undefined, '', 'not-a-range', 'workspace:*', '<0.0.0', 123]) {
    expect(() => checkRolldownCompatibility(fixture({ [key]: range }).root)).toThrow(
      'Missing or invalid dependencies.rolldown',
    );
  }
});

test.each([undefined, 'not-a-version', '1.2', 123])(
  'rejects invalid Rolldown version %s',
  (version) => {
    expect(() => checkRolldownCompatibility(fixture({ version }).root)).toThrow(
      'Missing or invalid version',
    );
  },
);

test('the command fails when a workspace override hides an incompatible tsdown requirement', () => {
  const { root, rolldownPath, tsdownPath } = fixture({ tsdownRange: '~1.3.0' });
  expect(realpathSync(createRequire(tsdownPath).resolve('rolldown/package.json'))).toBe(
    realpathSync(rolldownPath),
  );
  const result = spawnSync(process.execPath, [script], { cwd: root, encoding: 'utf8' });
  expect(result.status).toBe(1);
  expect(result.stderr).toContain("Rolldown 1.2.16 must use tsdown's 1.3.x release line (~1.3.0)");
});

test('the command succeeds when the upstream requirements agree', () => {
  const result = spawnSync(process.execPath, [script], { cwd: fixture().root, encoding: 'utf8' });
  expect(result.status, result.stderr).toBe(0);
  expect(result.stdout).toContain('Rolldown is compatible with Vite and tsdown.');
});

test('the checked-out upstreams satisfy the Rolldown policy', () => {
  checkRolldownCompatibility(repoRoot);
});

// The upgrade workflow uses Bash on Ubuntu.
test.skipIf(process.platform === 'win32').each([
  { tsdownRange: '~1.3.0', expectedStatus: 1, buildStarted: false },
  { tsdownRange: '~1.2.15', expectedStatus: 99, buildStarted: true },
])(
  'upgrade final validation gates the build with tsdown $tsdownRange',
  ({ tsdownRange, expectedStatus, buildStarted }) => {
    const workflow = parse(
      readFileSync(join(repoRoot, '.github/workflows/upgrade-deps.yml'), 'utf8'),
    );
    const step = workflow.jobs.upgrade.steps.find(
      (step: { id?: string }) => step.id === 'final-validation',
    );
    expect(step.if).toBeUndefined();
    const { root } = fixture({ tsdownRange });
    symlinkSync(join(repoRoot, 'packages/tools'), join(root, 'packages/tools'), 'junction');
    const bin = join(root, 'bin');
    const marker = join(root, 'build-started');
    mkdirSync(bin);
    symlinkSync(process.execPath, join(bin, 'node'));
    // Stop at the first build command without running bootstrap or later commands.
    writeFileSync(join(bin, 'pnpm'), '#!/bin/sh\n: > "$VP_GUARD_TEST_MARKER"\nexit 99\n', {
      mode: 0o755,
    });
    const result = spawnSync('bash', ['-c', step.run], {
      cwd: root,
      encoding: 'utf8',
      env: {
        ...process.env,
        PATH: `${bin}:${process.env.PATH ?? ''}`,
        VP_GUARD_TEST_MARKER: marker,
      },
    });
    expect(result.status, result.stderr).toBe(expectedStatus);
    expect(existsSync(marker)).toBe(buildStarted);
  },
);
