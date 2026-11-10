import * as verkit from 'verkit';
import { describe, expect, test } from 'vitest';
import * as yaml from 'yaml';

import { mergePnpmWorkspaces, mergeWorkspaceYaml } from '../sync-remote-deps.ts';

// `pnpm tool sync-remote` rewrites the whole pnpm-workspace.yaml. It must merge
// the upstream rolldown/vite workspaces into the main one WITHOUT dropping the
// comments authors put in the main file (e.g. the zod-v3 pin rationale added in
// PR #1777). `mergeWorkspaceYaml` is the comment-preserving parse+merge+serialize
// seam that syncRemote uses.
const VITE_PACKAGE = { devDependencies: { typescript: '~7.0.2' } };

const MAIN_SRC = `packages:
  - packages/*
catalog:
  acorn: ^8.12.1
  # bingo introspects template option schemas via zod 3 internals;
  # keep zod on v3 until bingo supports zod 4
  zod: ^3.25.76
catalogMode: prefer
ignoreScripts: true
minimumReleaseAge: 1440
minimumReleaseAgeExclude:
  - '@oxc-parser/*'
  - oxc-parser
  - lodash-es@4.18.1
overrides:
  rolldown: workspace:rolldown@*
`;

const ROLLDOWN_SRC = `catalog:
  some-rolldown-dep: ^1.0.0
minimumReleaseAgeExclude:
  - oxc-parser@0.134.0
`;

const VITE_SRC = `catalog:
  another-vite-dep: ^2.0.0
`;

describe('mergeWorkspaceYaml()', () => {
  test.each(['~7.0.2', '~7.1.0', '~6.0.3'])(
    "follows Vite's TypeScript declaration %s instead of merging catalog ranges",
    (typescript) => {
      const main = `catalog:
  # Follow the compiler declared by Vite.
  typescript: ^6.0.0
overrides:
  vite: workspace:@voidzero-dev/vite-plus-core@*
`;
      const rolldown = 'catalog:\n  typescript: =6.0.3\n';
      const vite = 'catalog:\n  typescript: ^7.2.0\n';
      const vitePackage = { devDependencies: { typescript } };
      const output = mergeWorkspaceYaml(main, rolldown, vite, vitePackage, yaml, verkit);

      expect(yaml.parse(output).catalog.typescript).toBe(typescript);
      expect(yaml.parse(output).overrides).toEqual({
        vite: 'workspace:@voidzero-dev/vite-plus-core@*',
        'rolldown>typescript': '=6.0.3',
      });
      expect(output).toContain('# Follow the compiler declared by Vite.');
      expect(mergeWorkspaceYaml(output, rolldown, vite, vitePackage, yaml, verkit)).toBe(output);
    },
  );

  test('resolves a TypeScript catalog reference from Vite', () => {
    const output = mergeWorkspaceYaml(
      'catalog:\n  typescript: ^6.0.0\n',
      'catalog:\n  typescript: ^6.0.0\n',
      'catalog:\n  typescript: ~7.0.2\n',
      { devDependencies: { typescript: 'catalog:' } },
      yaml,
      verkit,
    );
    expect(yaml.parse(output).catalog.typescript).toBe('~7.0.2');
  });

  test('updates the Rolldown compiler override when its catalog changes', () => {
    const output = mergeWorkspaceYaml(
      'overrides:\n  rolldown>typescript: ^6.0.0\n',
      'catalog:\n  typescript: ~7.0.2\n',
      VITE_SRC,
      VITE_PACKAGE,
      yaml,
      verkit,
    );
    expect(yaml.parse(output).overrides['rolldown>typescript']).toBe('~7.0.2');
  });

  test("preserves Vite's TypeScript compatibility overrides without replacing workspace aliases", () => {
    const output = mergeWorkspaceYaml(
      MAIN_SRC,
      'catalog:\n  typescript: ^6.0.0\n',
      `overrides:
  vite: workspace:*
  '@volar/typescript>typescript@>=7': '-'
  '@volar/language-service>typescript@>=7': '-'
`,
      VITE_PACKAGE,
      yaml,
      verkit,
    );
    expect(yaml.parse(output).overrides).toEqual({
      rolldown: 'workspace:rolldown@*',
      'rolldown>typescript': '^6.0.0',
      '@volar/typescript>typescript@>=7': '-',
      '@volar/language-service>typescript@>=7': '-',
    });
  });

  test.each([undefined, 'invalid', 'catalog:'])(
    'rejects a missing or unresolved Vite TypeScript range: %s',
    (typescript) => {
      expect(() =>
        mergeWorkspaceYaml(
          'catalog:\n  typescript: ^6.0.0\n',
          ROLLDOWN_SRC,
          VITE_SRC,
          { devDependencies: typescript === undefined ? {} : { typescript } },
          yaml,
          verkit,
        ),
      ).toThrow('Vite must declare a valid TypeScript devDependency range');
    },
  );

  test('preserves the latest supported pin across repeated upstream syncs', () => {
    const main = `catalog:
  # Keep the runner and official packages on the selected release.
  vitest: 5.1.2
  '@vitest/browser': 5.1.2
`;
    const rolldown = `catalog:
  vitest: '=4.1.11'
  '@vitest/utils': '^4.1.11'
`;
    const vite = `catalog:
  vitest: '^5.2.0'
  '@vitest/browser': '^5.2.0'
`;

    const output = mergeWorkspaceYaml(main, rolldown, vite, VITE_PACKAGE, yaml, verkit);

    expect(yaml.parse(output).catalog).toEqual({
      typescript: '~7.0.2',
      vitest: '5.1.2',
      '@vitest/browser': '5.1.2',
      '@vitest/utils': '5.1.2',
    });
    expect(output).toContain('# Keep the runner and official packages on the selected release.');
    expect(mergeWorkspaceYaml(output, rolldown, vite, VITE_PACKAGE, yaml, verkit)).toBe(output);
  });

  test('preserves comments from the main workspace', () => {
    const output = mergeWorkspaceYaml(MAIN_SRC, ROLLDOWN_SRC, VITE_SRC, VITE_PACKAGE, yaml, verkit);

    expect(output).toContain('# keep zod on v3 until bingo supports zod 4');
    expect(output).toContain('# bingo introspects template option schemas via zod 3 internals;');
  });

  test('produces the same merged data as mergePnpmWorkspaces (no semantic drift)', () => {
    const expected = mergePnpmWorkspaces(
      yaml.parse(MAIN_SRC),
      yaml.parse(ROLLDOWN_SRC),
      yaml.parse(VITE_SRC),
      VITE_PACKAGE,
      verkit,
    );

    const output = mergeWorkspaceYaml(MAIN_SRC, ROLLDOWN_SRC, VITE_SRC, VITE_PACKAGE, yaml, verkit);

    expect(yaml.parse(output)).toEqual(expected);
  });

  test('merges upstream catalog entries and dedupes redundant exclude entries', () => {
    const output = mergeWorkspaceYaml(MAIN_SRC, ROLLDOWN_SRC, VITE_SRC, VITE_PACKAGE, yaml, verkit);
    const parsed = yaml.parse(output);

    // Upstream catalog entries merged in, original pin kept.
    expect(parsed.catalog['some-rolldown-dep']).toBe('^1.0.0');
    expect(parsed.catalog['another-vite-dep']).toBe('^2.0.0');
    expect(parsed.catalog.zod).toBe('^3.25.76');
    // The versioned exclude entry is redundant (covered by `oxc-parser`) and dropped.
    expect(parsed.minimumReleaseAgeExclude).not.toContain('oxc-parser@0.134.0');
    expect(parsed.minimumReleaseAgeExclude).toContain('oxc-parser');
  });
});
