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
  it('keeps progress and readable errors while dropping verbose pnpm records', () => {
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

    expect(formatPnpmInstallOutput(output)).toBe(
      'launcher warning\n' +
        'ERR_PNPM_LINKED_PKG_DIR_NOT_FOUND Missing local dependency\n' +
        'Progress: resolved 1, reused 0, downloaded 1, added 0',
    );
  });
});
