import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { PackageManager } from '../../types/index.ts';
import {
  collectDirectDependencyNames,
  filterToDirectDependencies,
  isPnpmIgnoredBuildsError,
  parseIgnoredBuilds,
  resolveApproveBuildTargets,
  stripPackageVersion,
} from '../approve-builds.ts';

describe('isPnpmIgnoredBuildsError', () => {
  it('detects the pnpm >= 11 hard-error token', () => {
    expect(
      isPnpmIgnoredBuildsError(
        '[ERR_PNPM_IGNORED_BUILDS] Ignored build scripts: better-sqlite3@11.0.0',
      ),
    ).toBe(true);
  });

  it('is false for a clean install log', () => {
    expect(isPnpmIgnoredBuildsError('Done in 399ms using pnpm v11.6.0')).toBe(false);
  });

  it('is false for the pnpm 10 warning (no error token)', () => {
    expect(isPnpmIgnoredBuildsError('Ignored build scripts: esbuild.')).toBe(false);
  });
});

describe('stripPackageVersion', () => {
  it('strips a trailing @version from an unscoped package', () => {
    expect(stripPackageVersion('better-sqlite3@11.0.0')).toBe('better-sqlite3');
  });

  it('keeps the leading @ of a scoped package and strips only the version', () => {
    expect(stripPackageVersion('@scope/pkg@1.2.3')).toBe('@scope/pkg');
  });

  it('returns the name unchanged when there is no version', () => {
    expect(stripPackageVersion('esbuild')).toBe('esbuild');
    expect(stripPackageVersion('@scope/pkg')).toBe('@scope/pkg');
  });
});

describe('parseIgnoredBuilds', () => {
  it('parses the pnpm >= 11 ERR_PNPM_IGNORED_BUILDS line with versions', () => {
    const output = [
      '+ better-sqlite3 11.0.0 (12.10.0 is available)',
      '',
      '[ERR_PNPM_IGNORED_BUILDS] Ignored build scripts: better-sqlite3@11.0.0, esbuild@0.25.0',
      '',
      'Run "pnpm approve-builds" to pick which dependencies should be allowed to run scripts.',
    ].join('\n');
    expect(parseIgnoredBuilds(output)).toEqual(['better-sqlite3', 'esbuild']);
  });

  it('dedupes the same package listed under multiple versions', () => {
    const output =
      '[ERR_PNPM_IGNORED_BUILDS] Ignored build scripts: esbuild@0.25.0, esbuild@0.27.7, better-sqlite3@11.0.0';
    expect(parseIgnoredBuilds(output)).toEqual(['esbuild', 'better-sqlite3']);
  });

  it('parses the pnpm 10 warning box (names only, trailing period, box borders)', () => {
    const output = [
      '╭ Warning ─────────────────────────────────────────────────────────────────────╮',
      '│                                                                              │',
      '│   Ignored build scripts: esbuild.                                            │',
      '│   Run "pnpm approve-builds" to pick which dependencies should run scripts.   │',
      '│                                                                              │',
      '╰──────────────────────────────────────────────────────────────────────────────╯',
      '',
      'Done in 171ms using pnpm v10.16.1',
    ].join('\n');
    expect(parseIgnoredBuilds(output)).toEqual(['esbuild']);
  });

  it('parses scoped packages', () => {
    const output =
      '[ERR_PNPM_IGNORED_BUILDS] Ignored build scripts: @scope/native@1.0.0, better-sqlite3@11.0.0';
    expect(parseIgnoredBuilds(output)).toEqual(['@scope/native', 'better-sqlite3']);
  });

  it('returns [] when there is no ignored-builds marker', () => {
    expect(parseIgnoredBuilds('Done in 399ms using pnpm v11.6.0')).toEqual([]);
    expect(parseIgnoredBuilds('')).toEqual([]);
  });
});

describe('collectDirectDependencyNames', () => {
  it('collects dependencies, devDependencies and optionalDependencies', () => {
    const names = collectDirectDependencyNames({
      dependencies: { 'better-sqlite3': '^11.0.0' },
      devDependencies: { vite: '^7.0.0' },
      optionalDependencies: { fsevents: '^2.0.0' },
      peerDependencies: { react: '^19.0.0' },
    });
    expect(names.has('better-sqlite3')).toBe(true);
    expect(names.has('vite')).toBe(true);
    expect(names.has('fsevents')).toBe(true);
    // peerDependencies are not installed locally, so they are not "direct".
    expect(names.has('react')).toBe(false);
  });

  it('is empty for a package.json without dependency fields', () => {
    expect(collectDirectDependencyNames({ name: 'x', version: '1.0.0' }).size).toBe(0);
    expect(collectDirectDependencyNames(undefined).size).toBe(0);
  });
});

describe('filterToDirectDependencies', () => {
  it('keeps only ignored packages that are direct dependencies', () => {
    const direct = new Set(['better-sqlite3']);
    expect(filterToDirectDependencies(['better-sqlite3', 'esbuild'], direct)).toEqual([
      'better-sqlite3',
    ]);
  });

  it('returns [] when nothing matches (only transitive noise)', () => {
    expect(filterToDirectDependencies(['esbuild'], new Set(['better-sqlite3']))).toEqual([]);
  });
});

describe('resolveApproveBuildTargets', () => {
  let dir: string;

  beforeEach(() => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), 'vp-approve-builds-'));
  });

  afterEach(() => {
    fs.rmSync(dir, { recursive: true, force: true });
  });

  function writePkg(pkg: Record<string, unknown>) {
    fs.writeFileSync(path.join(dir, 'package.json'), JSON.stringify(pkg), 'utf-8');
  }

  it('returns direct-dep build targets for pnpm', () => {
    writePkg({ dependencies: { 'better-sqlite3': '^11.0.0' } });
    expect(
      resolveApproveBuildTargets(dir, ['better-sqlite3', 'esbuild'], PackageManager.pnpm),
    ).toEqual(['better-sqlite3']);
  });

  it('returns [] for non-pnpm package managers', () => {
    writePkg({ dependencies: { 'better-sqlite3': '^11.0.0' } });
    expect(resolveApproveBuildTargets(dir, ['better-sqlite3'], PackageManager.npm)).toEqual([]);
    expect(resolveApproveBuildTargets(dir, ['better-sqlite3'], PackageManager.yarn)).toEqual([]);
  });

  it('returns [] when there are no pending builds', () => {
    writePkg({ dependencies: { 'better-sqlite3': '^11.0.0' } });
    expect(resolveApproveBuildTargets(dir, undefined, PackageManager.pnpm)).toEqual([]);
    expect(resolveApproveBuildTargets(dir, [], PackageManager.pnpm)).toEqual([]);
  });

  it('returns [] when the project package.json is missing', () => {
    expect(resolveApproveBuildTargets(dir, ['better-sqlite3'], PackageManager.pnpm)).toEqual([]);
  });

  it('ignores transitive-only pending builds (e.g. esbuild from vite)', () => {
    writePkg({ devDependencies: { vite: '^7.0.0' } });
    expect(resolveApproveBuildTargets(dir, ['esbuild'], PackageManager.pnpm)).toEqual([]);
  });
});
