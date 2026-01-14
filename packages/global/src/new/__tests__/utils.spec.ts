import { describe, expect, it } from 'vitest';

import { formatTargetDir, getProjectDirFromPackageName } from '../utils.js';

describe('getProjectDirFromPackageName', () => {
  it('should get project dir from package name', () => {
    expect(getProjectDirFromPackageName('@my/package')).toBe('package');
    expect(getProjectDirFromPackageName('my-package')).toBe('my-package');
  });
});

describe('formatTargetDir', () => {
  it('should format target dir with invalid input', () => {
    expect(formatTargetDir('.')).matchSnapshot();
    expect(formatTargetDir('/foo/bar')).matchSnapshot();
    expect(formatTargetDir('@scope/')).matchSnapshot();
    expect(formatTargetDir('../../foo/bar')).matchSnapshot();
  });

  it.skipIf(process.platform === 'win32')('should format target dir with valid input', () => {
    expect(formatTargetDir('./my-package')).matchSnapshot();
    expect(formatTargetDir('my-package')).matchSnapshot();
    expect(formatTargetDir('@my-scope/my-package')).matchSnapshot();
    expect(formatTargetDir('foo/@my-scope/my-package')).matchSnapshot();
    expect(formatTargetDir('./foo/@my-scope/my-package')).matchSnapshot();
    expect(formatTargetDir('./foo/bar/@scope/my-package')).matchSnapshot();
    expect(formatTargetDir('./foo/bar/@scope/my-package/')).matchSnapshot();
    expect(formatTargetDir('./foo/bar/@scope/my-package/sub-package')).matchSnapshot();
  });

  it('should format target dir with invalid package name', () => {
    expect(formatTargetDir('my-package@').error).matchSnapshot();
    expect(formatTargetDir('my-package@1.0.0').error).matchSnapshot();
  });
});
