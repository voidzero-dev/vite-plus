import { describe, expect, it } from 'vitest';

import {
  deriveDefaultPackageName,
  formatTargetDir,
  getProjectDirFromPackageName,
} from '../utils.js';

describe('getProjectDirFromPackageName', () => {
  it('should get project dir from package name', () => {
    expect(getProjectDirFromPackageName('@my/package')).toBe('package');
    expect(getProjectDirFromPackageName('my-package')).toBe('my-package');
  });
});

describe('formatTargetDir', () => {
  it('should format "." as current directory with empty package name', () => {
    expect(formatTargetDir('.')).toEqual({
      directory: '.',
      packageName: '',
    });
  });

  it('should format "./" as current directory with empty package name', () => {
    expect(formatTargetDir('./')).toEqual({
      directory: '.',
      packageName: '',
    });
  });

  it('should format target dir with invalid input', () => {
    expect(formatTargetDir('/foo/bar')).matchSnapshot();
    expect(formatTargetDir('@scope/')).matchSnapshot();
    expect(formatTargetDir('../../foo/bar')).matchSnapshot();
  });

  // Should work on all platforms (including Windows) - directory must always use forward slashes
  it('should format target dir with valid input', () => {
    expect(formatTargetDir('./my-package')).matchSnapshot();
    expect(formatTargetDir('my-package')).matchSnapshot();
    expect(formatTargetDir('@my-scope/my-package')).matchSnapshot();
    expect(formatTargetDir('foo/@my-scope/my-package')).matchSnapshot();
    expect(formatTargetDir('./foo/@my-scope/my-package')).matchSnapshot();
    expect(formatTargetDir('./foo/bar/@scope/my-package')).matchSnapshot();
    expect(formatTargetDir('./foo/bar/@scope/my-package/')).matchSnapshot();
    expect(formatTargetDir('./foo/bar/@scope/my-package/sub-package')).matchSnapshot();
  });

  // Regression test for https://github.com/voidzero-dev/vite-plus/issues/938
  // On Windows, path.join/normalize produce backslashes which break when passed as CLI args.
  // Nested paths are the critical cases since they involve path separators.
  it('should always use forward slashes in directory (issue #938)', () => {
    expect(formatTargetDir('foo/@my-scope/my-package').directory).toBe('foo/my-package');
    expect(formatTargetDir('./foo/bar/@scope/my-package').directory).toBe('foo/bar/my-package');
    expect(formatTargetDir('./foo/bar/@scope/my-package/sub-package').directory).toBe(
      'foo/bar/@scope/my-package/sub-package',
    );
  });

  it('should format target dir with invalid package name', () => {
    expect(formatTargetDir('my-package@').error).matchSnapshot();
    expect(formatTargetDir('my-package@1.0.0').error).matchSnapshot();
  });
});

describe('deriveDefaultPackageName', () => {
  it('should derive package name from directory basename', () => {
    expect(deriveDefaultPackageName('/home/user/my-app', undefined, 'fallback')).toBe('my-app');
  });

  it('should derive scoped package name when scope is provided', () => {
    expect(deriveDefaultPackageName('/home/user/my-app', '@my-scope', 'fallback')).toBe(
      '@my-scope/my-app',
    );
  });

  it('should fallback to random name when directory name is invalid', () => {
    const result = deriveDefaultPackageName('/home/user/.hidden', undefined, 'vite-plus-app');
    // directory name starts with '.', so a random name is generated instead
    expect(result).not.toBe('.hidden');
    expect(result.length).toBeGreaterThan(0);
  });

  it('should fallback when directory is filesystem root', () => {
    const result = deriveDefaultPackageName('/', undefined, 'vite-plus-app');
    // basename of '/' is empty, so a random name is generated
    expect(result.length).toBeGreaterThan(0);
  });
});
