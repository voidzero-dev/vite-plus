import { describe, expect, it } from 'vitest';
import { parse as parseYaml } from 'yaml';

import {
  parseSyncVersionsRequest,
  planSyncVersions,
  type SyncVersionsRequestV1,
  type SyncVersionsToolchain,
} from '../plan.ts';

const toolchain: SyncVersionsToolchain = {
  vitePlus: '0.4.0',
  vitest: '5.0.0',
};

function packageJsonRequest(contents: string): SyncVersionsRequestV1 {
  return parseSyncVersionsRequest({
    schemaVersion: 1,
    workspace: '.',
    manifests: [{ path: 'package.json', kind: 'packageJson', contents }],
  });
}

describe('planSyncVersions', () => {
  it('aligns existing Vite+ and Vitest install dependencies', () => {
    const before = `${JSON.stringify(
      {
        devDependencies: {
          '@vitest/browser-playwright': '4.1.11',
          '@vitest/coverage-v8': '^4.1.11',
          '@vitest/eslint-plugin': '1.6.0',
          '@vitest/coverage-c8': '0.33.0',
          '@voidzero-dev/vite-plus-core': '0.3.0',
          'vite-plus': '0.3.0',
          vitest: '~4.1.11',
        },
      },
      null,
      2,
    )}\n`;

    const plan = planSyncVersions(packageJsonRequest(before), toolchain);

    expect(plan.tool).toEqual({ name: 'vite-plus', version: '0.4.0' });
    expect(plan.replacements).toHaveLength(1);
    expect(plan.replacements[0].before).toBe(before);
    expect(JSON.parse(plan.replacements[0].after)).toEqual({
      devDependencies: {
        '@vitest/browser-playwright': '5.0.0',
        '@vitest/coverage-v8': '5.0.0',
        '@vitest/eslint-plugin': '1.6.0',
        '@vitest/coverage-c8': '0.33.0',
        '@voidzero-dev/vite-plus-core': '0.4.0',
        'vite-plus': '0.4.0',
        vitest: '5.0.0',
      },
    });
  });

  it('aligns managed npm aliases and preserves custom aliases', () => {
    const before = `${JSON.stringify(
      {
        devDependencies: {
          '@vitest/ui': 'npm:@vitest/ui@4.1.11',
          'vite-plus': 'npm:@scope/vite-plus-fork@0.3.0',
          vite: 'npm:@voidzero-dev/vite-plus-core@^0.3.0',
          vitest: 'npm:vitest@~4.1.11',
        },
      },
      null,
      2,
    )}\n`;

    const plan = planSyncVersions(packageJsonRequest(before), toolchain);

    expect(JSON.parse(plan.replacements[0].after)).toEqual({
      devDependencies: {
        '@vitest/ui': 'npm:@vitest/ui@5.0.0',
        'vite-plus': 'npm:@scope/vite-plus-fork@0.3.0',
        vite: 'npm:@voidzero-dev/vite-plus-core@0.4.0',
        vitest: 'npm:vitest@5.0.0',
      },
    });
  });

  it('updates referenced pnpm catalogs without replacing catalog protocols', () => {
    const packageJson = `${JSON.stringify(
      {
        devDependencies: {
          '@vitest/coverage-v8': 'catalog:',
          'vite-plus': 'catalog:toolchain',
        },
      },
      null,
      2,
    )}\n`;
    const pnpmWorkspace = `packages:\n  - packages/*\ncatalog:\n  '@vitest/coverage-v8': 4.1.11\ncatalogs:\n  toolchain:\n    vite: npm:@voidzero-dev/vite-plus-core@0.3.0\n    vite-plus: 0.3.0\n`;
    const request = parseSyncVersionsRequest({
      schemaVersion: 1,
      workspace: '.',
      manifests: [
        { path: 'package.json', kind: 'packageJson', contents: packageJson },
        { path: 'pnpm-workspace.yaml', kind: 'pnpmWorkspace', contents: pnpmWorkspace },
      ],
    });

    const plan = planSyncVersions(request, toolchain);

    expect(plan.replacements).toHaveLength(1);
    expect(plan.replacements[0].path).toBe('pnpm-workspace.yaml');
    expect(parseYaml(plan.replacements[0].after)).toEqual({
      packages: ['packages/*'],
      catalog: { '@vitest/coverage-v8': '5.0.0' },
      catalogs: {
        toolchain: {
          vite: 'npm:@voidzero-dev/vite-plus-core@0.4.0',
          'vite-plus': '0.4.0',
        },
      },
    });
  });

  it('updates Yarn catalogs without touching pnpm-only overrides or surrounding bytes', () => {
    const before =
      'nodeLinker: node-modules\r\ncatalog:\r\n    vitest: \'^4.1.11\' # keep this comment\r\ncatalogs:\r\n  toolchain:\r\n    vite: "npm:@voidzero-dev/vite-plus-core@0.3.0"\r\n    vite-plus: 0.3.0\r\noverrides:\r\n  vitest: 4.1.11\r\n';
    const request = parseSyncVersionsRequest({
      schemaVersion: 1,
      workspace: '.',
      manifests: [{ path: '.yarnrc.yml', kind: 'yarnRc', contents: before }],
    });

    const plan = planSyncVersions(request, toolchain);

    expect(plan.replacements).toHaveLength(1);
    expect(plan.replacements[0].kind).toBe('yarnRc');
    expect(plan.replacements[0].after).toBe(
      before
        .replace("vitest: '^4.1.11'", "vitest: '5.0.0'")
        .replace(
          'vite: "npm:@voidzero-dev/vite-plus-core@0.3.0"',
          'vite: "npm:@voidzero-dev/vite-plus-core@0.4.0"',
        )
        .replace('vite-plus: 0.3.0', 'vite-plus: 0.4.0'),
    );
  });

  it('returns a byte-identical Yarn no-op and ignores its overrides', () => {
    const before = 'catalog: { vitest: "5.0.0" }  # keep spacing\noverrides:\n  vitest: 4.1.11\n';
    const request = parseSyncVersionsRequest({
      schemaVersion: 1,
      workspace: '.',
      manifests: [{ path: 'config/.yarnrc.yml', kind: 'yarnRc', contents: before }],
    });

    expect(planSyncVersions(request, toolchain).replacements).toEqual([]);
  });

  it('changes only YAML scalar tokens and preserves surrounding formatting', () => {
    const before =
      "packages:\r\n- packages/*\r\ncatalog:\r\n    vitest: '^4.1.11' # keep this comment\r\ndescription: this-is-a-very-long-plain-scalar-that-must-not-be-folded-even-when-it-crosses-the-default-yaml-printer-width";
    const request = parseSyncVersionsRequest({
      schemaVersion: 1,
      workspace: '.',
      manifests: [{ path: 'pnpm-workspace.yaml', kind: 'pnpmWorkspace', contents: before }],
    });

    const plan = planSyncVersions(request, toolchain);

    expect(plan.replacements).toHaveLength(1);
    expect(plan.replacements[0].after).toBe(before.replace("vitest: '^4.1.11'", "vitest: '5.0.0'"));
  });

  it('preserves double-quoted YAML scalar style', () => {
    const before = 'catalog: { vitest: "4.1.11", unrelated: 42 }\n';
    const request = parseSyncVersionsRequest({
      schemaVersion: 1,
      workspace: '.',
      manifests: [{ path: 'pnpm-workspace.yaml', kind: 'pnpmWorkspace', contents: before }],
    });

    const plan = planSyncVersions(request, toolchain);

    expect(plan.replacements[0].after).toBe('catalog: { vitest: "5.0.0", unrelated: 42 }\n');
  });

  it.each([
    ['version: &version 4.1.11\ncatalog:\n  vitest: *version\n', 'Expected a string scalar'],
    ['catalog:\n  vitest: |\n    4.1.11\n', 'Unsupported YAML scalar style'],
    ['catalog: [', 'Invalid pnpm-workspace.yaml manifest'],
    ['- package\n', 'Invalid pnpm-workspace.yaml manifest'],
  ])('rejects an unsafe or invalid YAML manifest', (before, message) => {
    const request = parseSyncVersionsRequest({
      schemaVersion: 1,
      workspace: '.',
      manifests: [{ path: 'pnpm-workspace.yaml', kind: 'pnpmWorkspace', contents: before }],
    });

    expect(() => planSyncVersions(request, toolchain)).toThrow(message);
  });

  it.each([
    "packages:\r\n- packages/*\r\ncatalog:\r\n    vitest: '5.0.0'",
    'packages:\n    - packages/*\ncatalog:\n    vitest: 5.0.0\n',
    'catalog: { vitest: "5.0.0" }  # keep spacing\n',
  ])('returns a byte-identical YAML no-op', (before) => {
    const request = parseSyncVersionsRequest({
      schemaVersion: 1,
      workspace: '.',
      manifests: [{ path: 'pnpm-workspace.yaml', kind: 'pnpmWorkspace', contents: before }],
    });

    expect(planSyncVersions(request, toolchain).replacements).toEqual([]);
  });

  it('preserves custom npm aliases and non-registry protocols in a byte-identical no-op', () => {
    const before = `${JSON.stringify(
      {
        dependencies: {
          '@vitest/browser-playwright': 'file:../provider.tgz',
          vite: 'npm:@scope/vite-fork@6.0.0',
          'vite-plus': 'workspace:*',
          vitest: 'npm:@scope/vitest-fork@4.1.11',
        },
      },
      null,
      '\t',
    )}\r\n`;

    const plan = planSyncVersions(packageJsonRequest(before), toolchain);

    expect(plan.replacements).toEqual([]);
  });

  it('aligns versioned and parent-scoped override selectors', () => {
    const before = `${JSON.stringify(
      {
        pnpm: {
          overrides: {
            'app>vite-plus@0.3': '0.3.0',
            'app>@vitest/coverage-v8@4': '4.1.11',
          },
        },
        resolutions: {
          '**/@vitest/browser-playwright@4': '4.1.11',
        },
      },
      null,
      2,
    )}\n`;

    const plan = planSyncVersions(packageJsonRequest(before), toolchain);

    expect(JSON.parse(plan.replacements[0].after)).toEqual({
      pnpm: {
        overrides: {
          'app>vite-plus@0.3': '0.4.0',
          'app>@vitest/coverage-v8@4': '5.0.0',
        },
      },
      resolutions: {
        '**/@vitest/browser-playwright@4': '5.0.0',
      },
    });
  });

  it('aligns nested npm overrides and Bun workspace catalogs', () => {
    const before = `${JSON.stringify(
      {
        overrides: {
          app: {
            '.': '1.0.0',
            vitest: '^4.1.11',
            wrapper: { '@vitest/coverage-v8': '~4.1.11' },
          },
        },
        workspaces: {
          packages: ['packages/*'],
          catalog: { 'vite-plus': '^0.3.0' },
          catalogs: {
            test: { '@vitest/browser-playwright': '4.1.11' },
          },
        },
      },
      null,
      2,
    )}\n`;

    const plan = planSyncVersions(packageJsonRequest(before), toolchain);

    expect(JSON.parse(plan.replacements[0].after)).toMatchObject({
      overrides: {
        app: {
          '.': '1.0.0',
          vitest: '5.0.0',
          wrapper: { '@vitest/coverage-v8': '5.0.0' },
        },
      },
      workspaces: {
        catalog: { 'vite-plus': '0.4.0' },
        catalogs: {
          test: { '@vitest/browser-playwright': '5.0.0' },
        },
      },
    });
  });

  it('aligns package catalogs and ignores non-string dependency entries', () => {
    const before = `${JSON.stringify({
      dependencies: { unrelated: false },
      optionalDependencies: { vitest: '^4.1.11' },
      catalog: { 'vite-plus': '^0.3.0' },
      catalogs: {
        test: { '@vitest/coverage-v8': '4.1.11' },
        ignored: false,
      },
    })}\n`;

    const plan = planSyncVersions(packageJsonRequest(before), toolchain);

    expect(JSON.parse(plan.replacements[0].after)).toEqual({
      dependencies: { unrelated: false },
      optionalDependencies: { vitest: '5.0.0' },
      catalog: { 'vite-plus': '0.4.0' },
      catalogs: {
        test: { '@vitest/coverage-v8': '5.0.0' },
        ignored: false,
      },
    });
  });

  it('preserves npm references and non-semver shorthand specs', () => {
    const before = `${JSON.stringify({
      overrides: { vitest: '$vitest' },
      devDependencies: {
        '@vitest/coverage-v8': 'owner/provider#main',
        vitest: 'latest',
      },
    })}\n`;

    expect(planSyncVersions(packageJsonRequest(before), toolchain).replacements).toEqual([]);
  });

  it('is idempotent when planned replacements are used as the next input', () => {
    const before = '{\n  "devDependencies": {\n    "vite-plus": "0.3.0"\n  }\n}\n';
    const first = planSyncVersions(packageJsonRequest(before), toolchain);
    const second = planSyncVersions(packageJsonRequest(first.replacements[0].after), toolchain);

    expect(second.replacements).toEqual([]);
  });

  it.each(['{', 'null'])('rejects an invalid package.json manifest', (before) => {
    expect(() => planSyncVersions(packageJsonRequest(before), toolchain)).toThrow(
      'Invalid package.json manifest',
    );
  });
});

describe('parseSyncVersionsRequest', () => {
  it.each([
    {
      schemaVersion: 2,
      workspace: '.',
      manifests: [],
    },
    {
      schemaVersion: 1,
      workspace: '.',
      manifests: [{ path: '../package.json', kind: 'packageJson', contents: '{}' }],
    },
    {
      schemaVersion: 1,
      workspace: '.',
      manifests: [{ path: '/package.json', kind: 'packageJson', contents: '{}' }],
    },
    {
      schemaVersion: 1,
      workspace: '.',
      manifests: [{ path: 'dir\\package.json', kind: 'packageJson', contents: '{}' }],
    },
    {
      schemaVersion: 1,
      workspace: '.',
      manifests: [{ path: 'dir//package.json', kind: 'packageJson', contents: '{}' }],
    },
    {
      schemaVersion: 1,
      workspace: '.',
      manifests: [
        { path: 'package.json', kind: 'packageJson', contents: '{}' },
        { path: 'package.json', kind: 'packageJson', contents: '{}' },
      ],
    },
    {
      schemaVersion: 1,
      workspace: '.',
      manifests: [{ path: 'pnpm-workspace.yaml', kind: 'packageJson', contents: '{}' }],
    },
    {
      schemaVersion: 1,
      workspace: '.',
      manifests: [{ path: 'yarnrc.yml', kind: 'yarnRc', contents: '{}' }],
    },
  ])('rejects an invalid or ambiguous request', (request) => {
    expect(() => parseSyncVersionsRequest(request)).toThrow();
  });

  it('enforces manifest count and size limits', () => {
    expect(() =>
      parseSyncVersionsRequest({
        schemaVersion: 1,
        workspace: '.',
        manifests: Array.from({ length: 257 }, (_, index) => ({
          path: `${index}/package.json`,
          kind: 'packageJson',
          contents: '{}',
        })),
      }),
    ).toThrow();
    expect(() =>
      parseSyncVersionsRequest({
        schemaVersion: 1,
        workspace: '.',
        manifests: [
          {
            path: 'package.json',
            kind: 'packageJson',
            contents: 'a'.repeat(1024 * 1024 + 1),
          },
        ],
      }),
    ).toThrow();
  });
});
