import { readFile } from 'node:fs/promises';
import { runInNewContext } from 'node:vm';

import { describe, expect, it, vi } from 'vitest';
import { parse } from 'yaml';

interface Step {
  name?: string;
  id?: string;
  uses?: string;
  run?: string;
  if?: string;
  with?: Record<string, string | boolean>;
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
  it('allows build artifact uploads to repeat but preserves publication artifacts', async () => {
    for (const name of ['reusable-release-build', 'publish-preview']) {
      const { jobs } = await workflow(name);
      const uploads = Object.values(jobs).flatMap((job) =>
        (job.steps ?? []).filter((step) => step.uses?.startsWith('actions/upload-artifact@')),
      );
      expect(uploads.length).toBeGreaterThan(0);
      for (const upload of uploads) {
        expect(upload.with?.overwrite).toBe(true);
      }
    }
    const release = (await workflow('release')).jobs.Release;
    const lockfile = release.steps.find((step) => step.with?.name === 'installation-lockfile')!;
    expect(lockfile.with?.overwrite).not.toBe(true);
    expect(lockfile.if).toBe("steps.install-lockfile.outputs.restored != 'true'");
    const preview = (await workflow('publish-preview-register')).jobs['prepare-global'];
    const prepared = preview.steps.find((step) => step.with?.name === 'prepared-preview-cli')!;
    expect(prepared.with?.overwrite).not.toBe(true);
    expect(prepared.if).toBe("steps.prepared.outputs.exists != 'true'");
  });

  it.each(['missing', 'draft', 'published', 'unavailable'])(
    'handles a %s GitHub release when retrying asset publication',
    async (state) => {
      const { steps } = (await workflow('release')).jobs.Release;
      const check = steps.find((step) => step.id === 'github-release')!;
      const getReleaseByTag = vi.fn(async () => {
        if (state === 'missing' || state === 'unavailable') {
          throw Object.assign(new Error(state), { status: state === 'missing' ? 404 : 503 });
        }
        return { data: { draft: state === 'draft' } };
      });
      const setOutput = vi.fn();
      const result = runInNewContext(`(async () => { ${check.with!.script} })()`, {
        github: { rest: { repos: { getReleaseByTag } } },
        context: { repo: { owner: 'voidzero-dev', repo: 'vite-plus' } },
        core: { setOutput },
        process: { env: { VERSION: '1.4.0' } },
      }) as Promise<void>;
      if (state === 'unavailable') {
        await expect(result).rejects.toThrow('unavailable');
        expect(setOutput).not.toHaveBeenCalled();
      } else {
        await result;
        expect(setOutput).toHaveBeenCalledExactlyOnceWith(
          'published',
          String(state === 'published'),
        );
      }
      expect(getReleaseByTag).toHaveBeenCalledExactlyOnceWith({
        owner: 'voidzero-dev',
        repo: 'vite-plus',
        tag: 'v1.4.0',
      });
      for (const name of [
        'Create release body',
        'Create GitHub Release',
        'Publish GitHub Release',
      ]) {
        const step = steps.find((step) => step.name === name)!;
        expect(step.if).toBe("steps.github-release.outputs.published != 'true'");
      }
      const upload = steps.find((step) => step.name === 'Create GitHub Release')!;
      expect(upload.with?.overwrite_files).toBe(true);
      expect(upload.with?.fail_on_unmatched_files).toBe(true);
    },
  );

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
