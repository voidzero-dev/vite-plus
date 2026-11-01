import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { afterEach, expect, test } from 'vite-plus/test';
import { parse } from 'yaml';

interface Step {
  name?: string;
  id?: string;
  if?: string;
  'continue-on-error'?: boolean;
  env?: Record<string, string>;
  run?: string;
  with?: Record<string, unknown>;
}

const workflow = parse(
  readFileSync(new URL('../../../../.github/workflows/upgrade-deps.yml', import.meta.url), 'utf8'),
);
const steps: Step[] = workflow.jobs.upgrade.steps;
const temporaryDirs: string[] = [];
const bashTest = test.skipIf(process.platform === 'win32');
const outcomes = {
  REPAIR_OUTCOME: 'success',
  LOCKFILE_OUTCOME: 'success',
  FORMAT_OUTCOME: 'success',
  VALIDATION_OUTCOME: 'success',
};

function step(name: string): Step {
  const result = steps.find((entry) => entry.name === name);
  expect(result, `Missing workflow step: ${name}`).toBeDefined();
  return result!;
}

function recordStatus(overrides: Partial<typeof outcomes> = {}) {
  const dir = mkdtempSync(path.join(tmpdir(), 'vp-upgrade-workflow-'));
  temporaryDirs.push(dir);
  const output = path.join(dir, 'output');
  const summary = path.join(dir, 'summary');
  execFileSync('bash', ['-e', '-c', step('Record upgrade status').run!], {
    cwd: dir,
    env: {
      ...process.env,
      ...outcomes,
      ...overrides,
      GITHUB_OUTPUT: output,
      GITHUB_STEP_SUMMARY: summary,
    },
  });
  return {
    output: readFileSync(output, 'utf8').trim(),
    summary: readFileSync(summary, 'utf8'),
  };
}

afterEach(() => {
  for (const dir of temporaryDirs.splice(0)) {
    rmSync(dir, { recursive: true, force: true });
  }
});

bashTest('allows a ready dependency PR only after all required steps succeed', () => {
  const result = recordStatus();
  expect(result.output).toBe('draft=false');
  expect(result.summary.match(/\|\s*success\s*\|/g)).toHaveLength(4);
});

bashTest.each(
  Object.keys(outcomes).flatMap((name) =>
    ['failure', 'skipped', 'cancelled'].map((outcome) => [name, outcome]),
  ),
)('keeps the dependency PR as a draft when %s is %s', (name, outcome) => {
  const result = recordStatus({ [name]: outcome });
  expect(result.output).toBe('draft=true');
  expect(result.summary).toContain(`| ${outcome} |`);
  expect(result.summary.match(/\|\s*success\s*\|/g)).toHaveLength(3);
});

test('preserves PR creation after repair or validation failures and reports failure last', () => {
  for (const name of [
    'Check upgrade dependencies with Kimi K3',
    'Update lockfile',
    'Format code',
    'Final validation',
  ]) {
    expect(step(name)['continue-on-error'], name).toBe(true);
    expect(step(name).if, name).toBeUndefined();
  }

  expect(step('Record upgrade status').env).toEqual({
    REPAIR_OUTCOME: '${{ steps.check-upgrade-dependencies.outcome }}',
    LOCKFILE_OUTCOME: '${{ steps.update-lockfile.outcome }}',
    FORMAT_OUTCOME: '${{ steps.format.outcome }}',
    VALIDATION_OUTCOME: '${{ steps.final-validation.outcome }}',
  });
  expect(step('Enhance PR description with Kimi K3').if).toBe(
    "steps.check-upgrade-dependencies.outcome == 'success'",
  );
  expect(step('Create/Update PR').with?.draft).toBe(
    "${{ steps.upgrade-status.outputs.draft != 'false' }}",
  );
  expect(step('Report validation failure').if).toBe(
    "always() && steps.upgrade-status.outputs.draft != 'false'",
  );

  const names = [
    'Final validation',
    'Record upgrade status',
    'Read generated PR content',
    'Create/Update PR',
    'Report validation failure',
  ];
  const indexes = names.map((name) => steps.indexOf(step(name)));
  expect(indexes).toEqual(indexes.toSorted((a, b) => a - b));
  for (const name of ['Record upgrade status', 'Read generated PR content', 'Create/Update PR']) {
    expect(step(name).if, name).toBeUndefined();
  }
});
