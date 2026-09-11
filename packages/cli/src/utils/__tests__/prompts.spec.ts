import { describe, expect, it } from 'vitest';

import { PackageManager } from '../../types/index.ts';
import { resolveGitInit, shouldIgnoreScriptsForAutoInstall } from '../prompts.ts';

describe('resolveGitInit', () => {
  it('never initializes git when adding a package to an existing monorepo', async () => {
    // A sub-package shares the monorepo's repository, so git setup must be
    // skipped even if `--git` is forced — and no interactive prompt is shown.
    expect(await resolveGitInit({ git: true, interactive: true }, true)).toBe(false);
    expect(await resolveGitInit({ git: undefined, interactive: true }, true)).toBe(false);
    expect(await resolveGitInit({ git: undefined, interactive: false }, true)).toBe(false);
  });

  it('respects the git option for a new standalone project', async () => {
    expect(await resolveGitInit({ git: true, interactive: false }, false)).toBe(true);
    expect(await resolveGitInit({ git: false, interactive: false }, false)).toBe(false);
    // non-interactive default is no git
    expect(await resolveGitInit({ git: undefined, interactive: false }, false)).toBe(false);
  });
});

describe('shouldIgnoreScriptsForAutoInstall', () => {
  it('returns true for pnpm >= 11.0.0', () => {
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.pnpm, '11.0.0')).toBe(true);
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.pnpm, '11.0.8')).toBe(true);
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.pnpm, '12.0.0')).toBe(true);
  });

  it('returns false for pnpm < 11.0.0', () => {
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.pnpm, '10.33.2')).toBe(false);
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.pnpm, '9.5.0')).toBe(false);
  });

  it('returns false for non-pnpm package managers', () => {
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.npm, '11.0.0')).toBe(false);
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.yarn, '11.0.0')).toBe(false);
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.bun, '11.0.0')).toBe(false);
  });

  it('returns false when version is unknown or unparsable', () => {
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.pnpm, undefined)).toBe(false);
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.pnpm, 'latest')).toBe(false);
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.pnpm, '')).toBe(false);
  });

  it('returns false when packageManager is undefined', () => {
    expect(shouldIgnoreScriptsForAutoInstall(undefined, '11.0.0')).toBe(false);
  });

  it('coerces non-strict semver pnpm versions', () => {
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.pnpm, 'v11.0.0')).toBe(true);
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.pnpm, '11')).toBe(true);
    expect(shouldIgnoreScriptsForAutoInstall(PackageManager.pnpm, '10')).toBe(false);
  });
});
