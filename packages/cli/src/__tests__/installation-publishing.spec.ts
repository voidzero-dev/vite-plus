import { readFile } from 'node:fs/promises';

import { describe, expect, it } from 'vitest';
import { parse } from 'yaml';

interface Step {
  name?: string;
  id?: string;
  uses?: string;
  run?: string;
  if?: string;
  with?: Record<string, string>;
}

interface Job {
  steps: Step[];
  needs?: string | string[];
  permissions?: Record<string, string>;
  environment?: string;
}

async function workflow(name: string): Promise<{ jobs: Record<string, Job> }> {
  return parse(
    await readFile(new URL(`../../../../.github/workflows/${name}.yml`, import.meta.url), 'utf8'),
  );
}

function bridge(job: Job): Step {
  return job.steps.find((step) => step.with?.mode === 'upload')!;
}

describe('installation publishing order', () => {
  it('resolves the staged local CLI before publishing global binaries and moving latest', async () => {
    const { steps } = (await workflow('release')).jobs.Release;
    const publishLocal = steps.findIndex((step) => step.run?.includes('--tag staging'));
    const waitLocal = steps.findIndex(
      (step) => step.name === 'Wait for the local CLI to propagate',
    );
    const lock = steps.findIndex(
      (step) => step.run === 'node ./packages/cli/generate-install-lockfile.ts',
    );
    const save = steps.findIndex((step) => step.with?.name === 'installation-lockfile');
    const publishGlobal = steps.findIndex((step) => step.run?.includes('--phase cli'));
    const promote = steps.findIndex((step) => step.run?.startsWith('npm dist-tag add '));
    expect(publishLocal).toBeGreaterThan(-1);
    expect(waitLocal).toBeGreaterThan(publishLocal);
    expect(lock).toBeGreaterThan(waitLocal);
    expect(save).toBeGreaterThan(lock);
    expect(publishGlobal).toBeGreaterThan(save);
    expect(promote).toBeGreaterThan(publishGlobal);
    expect(steps[promote].run).toBe('npm dist-tag add "vite-plus@${VERSION}" latest');
    expect(steps.some((step) => step.run?.includes('local-npm-registry'))).toBe(false);
    expect(steps.some((step) => step.run === 'npm install --global npm@11.21.0')).toBe(true);
    expect(steps[save].with?.path).toBe('binary-release/vp-install-lock.yaml');
  });

  it('separates preview publication from pnpm resolution and moves the PR tag last', async () => {
    const build = await workflow('publish-preview');
    const publish = await workflow('publish-preview-register');
    const local = publish.jobs['publish-local'];
    const prepare = publish.jobs['prepare-global'];
    const global = publish.jobs.publish;
    expect(local.needs).toBe('authorize');
    expect(local.environment).toContain('preview-build-release');
    expect(prepare.needs).toContain('publish-local');
    expect(global.needs).toContain('prepare-global');
    expect(global.needs).toContain('publish-local');
    expect(prepare.permissions).toEqual({ contents: 'read', actions: 'read' });
    for (const job of [local, global]) {
      expect(job.permissions?.['id-token']).toBe('write');
      expect(job.steps.some((step) => step.run)).toBe(false);
      expect(job.steps[0].name).toBe('Re-check authorization before publish');
      expect(bridge(job).with?.sha).toBe('${{ github.event.workflow_run.head_sha }}');
    }
    expect(bridge(local).with?.['pr-url']).toBeUndefined();
    expect(bridge(global).with?.['pr-url']).toBe('${{ needs.authorize.outputs.pr-url }}');
    expect(bridge(global).with?.['workspace-packages']).toBe('@voidzero-dev/vite-plus-cli-*');

    const packs = build.jobs.publish.steps.filter((step) => step.with?.mode === 'pack');
    expect(packs).toHaveLength(2);
    expect(packs[0].with?.packages).not.toContain('packages/cli/cli-npm/*');
    expect(packs[1].with?.packages).toBe('packages/cli/cli-npm/*');
    expect(new Set([...packs, bridge(local), bridge(global)].map((step) => step.uses)).size).toBe(
      1,
    );

    const generate = prepare.steps.find((step) =>
      step.run?.includes('generate-preview-install-lockfile'),
    )!;
    expect(generate.if).toBe("steps.prepared.outputs.exists != 'true'");
    const save = prepare.steps.find((step) => step.with?.name === 'prepared-preview-cli')!;
    expect(save.if).toBe(generate.if);
    expect(
      global.steps.find((step) => step.with?.name === 'prepared-preview-cli')?.with?.['run-id'],
    ).toBe('${{ github.run_id }}');
    expect(
      prepare.steps.find((step) => step.with?.name === 'bridge-cli-packages')?.with?.['run-id'],
    ).toBe('${{ github.event.workflow_run.id }}');
  });
});
