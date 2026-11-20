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
  it('replays the original records when pnpm is configured for NDJSON', async () => {
    const output = [
      stage('/project', 'resolution_started'),
      JSON.stringify({ name: 'pnpm:lifecycle', level: 'debug', line: 'setup failed' }),
      '',
    ].join('\n');

    expect(await formatPnpmInstallOutput(output, '/project', 'ndjson')).toBe(output);
  });

  it('preserves the pnpm error object in JSON and parseable output', async () => {
    const error = {
      code: 'ELIFECYCLE',
      message: 'fixture@ postinstall: `node fail.cjs`\nExit status 7',
    };
    const output = [
      stage('/project', 'resolution_started'),
      JSON.stringify({
        name: 'pnpm',
        level: 'error',
        code: error.code,
        err: { name: 'pnpm', message: error.message, stack: 'internal stack' },
      }),
    ].join('\n');

    expect(JSON.parse(await formatPnpmInstallOutput(output, '/project', 'json'))).toEqual({
      error,
    });
  });

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

  it('shows the published versions and tags when a requested version is missing', async () => {
    const output = JSON.stringify({
      name: 'pnpm',
      level: 'error',
      code: 'ERR_PNPM_NO_MATCHING_VERSION',
      err: { message: 'No matching version found for @scope/package@999.0.0' },
      packageMeta: {
        name: '@scope/package',
        'dist-tags': { latest: '1.2.3', next: '2.0.0-beta.1' },
        versions: { '1.2.3': {}, '2.0.0-beta.1': {} },
      },
    });

    expect(await formatPnpmInstallOutput(output, '/project')).toBe(
      [
        'ERR_PNPM_NO_MATCHING_VERSION No matching version found for @scope/package@999.0.0',
        'The latest release of @scope/package is "1.2.3".',
        'Other release tags',
        '  next: 2.0.0-beta.1',
        'Run "pnpm view @scope/package versions" to see all 2 published versions.',
      ].join('\n'),
    );
  });

  it.each([
    ['warn', 'dependency warning\n'],
    ['error', ''],
    ['silent', 'setup failed\n'],
  ] as const)('filters pnpm output at the %s log level', async (logLevel, detail) => {
    const output = [
      'plain launcher output',
      { name: 'pnpm', level: 'warn', message: 'dependency warning' },
      { name: 'pnpm:progress', status: 'resolved', level: 'debug' },
      {
        name: 'pnpm:lifecycle',
        stage: 'postinstall',
        script: 'node install.cjs',
        level: 'debug',
      },
      {
        name: 'pnpm:lifecycle',
        stage: 'postinstall',
        line: 'working',
        stdio: 'stdout',
        level: 'debug',
      },
      {
        name: 'pnpm:lifecycle',
        stage: 'postinstall',
        line: 'setup failed',
        stdio: 'stderr',
        level: 'debug',
      },
      { name: 'pnpm:lifecycle', stage: 'postinstall', exitCode: 7, level: 'debug' },
      { name: 'pnpm', level: 'error', code: 'ELIFECYCLE', err: { message: 'Command failed' } },
    ]
      .map((line) => (typeof line === 'string' ? line : JSON.stringify(line)))
      .join('\n');

    expect(await formatPnpmInstallOutput(output, '/project', logLevel)).toBe(
      `plain launcher output\n${detail}ELIFECYCLE Command failed`,
    );
  });

  it.each([401, 403])('shows safe registry configuration for HTTP %s', async (status) => {
    const cwd = mkdtempSync(join(tmpdir(), 'vp-pnpm-auth-context-'));
    try {
      const registry = 'https://fixture-user:fixture-password@registry.example.com/team/';
      const url = `${registry}@vp-pnpm-auth%2Fpackage?token=fixture-query`;
      writeFileSync(join(cwd, 'package.json'), '{"name":"fixture"}');
      writeFileSync(
        join(cwd, '.npmrc'),
        `@vp-pnpm-auth:registry=${registry}?token=fixture-config\n`,
      );
      const output = JSON.stringify({
        name: 'pnpm',
        level: 'error',
        code: `ERR_PNPM_FETCH_${status}`,
        err: { message: `GET ${url}: Registry refused access` },
        pkgName: '@vp-pnpm-auth/package',
        request: { url },
        hint: 'An authorization header was used: Bearer fixture-auth[hidden]',
      });
      const formatted = await formatPnpmInstallOutput(output, cwd);

      expect(formatted).toContain(
        `ERR_PNPM_FETCH_${status} GET https://registry.example.com/team/@vp-pnpm-auth%2Fpackage`,
      );
      expect(formatted).toContain('@vp-pnpm-auth:registry=https://registry.example.com/team/');
      expect(formatted).toContain('An authorization header was used: [hidden]');
      expect(formatted).toContain('authentication settings in .npmrc');
      expect(formatted).not.toMatch(/fixture-(user|password|query|config|auth)/);
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  it('uses the tarball request when the auth failure has no package name', async () => {
    const cwd = mkdtempSync(join(tmpdir(), 'vp-pnpm-tarball-auth-'));
    try {
      const url = 'https://private.example.com/package.tgz';
      writeFileSync(join(cwd, '.npmrc'), 'registry=https://default.example.com/\n');
      const output = JSON.stringify({
        name: 'pnpm',
        level: 'error',
        code: 'ERR_PNPM_FETCH_401',
        err: { message: `GET ${url}: Unauthorized - 401` },
        request: { url },
      });

      expect(await formatPnpmInstallOutput(output, cwd)).toBe(
        `ERR_PNPM_FETCH_401 GET ${url}: Unauthorized - 401\n` +
          'Check the registry and authentication settings in .npmrc. Run "pnpm login" if you need to sign in.',
      );
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  it('shows only public URL components in retries and final fetch errors', async () => {
    const publicUrl = 'https://registry.example.com/team/package';
    const requestUrl = 'https://fixture-user:fixture-password@registry.example.com/team/package';
    const suffix = '?token=fixture-query#fixture-fragment';
    const output = [
      JSON.stringify({
        name: 'pnpm:request-retry',
        level: 'debug',
        method: 'GET',
        url: requestUrl + suffix,
        error: { message: `request to ${publicUrl}${suffix} failed` },
        attempt: 1,
        maxRetries: 1,
        timeout: 50,
      }),
      JSON.stringify({
        name: 'pnpm',
        level: 'error',
        code: 'ERR_PNPM_FETCH_503',
        err: { message: `GET ${requestUrl}${suffix}: Service Unavailable - 503` },
        request: { url: requestUrl + suffix },
        hint: `Check "${requestUrl}${suffix}"`,
      }),
      JSON.stringify({
        name: 'pnpm',
        level: 'error',
        code: 'ERR_PNPM_META_FETCH_FAIL',
        err: { message: `GET ${requestUrl}${suffix}: request to ${publicUrl}${suffix} failed` },
      }),
    ].join('\n');

    expect(await formatPnpmInstallOutput(output, '/project')).toBe(
      [
        `GET ${publicUrl} error (request to ${publicUrl} failed). Will retry in 50ms. 1 retry left.`,
        `ERR_PNPM_FETCH_503 GET ${publicUrl}: Service Unavailable - 503`,
        `Check "${publicUrl}"`,
        `ERR_PNPM_META_FETCH_FAIL GET ${publicUrl}: request to ${publicUrl} failed`,
      ].join('\n'),
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

  it.each([
    [{ status: 503 }, '503', 2, 2, '1 retry left.'],
    [{ code: 'ECONNRESET' }, 'ECONNRESET', 1, 2, '2 retries left.'],
  ])(
    'keeps the request and timing for registry retries with %o',
    async (error, reason, attempt, maxRetries, retries) => {
      const url = 'http://127.0.0.1:4500/@scope%2Fpackage';
      const output = JSON.stringify({
        name: 'pnpm:request-retry',
        level: 'debug',
        method: 'GET',
        url,
        error,
        attempt,
        maxRetries,
        timeout: 50,
      });

      for (const mode of ['info', 'warn'] as const) {
        expect(await formatPnpmInstallOutput(output, '/project', mode)).toBe(
          `GET ${url} error (${reason}). Will retry in 50ms. ${retries}`,
        );
      }
      expect(await formatPnpmInstallOutput(output, '/project', 'error')).toBe('');
      expect(await formatPnpmInstallOutput(output, '/project', 'silent')).toBe('');
    },
  );

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
