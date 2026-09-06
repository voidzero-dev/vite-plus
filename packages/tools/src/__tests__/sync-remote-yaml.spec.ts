import * as semver from 'semver';
import { describe, expect, test } from 'vitest';
import * as yaml from 'yaml';

import { findLatestStableVersionForMajor } from '../../../../.github/scripts/upgrade-deps-utils.ts';
import { mergePnpmWorkspaces, mergeWorkspaceYaml } from '../sync-remote-deps.ts';

// `pnpm tool sync-remote` rewrites the whole pnpm-workspace.yaml. It must merge
// the upstream rolldown/vite workspaces into the main one WITHOUT dropping the
// comments authors put in the main file (e.g. the zod-v3 pin rationale added in
// PR #1777). `mergeWorkspaceYaml` is the comment-preserving parse+merge+serialize
// seam that syncRemote uses.
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
  test('preserves the latest supported pin across repeated upstream syncs', () => {
    const version = findLatestStableVersionForMajor(['5.0.0', '5.1.2', '5.2.0-beta.1', '6.0.0'], 5);
    expect(version).toBe('5.1.2');
    const main = `catalog:
  # Keep the runner and official packages on the selected release.
  vitest: ${version}
  '@vitest/browser': ${version}
`;
    const rolldown = `catalog:
  vitest: '=4.1.11'
  '@vitest/utils': '^4.1.11'
`;
    const vite = `catalog:
  vitest: '^5.2.0'
  '@vitest/browser': '^5.2.0'
`;

    const output = mergeWorkspaceYaml(main, rolldown, vite, yaml, semver);

    expect(yaml.parse(output).catalog).toEqual({
      vitest: '5.1.2',
      '@vitest/browser': '5.1.2',
      '@vitest/utils': '5.1.2',
    });
    expect(output).toContain('# Keep the runner and official packages on the selected release.');
    expect(mergeWorkspaceYaml(output, rolldown, vite, yaml, semver)).toBe(output);
  });

  test('preserves comments from the main workspace', () => {
    const output = mergeWorkspaceYaml(MAIN_SRC, ROLLDOWN_SRC, VITE_SRC, yaml, semver);

    expect(output).toContain('# keep zod on v3 until bingo supports zod 4');
    expect(output).toContain('# bingo introspects template option schemas via zod 3 internals;');
  });

  test('produces the same merged data as mergePnpmWorkspaces (no semantic drift)', () => {
    const expected = mergePnpmWorkspaces(
      yaml.parse(MAIN_SRC),
      yaml.parse(ROLLDOWN_SRC),
      yaml.parse(VITE_SRC),
      semver,
    );

    const output = mergeWorkspaceYaml(MAIN_SRC, ROLLDOWN_SRC, VITE_SRC, yaml, semver);

    expect(yaml.parse(output)).toEqual(expected);
  });

  test('merges upstream catalog entries and dedupes redundant exclude entries', () => {
    const output = mergeWorkspaceYaml(MAIN_SRC, ROLLDOWN_SRC, VITE_SRC, yaml, semver);
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
