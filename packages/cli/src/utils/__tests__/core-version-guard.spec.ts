import { mkdirSync, mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { VITE_PLUS_CORE_PACKAGE_NAME as CORE } from '../constants.ts';
import {
  assertCoreVersionMatch,
  checkCoreVersionMatch,
  SKIP_CORE_VERSION_CHECK_ENV,
} from '../core-version-guard.ts';

describe('assertCoreVersionMatch', () => {
  it('does not throw when the aliased core matches the expected version', () => {
    expect(() => assertCoreVersionMatch('1.2.3', '1.2.3')).not.toThrow();
  });

  it('throws with both versions, the fix spec, and the escape hatch on a skew', () => {
    expect(() => assertCoreVersionMatch('1.2.0', '1.2.3')).toThrow(
      new RegExp(`${CORE}@1\\.2\\.0.*npm:${CORE}@1\\.2\\.3.*${SKIP_CORE_VERSION_CHECK_ENV}`, 's'),
    );
  });

  it('does not throw when no aliased core is installed', () => {
    expect(() => assertCoreVersionMatch(null, '1.2.3')).not.toThrow();
    expect(() => assertCoreVersionMatch(undefined, '1.2.3')).not.toThrow();
  });
});

describe('checkCoreVersionMatch', () => {
  let projectDir: string;

  // What `vite` resolves to in the project, shaped like a real install.
  function writeVitePackage(pkg: { name: string; version: string }) {
    const dir = join(projectDir, 'node_modules', 'vite');
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, 'package.json'), JSON.stringify(pkg));
  }

  beforeEach(() => {
    // realpath so resolution output matches (macOS tmpdir is /var -> /private/var).
    projectDir = realpathSync(mkdtempSync(join(tmpdir(), 'vp-core-guard-')));
  });

  afterEach(() => {
    vi.unstubAllEnvs();
    rmSync(projectDir, { recursive: true, force: true });
  });

  it('throws when the installed aliased core skews from the expected version', () => {
    writeVitePackage({ name: CORE, version: '1.2.0' });
    expect(() => checkCoreVersionMatch(projectDir, '1.2.3')).toThrow(`${CORE}@1.2.0`);
  });

  it('does not throw when the installed aliased core matches', () => {
    writeVitePackage({ name: CORE, version: '1.2.3' });
    expect(() => checkCoreVersionMatch(projectDir, '1.2.3')).not.toThrow();
  });

  it(`skips the check when ${SKIP_CORE_VERSION_CHECK_ENV} is set`, () => {
    vi.stubEnv(SKIP_CORE_VERSION_CHECK_ENV, '1');
    writeVitePackage({ name: CORE, version: '1.2.0' });
    expect(() => checkCoreVersionMatch(projectDir, '1.2.3')).not.toThrow();
  });

  it('does not throw for a project on real Vite', () => {
    writeVitePackage({ name: 'vite', version: '99.0.0' });
    expect(() => checkCoreVersionMatch(projectDir, '1.2.3')).not.toThrow();
  });

  it('does not throw when vite is not installed', () => {
    expect(() => checkCoreVersionMatch(projectDir, '1.2.3')).not.toThrow();
  });
});
