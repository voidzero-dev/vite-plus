import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { stripVTControlCharacters } from 'node:util';

import { describe, expect, it } from 'vitest';

import { formatPnpmInstallOutput, watchPnpmResolution } from '../pnpm-install-progress.ts';

function stage(prefix: string, stage: string) {
  return JSON.stringify({ name: 'pnpm:stage', prefix, stage });
}

function lifecycle(wd: string, event: { script: string } | { exitCode: number }) {
  return JSON.stringify({ name: 'pnpm:lifecycle', wd, stage: 'prepare', ...event });
}

describe('watchPnpmResolution', () => {
  it('tracks concurrent resolutions and suspends the timer while any lifecycle script runs', () => {
    const watch = watchPnpmResolution();
    expect(watch('> node dev-preinstall.cjs')).toBe(false);
    expect(watch(stage('/workspace/a', 'resolution_started'))).toBe(true);
    expect(watch(stage('/workspace/b', 'resolution_started'))).toBe(true);
    expect(watch(lifecycle('/store/git-a', { script: 'npm install' }))).toBe(false);
    expect(watch(lifecycle('/store/git-b', { script: 'npm install' }))).toBe(false);
    expect(watch(lifecycle('/store/git-a', { exitCode: 0 }))).toBe(false);
    expect(watch(lifecycle('/store/git-b', { exitCode: 1 }))).toBe(true);
    expect(watch(stage('/workspace/a', 'resolution_done'))).toBe(true);
    expect(watch(stage('/workspace/b', 'resolution_done'))).toBe(false);
    expect(watch('> node postinstall.cjs')).toBe(false);
  });
});

describe('formatPnpmInstallOutput', () => {
  it('keeps the real peer conflict and respects the root peer dependency rules', async () => {
    const request = { parents: [{ name: 'react-dom', version: '18.3.1' }], wantedRange: '^18.3.1' };
    const allowed = {
      parents: [{ name: 'legacy-plugin', version: '1.0.0' }],
      wantedRange: '^19',
      foundVersion: '17.0.2',
    };
    const output = JSON.stringify({
      name: 'pnpm',
      level: 'error',
      code: 'ERR_PNPM_PEER_DEP_ISSUES',
      err: { message: 'Unmet peer dependencies' },
      issuesByProjects: {
        '.': {
          bad: { react: [{ ...request, foundVersion: '17.0.2' }, allowed] },
          missing: {},
          conflicts: [],
          intersections: {},
        },
      },
    });

    const cwd = mkdtempSync(join(tmpdir(), 'vp-pnpm-peer-rules-'));
    try {
      writeFileSync(
        join(cwd, 'package.json'),
        JSON.stringify({
          pnpm: {
            peerDependencyRules: { allowedVersions: { 'legacy-plugin@^1>react': '17' } },
          },
        }),
      );
      const formatted = stripVTControlCharacters(await formatPnpmInstallOutput(output, cwd));
      expect(formatted).toContain('ERR_PNPM_PEER_DEP_ISSUES Unmet peer dependencies');
      expect(formatted).toContain('react-dom 18.3.1');
      expect(formatted).toContain('unmet peer react@^18.3.1: found 17.0.2');
      expect(formatted).not.toContain('legacy-plugin');
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  it.each([
    [
      [
        { name: 'requester', version: '1.0.0' },
        { name: 'parent', version: '2.0.0' },
      ],
      'While installing the dependencies of requester@1.0.0\n  at parent@2.0.0',
    ],
    [[], 'While installing a direct dependency of /project/packages/app'],
  ])('keeps the dependency that requested an unavailable version', async (pkgsStack, context) => {
    const error = JSON.stringify({
      name: 'pnpm',
      level: 'error',
      code: 'ERR_PNPM_NO_MATCHING_VERSION',
      err: { message: 'No matching version found for picocolors@999.0.0' },
      pkgsStack,
      prefix: '/project/packages/app',
    });
    expect(await formatPnpmInstallOutput(error, '/project')).toBe(
      `ERR_PNPM_NO_MATCHING_VERSION No matching version found for picocolors@999.0.0\n${context}`,
    );
  });

  it('keeps progress and readable errors while dropping verbose pnpm records', async () => {
    const output = [
      'launcher warning',
      JSON.stringify({ name: 'pnpm:package-manifest', level: 'debug', initial: {} }),
      JSON.stringify({
        name: 'pnpm:install-check',
        level: 'warn',
        message: 'Unsupported platform for an optional package',
      }),
      JSON.stringify({ name: 'pnpm:progress', status: 'resolved' }),
      JSON.stringify({ name: 'pnpm:progress', status: 'fetched' }),
      JSON.stringify({
        name: 'pnpm',
        level: 'error',
        code: 'ERR_PNPM_LINKED_PKG_DIR_NOT_FOUND',
        err: { message: 'Missing local dependency', stack: 'internal stack' },
      }),
    ].join('\n');

    expect(await formatPnpmInstallOutput(output, '/project')).toBe(
      'launcher warning\n' +
        'ERR_PNPM_LINKED_PKG_DIR_NOT_FOUND Missing local dependency\n' +
        'Progress: resolved 1, reused 0, downloaded 1, added 0',
    );
  });

  it('preserves the source location for unexpected errors from package hooks', async () => {
    const stack =
      "pnpm: Cannot read properties of undefined (reading 'name')\n" +
      '    at readPackage (/project/.pnpmfile.cjs:3:24)';
    const output = JSON.stringify({
      name: 'pnpm',
      level: 'error',
      err: { message: "Cannot read properties of undefined (reading 'name')", stack },
    });

    expect(await formatPnpmInstallOutput(output, '/project')).toBe(stack);
  });

  it('identifies the directories and failure of concurrent install scripts', async () => {
    const cwd = resolve('fixture');
    const lifecycleEvent = (pkg: string, event: Record<string, unknown>) =>
      JSON.stringify({
        name: 'pnpm:lifecycle',
        level: 'debug',
        stage: 'postinstall',
        wd: join(cwd, 'node_modules', pkg),
        ...event,
      });
    const output = [
      lifecycleEvent('a', { script: 'node postinstall.cjs' }),
      lifecycleEvent('b', { script: 'node postinstall.cjs' }),
      lifecycleEvent('a', { line: 'shared message' }),
      lifecycleEvent('b', { line: 'shared message' }),
      lifecycleEvent('a', { exitCode: 0 }),
      lifecycleEvent('b', { exitCode: 7 }),
    ].join('\n');

    expect(await formatPnpmInstallOutput(output, cwd)).toBe(
      [
        'node_modules/a postinstall$ node postinstall.cjs',
        'node_modules/b postinstall$ node postinstall.cjs',
        'node_modules/a postinstall: shared message',
        'node_modules/b postinstall: shared message',
        'node_modules/b postinstall: failed with exit code 7',
      ].join('\n'),
    );
  });
});
