import { execFileSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { stripVTControlCharacters } from 'node:util';

import { describe, expect, it, vi } from 'vitest';

import {
  formatPnpmInstallOutput,
  pnpmProbeEnv,
  watchPnpmResolution,
} from '../pnpm-install-progress.ts';

function stage(prefix: string, stage: string) {
  return JSON.stringify({ name: 'pnpm:stage', prefix, stage });
}

function lifecycle(wd: string, event: { script: string } | { exitCode: number }) {
  return JSON.stringify({ name: 'pnpm:lifecycle', wd, stage: 'prepare', ...event });
}

it('passes both hook settings to a real probe child with case-insensitive environment keys', () => {
  const env = {
    ...process.env,
    NODE_OPTIONS: '--require=missing-probe-preload',
    NPM_CONFIG_IGNORE_PNPMFILE: 'false',
    Pnpm_Config_Ignore_Pnpmfile: 'false',
  };
  const probeEnv = pnpmProbeEnv(env);
  const output = execFileSync(
    process.execPath,
    [
      '-p',
      'JSON.stringify([process.env.npm_config_ignore_pnpmfile, process.env.pnpm_config_ignore_pnpmfile])',
    ],
    { env: probeEnv, encoding: 'utf8' },
  );
  expect(JSON.parse(output)).toEqual(['true', 'true']);
  expect(probeEnv).not.toHaveProperty('NPM_CONFIG_IGNORE_PNPMFILE');
  expect(probeEnv).not.toHaveProperty('Pnpm_Config_Ignore_Pnpmfile');
  expect(env.NODE_OPTIONS).toBe('--require=missing-probe-preload');
  expect(env.NPM_CONFIG_IGNORE_PNPMFILE).toBe('false');
});

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

  it('defers the idle timer for retries scheduled during resolution', () => {
    const watch = watchPnpmResolution();
    const defer = vi.fn();
    const retry = JSON.stringify({ name: 'pnpm:request-retry', timeout: 360_000 });
    expect(watch(retry, defer)).toBe(false);
    expect(defer).not.toHaveBeenCalled();

    expect(watch(stage('/workspace/a', 'resolution_started'), defer)).toBe(true);
    expect(watch(retry, defer)).toBe(true);
    expect(defer).toHaveBeenCalledExactlyOnceWith(360_000);

    expect(watch(lifecycle('/store/git-a', { script: 'npm install' }), defer)).toBe(false);
    expect(watch(retry, defer)).toBe(false);
    expect(defer).toHaveBeenCalledTimes(2);
    expect(watch(lifecycle('/store/git-a', { exitCode: 0 }), defer)).toBe(true);
    expect(watch(stage('/workspace/a', 'resolution_done'), defer)).toBe(false);
    expect(watch(retry, defer)).toBe(false);
    expect(defer).toHaveBeenCalledTimes(2);
  });
});

describe('formatPnpmInstallOutput', () => {
  it('keeps silent script output immediately followed by a terminal pnpm record', async () => {
    const text = '{"time":0,"hostname":"fixture"} status from postinstall';
    const record = JSON.stringify({
      time: 1720000000000,
      hostname: 'test-host',
      pid: 123,
      level: 'error',
      name: 'pnpm',
      code: 'ELIFECYCLE',
      err: { name: 'pnpm', message: 'Exit status 7', stack: 'internal stack' },
    });
    const output = `${text}${record}\n`;

    expect(await formatPnpmInstallOutput(output, '/project', 'silent')).toBe(text);
    expect(await formatPnpmInstallOutput(output, '/project', 'ndjson')).toBe(output);
  });

  it('replays the original records when pnpm is configured for NDJSON', async () => {
    const output = [
      stage('/project', 'resolution_started'),
      JSON.stringify({
        name: 'pnpm:package-requester',
        level: 'warn',
        message: 'Fetching https://registry.example.com/package.tgz?token=fixture-query failed!',
      }),
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

  it('recovers the error code from a truncated pnpm version diagnostic', async () => {
    const code = 'ERR_PNPM_NO_MATCHING_VERSION';
    const output =
      '{"time":1720000000000,"hostname":"test-host","pid":123,"level":"error","name":"pnpm",' +
      `"code":"${code}","packageMeta":{"versions":{"1.0.0":{"description":"unfinished`;
    const message = 'No matching version found. pnpm did not finish writing the error details.';

    expect(await formatPnpmInstallOutput(output, '/project')).toBe(`${code} ${message}`);
    expect(JSON.parse(await formatPnpmInstallOutput(output, '/project', 'json'))).toEqual({
      error: { code, message },
    });
    expect(await formatPnpmInstallOutput(output, '/project', 'ndjson')).toBe(output);
  });

  it('keeps inherited text before a truncated error and ignores codes in its metadata', async () => {
    const inherited = 'status from postinstall';
    const error =
      '{"time":1720000000000,"hostname":"test-host","pid":123,"level":"error","name":"pnpm",' +
      '"packageMeta":{"code":"ERR_NESTED","description":"unfinished';
    const message = 'pnpm did not finish writing the error details.';

    expect(await formatPnpmInstallOutput(inherited + error, '/project')).toBe(
      `${inherited}\n${message}`,
    );
    expect(await formatPnpmInstallOutput(inherited + error, '/project', 'silent')).toBe(inherited);
    expect(JSON.parse(await formatPnpmInstallOutput(error, '/project', 'json'))).toEqual({
      error: { code: 'pnpm', message },
    });
  });

  it('keeps only the terminal error after pnpm repairs a lockfile', async () => {
    const repairedLockfile = JSON.stringify({
      name: 'pnpm',
      level: 'error',
      err: {
        name: 'Error',
        code: 'ERR_PNPM_LOCKFILE_MISSING_DEPENDENCY',
        message: 'The lockfile is broken! Resolution step will be performed to fix it.',
      },
    });
    const terminalError = {
      code: 'ELIFECYCLE',
      message: 'fixture@ postinstall: `node fail.cjs`\nExit status 7',
    };
    const output = [
      repairedLockfile,
      stage('/project', 'resolution_started'),
      stage('/project', 'resolution_done'),
      JSON.stringify({
        name: 'pnpm',
        level: 'error',
        code: terminalError.code,
        err: { name: 'pnpm', ...terminalError },
      }),
    ].join('\n');

    expect(await formatPnpmInstallOutput(repairedLockfile, '/project', 'json')).toBe('');
    expect(JSON.parse(await formatPnpmInstallOutput(output, '/project', 'json'))).toEqual({
      error: terminalError,
    });
  });

  it.each([
    ['info', true],
    ['warn', true],
    ['error', false],
    ['silent', false],
    ['json', false],
  ] as const)(
    'reports deprecations at the %s log level when warnings are enabled',
    async (mode, showWarnings) => {
      const cwd = resolve('workspace');
      const deprecation = {
        name: 'pnpm:deprecation',
        level: 'debug',
        prefix: join(cwd, 'packages', 'member'),
        pkgName: 'inflight',
        pkgVersion: '1.0.6',
        deprecated: 'This package is no longer supported.',
        depth: 0,
      };
      const output = [
        JSON.stringify(deprecation),
        JSON.stringify({
          ...deprecation,
          pkgName: 'old-transitive',
          pkgVersion: '2.0.0',
          depth: 1,
        }),
        stage(cwd, 'resolution_done'),
      ].join('\n');

      expect(await formatPnpmInstallOutput(output, cwd, mode)).toBe(
        showWarnings
          ? 'packages/member: WARN deprecated inflight@1.0.6: This package is no longer supported.\n' +
              'WARN 1 deprecated subdependencies found: old-transitive@2.0.0'
          : '',
      );
    },
  );

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
      expect(formatted).toContain('strict-peer-dependencies=false');
      expect(formatted).not.toContain('auto-install-peers=true');
    } finally {
      rmSync(cwd, { recursive: true, force: true });
    }
  });

  it.each([false, true])(
    'shows the appropriate hints for an optional=%s missing peer',
    async (optional) => {
      const code = 'ERR_PNPM_PEER_DEP_ISSUES';
      const message = 'Unmet peer dependencies';
      const output = JSON.stringify({
        name: 'pnpm',
        level: 'error',
        code,
        err: { name: 'pnpm', message },
        issuesByProjects: {
          '.': {
            bad: {},
            missing: {
              react: [
                {
                  parents: [{ name: 'react-dom', version: '18.3.1' }],
                  wantedRange: '^18.3.1',
                  optional,
                },
              ],
            },
            conflicts: [],
            intersections: { react: '^18.3.1' },
          },
        },
      });
      const text = await formatPnpmInstallOutput(output, '/project', 'warn');
      expect(text).toContain('missing peer');
      expect(text).toContain('strict-peer-dependencies=false');
      expect(text.includes('auto-install-peers=true')).toBe(!optional);
      expect(await formatPnpmInstallOutput(output, '/project', 'silent')).toBe('');
      expect(JSON.parse(await formatPnpmInstallOutput(output, '/project', 'json'))).toEqual({
        error: { code, message },
      });
      expect(await formatPnpmInstallOutput(output, '/project', 'ndjson')).toBe(output);
    },
  );

  it.each([
    [
      [
        { name: 'requester', version: '1.0.0' },
        { name: 'parent', version: '2.0.0' },
      ],
      'In packages/app\nWhile installing the dependencies of requester@1.0.0\n  at parent@2.0.0',
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
    ['warn', 'dependency warning\nELIFECYCLE Command failed'],
    ['error', 'ELIFECYCLE Command failed'],
    ['silent', ''],
  ] as const)('filters pnpm output at the %s log level', async (logLevel, detail) => {
    const script = { name: 'pnpm:lifecycle', stage: 'postinstall', wd: '/dep', level: 'debug' };
    const output = [
      'plain launcher output',
      { name: 'pnpm', level: 'warn', message: 'dependency warning' },
      { name: 'pnpm:progress', status: 'resolved', level: 'debug' },
      { ...script, script: 'node install.cjs' },
      { ...script, line: 'working', stdio: 'stdout' },
      { ...script, line: 'setup failed', stdio: 'stderr' },
      { ...script, exitCode: 7 },
      {
        name: 'pnpm',
        level: 'error',
        code: 'ELIFECYCLE',
        err: { name: 'pnpm', message: 'Command failed' },
      },
    ]
      .map((line) => (typeof line === 'string' ? line : JSON.stringify(line)))
      .join('\n');

    expect(await formatPnpmInstallOutput(output, '/project', logLevel)).toBe(
      ['plain launcher output', detail].filter(Boolean).join('\n'),
    );
  });

  it('suppresses terminal errors in silent mode', async () => {
    const output = JSON.stringify({
      name: 'pnpm',
      level: 'error',
      code: 'EACCES',
      errno: -13,
      err: { name: 'pnpm', code: 'EACCES', message: 'Store is not writable' },
    });
    expect(await formatPnpmInstallOutput(output, '/project', 'silent')).toBe('');
  });

  it('replays readPackage hook messages at the default log level', async () => {
    const output = JSON.stringify({
      name: 'pnpm:hook',
      level: 'debug',
      hook: 'readPackage',
      prefix: '/project',
      message: 'fixture hook rewrote dependency',
    });
    expect(await formatPnpmInstallOutput(output, '/project')).toBe(
      'readPackage: fixture hook rewrote dependency',
    );
    expect(await formatPnpmInstallOutput(output, '/project', 'warn')).toBe('');
  });

  it('keeps inherited stderr once and ignores structured script output in silent mode', async () => {
    const script = { name: 'pnpm:lifecycle', stage: 'postinstall', wd: '/project', level: 'debug' };
    const output = [
      'postinstall failed',
      JSON.stringify({ ...script, line: 'postinstall failed', stdio: 'stderr' }),
      JSON.stringify({ ...script, exitCode: 7 }),
    ].join('\n');

    expect(await formatPnpmInstallOutput(output, '/project', 'silent')).toBe('postinstall failed');
  });

  it('shows the lockfile diagnostic at debug level in output order', async () => {
    const diagnostic = 'No entry for "fixture@1.0.0" in pnpm-lock.yaml';
    const output = [
      { name: 'pnpm:lockfile', level: 'debug', message: diagnostic },
      { name: 'pnpm:install-check', level: 'debug', message: 'internal platform check' },
      { name: 'pnpm', level: 'error', message: 'install failed' },
    ]
      .map((log) => JSON.stringify(log))
      .join('\n');

    expect(await formatPnpmInstallOutput(output, '/project', 'debug')).toBe(
      `${diagnostic}\ninstall failed`,
    );
    expect(await formatPnpmInstallOutput(output, '/project', 'info')).toBe('install failed');
  });

  it.each(['info', 'warn', 'debug'] as const)(
    'filters internal package fetch warnings at the %s log level',
    async (mode) => {
      const output = [
        {
          name: 'pnpm:package-requester',
          level: 'warn',
          message: 'Fetching https://registry.example.com/package.tgz?token=fixture-query failed!',
        },
        { name: 'pnpm', level: 'warn', message: 'The package store was moved' },
      ]
        .map((log) => JSON.stringify(log))
        .join('\n');

      expect(await formatPnpmInstallOutput(output, '/project', mode)).toBe(
        'The package store was moved',
      );
    },
  );

  it('redacts registry URLs in visible diagnostics', async () => {
    const cwd = resolve('workspace');
    const url =
      'https://fixture-user:fixture-password@registry.example.com/team/package?token=fixture-query#fixture-fragment';
    const publicUrl = 'https://registry.example.com/team/package';
    const output = [
      {
        name: 'pnpm:registry',
        level: 'warn',
        prefix: join(cwd, 'packages', 'publisher'),
        message: `Registry rejected ${url}: Forbidden`,
      },
      { name: 'pnpm', level: 'info', message: `Using registry at "${url}"` },
      {
        name: 'pnpm:deprecation',
        level: 'debug',
        pkgName: 'old-package',
        pkgVersion: '1.0.0',
        deprecated: `See ${url} for the replacement`,
        depth: 0,
      },
      {
        name: 'pnpm',
        level: 'error',
        code: 'ERR_PNPM_FETCH_403',
        err: { message: `GET ${url}: Forbidden` },
        pkgsStack: [{ name: 'parent-package', version: url }],
        hint: '  An authorization header was used: Bearer fixture-auth[hidden]',
      },
    ]
      .map((log) => JSON.stringify(log))
      .join('\n');

    expect(await formatPnpmInstallOutput(output, cwd)).toBe(
      [
        `packages/publisher: Registry rejected ${publicUrl}: Forbidden`,
        `Using registry at "${publicUrl}"`,
        `WARN deprecated old-package@1.0.0: See ${publicUrl} for the replacement`,
        `ERR_PNPM_FETCH_403 GET ${publicUrl}: Forbidden`,
        `While installing the dependencies of parent-package@${publicUrl}`,
        '  An authorization header was used: [hidden]',
        'Check the registry and authentication settings in .npmrc. Run "pnpm login" if you need to sign in.',
      ].join('\n'),
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

  it.each([
    ['https://private.example.com/package.tgz', undefined],
    ['https://private.example.com/team/@vp-pnpm-auth%2Fpackage', '@vp-pnpm-auth/package'],
  ])(
    'shows the auth failure request for %s without guessing its registry config',
    async (url, pkgName) => {
      const cwd = mkdtempSync(join(tmpdir(), 'vp-pnpm-tarball-auth-'));
      try {
        writeFileSync(join(cwd, '.npmrc'), 'registry=https://default.example.com/\n');
        const output = JSON.stringify({
          name: 'pnpm',
          level: 'error',
          code: 'ERR_PNPM_FETCH_401',
          err: { message: `GET ${url}: Unauthorized - 401` },
          pkgName,
          request: { url },
        });

        expect(await formatPnpmInstallOutput(output, cwd)).toBe(
          `ERR_PNPM_FETCH_401 GET ${url}: Unauthorized - 401\n` +
            'Check the registry and authentication settings in .npmrc. Run "pnpm login" if you need to sign in.',
        );
      } finally {
        rmSync(cwd, { recursive: true, force: true });
      }
    },
  );

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

  it('groups concurrent lifecycle output when requested and can hide script line prefixes', async () => {
    const cwd = resolve('fixture');
    const event = (pkg: string, details: Record<string, unknown>) => ({
      name: 'pnpm:lifecycle',
      level: 'debug',
      wd: join(cwd, 'packages', pkg),
      stage: 'postinstall',
      ...details,
    });
    const output = [
      event('a', { script: 'node check.cjs' }),
      event('b', { script: 'node check.cjs' }),
      event('a', { line: 'a early', stdio: 'stdout' }),
      event('b', { line: 'b early', stdio: 'stdout' }),
      event('b', { line: 'b late', stdio: 'stderr' }),
      event('b', { exitCode: 7 }),
      event('a', { line: 'a late', stdio: 'stdout' }),
      event('a', { exitCode: 0 }),
    ]
      .map((log) => JSON.stringify(log))
      .join('\n');

    expect(await formatPnpmInstallOutput(output, cwd, 'info', { aggregateOutput: true })).toBe(
      [
        'packages/b postinstall$ node check.cjs',
        'packages/b postinstall: b early',
        'packages/b postinstall: b late',
        'packages/b postinstall: failed with exit code 7',
        'packages/a postinstall$ node check.cjs',
        'packages/a postinstall: a early',
        'packages/a postinstall: a late',
      ].join('\n'),
    );
    expect(await formatPnpmInstallOutput(output, cwd, 'info', { hideLifecyclePrefix: true })).toBe(
      [
        'packages/a postinstall$ node check.cjs',
        'packages/b postinstall$ node check.cjs',
        'a early',
        'b early',
        'b late',
        'packages/b postinstall: failed with exit code 7',
        'a late',
      ].join('\n'),
    );
  });
});
