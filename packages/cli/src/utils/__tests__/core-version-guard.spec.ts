import { mkdirSync, mkdtempSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { VITE_PLUS_CORE_PACKAGE_NAME as CORE } from '../constants.ts';
import {
  assertCoreVersionMatch,
  checkCoreVersionMatchForResolver,
  checkCoreVersionMatch,
  CORE_VERSION_MISMATCH_ERROR_KIND,
  CoreVersionMismatchError,
  SKIP_CORE_VERSION_CHECK_ENV,
} from '../core-version-guard.ts';

describe('assertCoreVersionMatch', () => {
  it('does not throw when the aliased core matches the expected version', () => {
    expect(() => assertCoreVersionMatch('1.2.3', '1.2.3')).not.toThrow();
  });

  it('throws with a readable explanation and fixes on a skew', () => {
    const expectedMessage = [
      `Your \`vite\` alias uses ${CORE}@1.2.0.`,
      `This Vite+ CLI requires ${CORE}@1.2.3.`,
      '',
      'Choose a fix:',
      `- Update the \`vite\` alias to npm:${CORE}@1.2.3.`,
      '- Run `vp migrate`.',
      '',
      `To skip this check, set ${SKIP_CORE_VERSION_CHECK_ENV}=1.`,
    ].join('\n');

    let error: unknown;
    try {
      assertCoreVersionMatch('1.2.0', '1.2.3');
    } catch (caught) {
      error = caught;
    }

    expect(error).toBeInstanceOf(CoreVersionMismatchError);
    expect(error).toMatchObject({ message: expectedMessage });
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

  it('returns a tagged resolver error when the installed core version differs', () => {
    writeVitePackage({ name: CORE, version: '1.2.0' });

    expect(checkCoreVersionMatchForResolver(projectDir, '1.2.3')).toMatchObject({
      errorKind: CORE_VERSION_MISMATCH_ERROR_KIND,
      errorMessage: expect.stringContaining(`${CORE}@1.2.0`),
    });
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
