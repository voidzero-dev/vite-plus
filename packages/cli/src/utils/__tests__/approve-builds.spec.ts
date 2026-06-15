import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { PackageManager } from '../../types/index.ts';
import {
  addYarnBuiltDependenciesMeta,
  collectDirectDependencyNames,
  filterToDirectDependencies,
  isPnpmIgnoredBuildsError,
  parseBunUntrusted,
  parseIgnoredBuilds,
  parseInstallGatedBuilds,
  parseYarnDisabledBuilds,
  pnpmSupportsApproveBuildsAll,
  pnpmSupportsPositionalApprove,
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

  it('keeps packages on box continuation lines when pnpm 10 wraps a long list', () => {
    const output = [
      '╭ Warning ─────────────────────────────────────────────────────────────────────╮',
      '│   Ignored build scripts: esbuild, better-sqlite3,                            │',
      '│   @scope/native.                                                             │',
      '│   Run "pnpm approve-builds" to pick which dependencies should run scripts.   │',
      '╰──────────────────────────────────────────────────────────────────────────────╯',
    ].join('\n');
    expect(parseIgnoredBuilds(output)).toEqual(['esbuild', 'better-sqlite3', '@scope/native']);
  });

  it('strips ANSI/VT control codes around colorized output', () => {
    // pnpm 11 single-line error wrapped in color codes (e.g. FORCE_COLOR in CI).
    const output =
      '[31m[ERR_PNPM_IGNORED_BUILDS][39m Ignored build scripts: [1mcore-js@3.39.0[22m.';
    expect(parseIgnoredBuilds(output)).toEqual(['core-js']);
  });

  it('returns [] when there is no ignored-builds marker', () => {
    expect(parseIgnoredBuilds('Done in 399ms using pnpm v11.6.0')).toEqual([]);
    expect(parseIgnoredBuilds('')).toEqual([]);
  });
});

describe('parseBunUntrusted', () => {
  it('parses the package names from real `bun pm untrusted` output', () => {
    const output = [
      'bun pm untrusted v1.3.11 (af24e281)',
      '',
      './node_modules/core-js @3.39.0',
      ' » [postinstall]: node -e "try{require(\'./postinstall\')}catch(e){}"',
      '',
      './node_modules/@scope/native @1.0.0',
      ' » [install]: node-gyp rebuild',
      '',
      'These dependencies had their lifecycle scripts blocked during install.',
      '',
      'If you trust them and wish to run their scripts, use `bun pm trust`.',
    ].join('\n');
    expect(parseBunUntrusted(output)).toEqual(['core-js', '@scope/native']);
  });

  it('takes the name after the last node_modules/ for nested packages', () => {
    expect(parseBunUntrusted('./node_modules/a/node_modules/b @1.0.0')).toEqual(['b']);
  });

  it('skips lines that mention node_modules but are not package entries', () => {
    // A lifecycle-script command can reference a node_modules path; it must not
    // be parsed as a package (no trailing ` @<version>`).
    expect(parseBunUntrusted(' » [postinstall]: node ./node_modules/.bin/tool')).toEqual([]);
  });

  it('dedupes and returns [] when nothing is blocked', () => {
    expect(parseBunUntrusted('./node_modules/x @1.0.0\n./node_modules/x @1.0.0')).toEqual(['x']);
    expect(parseBunUntrusted('No untrusted dependencies.')).toEqual([]);
    expect(parseBunUntrusted('')).toEqual([]);
  });
});

describe('parseYarnDisabledBuilds', () => {
  it('parses package names from yarn YN0004 "build scripts disabled" lines', () => {
    const output = [
      '➤ YN0000: ┌ Link step',
      '➤ YN0004: │ core-js@npm:3.39.0 lists build scripts, but all build scripts have been disabled.',
      '➤ YN0004: │ @scope/native@npm:1.0.0 lists build scripts, but all build scripts have been disabled.',
      '➤ YN0000: └ Completed',
    ].join('\n');
    expect(parseYarnDisabledBuilds(output)).toEqual(['core-js', '@scope/native']);
  });

  it('strips ANSI color codes and dedupes', () => {
    const output =
      '[33mcore-js@npm:3.39.0[39m lists build scripts, but all build scripts have been disabled.\n' +
      'core-js@npm:3.39.0 lists build scripts, but all build scripts have been disabled.';
    expect(parseYarnDisabledBuilds(output)).toEqual(['core-js']);
  });

  it('ignores yarn virtual-peer hashes trailing the descriptor', () => {
    const output =
      'svelte-preprocess@npm:6.0.3 [f4825] lists build scripts, but all build scripts have been disabled.';
    expect(parseYarnDisabledBuilds(output)).toEqual(['svelte-preprocess']);
  });

  it('returns [] when nothing is disabled', () => {
    expect(parseYarnDisabledBuilds('➤ YN0007: │ core-js@npm:3.39.0 must be built')).toEqual([]);
    expect(parseYarnDisabledBuilds('')).toEqual([]);
  });
});

describe('parseInstallGatedBuilds', () => {
  it('dispatches to the pnpm parser for pnpm', () => {
    expect(
      parseInstallGatedBuilds(
        '[ERR_PNPM_IGNORED_BUILDS] Ignored build scripts: better-sqlite3@11.0.0',
        PackageManager.pnpm,
      ),
    ).toEqual(['better-sqlite3']);
  });

  it('dispatches to the yarn parser for yarn', () => {
    expect(
      parseInstallGatedBuilds(
        'core-js@npm:3.39.0 lists build scripts, but all build scripts have been disabled.',
        PackageManager.yarn,
      ),
    ).toEqual(['core-js']);
  });

  it('returns [] for bun/npm (not parsed from install output)', () => {
    expect(parseInstallGatedBuilds('whatever', PackageManager.bun)).toEqual([]);
    expect(parseInstallGatedBuilds('whatever', PackageManager.npm)).toEqual([]);
    expect(parseInstallGatedBuilds('whatever', undefined)).toEqual([]);
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

  it('includes the real package name behind an npm: alias', () => {
    const names = collectDirectDependencyNames({
      dependencies: { sqlite: 'npm:better-sqlite3@^11.0.0', scoped: 'npm:@scope/native@1.0.0' },
    });
    // Both the alias key and the aliased real name (what the PM reports gated).
    expect(names.has('sqlite')).toBe(true);
    expect(names.has('better-sqlite3')).toBe(true);
    expect(names.has('@scope/native')).toBe(true);
  });

  it('is empty for a package.json without dependency fields', () => {
    expect(collectDirectDependencyNames({ name: 'x', version: '1.0.0' }).size).toBe(0);
    expect(collectDirectDependencyNames(undefined).size).toBe(0);
  });
});

describe('pnpmSupportsPositionalApprove', () => {
  it('is true for pnpm 11+ and unknown versions', () => {
    expect(pnpmSupportsPositionalApprove('11.0.0')).toBe(true);
    expect(pnpmSupportsPositionalApprove('11.6.0')).toBe(true);
    expect(pnpmSupportsPositionalApprove('12.1.0')).toBe(true);
    expect(pnpmSupportsPositionalApprove(undefined)).toBe(true);
  });

  it('is false for pnpm 10 (only `--all`, no positional approve)', () => {
    expect(pnpmSupportsPositionalApprove('10.33.2')).toBe(false);
    expect(pnpmSupportsPositionalApprove('10.0.0')).toBe(false);
  });
});

describe('pnpmSupportsApproveBuildsAll', () => {
  it('is true for pnpm 11+, pnpm 10.32+, and unknown versions', () => {
    expect(pnpmSupportsApproveBuildsAll('11.6.0')).toBe(true);
    expect(pnpmSupportsApproveBuildsAll('10.32.0')).toBe(true);
    expect(pnpmSupportsApproveBuildsAll('10.33.2')).toBe(true);
    expect(pnpmSupportsApproveBuildsAll(undefined)).toBe(true);
  });

  it('is false for pnpm < 10.32 (no `--all` support)', () => {
    expect(pnpmSupportsApproveBuildsAll('10.31.0')).toBe(false);
    expect(pnpmSupportsApproveBuildsAll('10.0.0')).toBe(false);
    expect(pnpmSupportsApproveBuildsAll('9.15.0')).toBe(false);
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

  it('returns direct-dep build targets for bun', () => {
    writePkg({ dependencies: { 'core-js': '3.39.0' } });
    expect(resolveApproveBuildTargets(dir, ['core-js', 'esbuild'], PackageManager.bun)).toEqual([
      'core-js',
    ]);
  });

  it('returns direct-dep build targets for yarn', () => {
    writePkg({ dependencies: { 'core-js': '3.39.0' } });
    expect(resolveApproveBuildTargets(dir, ['core-js', 'esbuild'], PackageManager.yarn)).toEqual([
      'core-js',
    ]);
  });

  it('returns [] for package managers that do not gate builds', () => {
    writePkg({ dependencies: { 'better-sqlite3': '^11.0.0' } });
    expect(resolveApproveBuildTargets(dir, ['better-sqlite3'], PackageManager.npm)).toEqual([]);
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

describe('addYarnBuiltDependenciesMeta', () => {
  it('adds dependenciesMeta[pkg].built=true, creating the container', () => {
    const pkg: Record<string, unknown> = { name: 'app' };
    addYarnBuiltDependenciesMeta(pkg, ['core-js', '@scope/native']);
    expect(pkg.dependenciesMeta).toEqual({
      'core-js': { built: true },
      '@scope/native': { built: true },
    });
  });

  it('preserves existing per-package metadata', () => {
    const pkg: Record<string, unknown> = {
      dependenciesMeta: { 'core-js': { optional: true }, other: { built: false } },
    };
    addYarnBuiltDependenciesMeta(pkg, ['core-js']);
    expect(pkg.dependenciesMeta).toEqual({
      'core-js': { optional: true, built: true },
      other: { built: false },
    });
  });

  it('does not corrupt a non-object existing entry (replaces it cleanly)', () => {
    // A hand-authored scalar value must not be spread into indexed-char keys.
    const pkg: Record<string, unknown> = { dependenciesMeta: { 'core-js': 'oops' } };
    addYarnBuiltDependenciesMeta(pkg, ['core-js']);
    expect(pkg.dependenciesMeta).toEqual({ 'core-js': { built: true } });
  });

  it('ignores a non-object dependenciesMeta container', () => {
    const pkg: Record<string, unknown> = { dependenciesMeta: 'nope' };
    addYarnBuiltDependenciesMeta(pkg, ['core-js']);
    expect(pkg.dependenciesMeta).toEqual({ 'core-js': { built: true } });
  });
});
