import { describe, expect, it } from 'vitest';

import { PackageManager } from '../../types/index.ts';
import { shouldIgnoreScriptsForAutoInstall } from '../prompts.ts';

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
