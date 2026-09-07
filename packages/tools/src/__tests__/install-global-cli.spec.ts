import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import { afterEach, beforeEach, describe, expect, it } from 'vitest';

import { createCiInstallPackage } from '../install-global-cli.ts';

describe('createCiInstallPackage', () => {
  let directory: string;

  beforeEach(() => {
    directory = mkdtempSync(join(tmpdir(), 'vp-ci-deps-'));
  });

  afterEach(() => rmSync(directory, { recursive: true, force: true }));

  it.each(['0.3.0', '0.0.0-commit.1234567'])('installs the packed core alias for %s', (version) => {
    const core = join(directory, `voidzero-dev-vite-plus-core-${version}.tgz`);
    const cli = join(directory, `vite-plus-${version}.tgz`);
    writeFileSync(core, '');
    const result = createCiInstallPackage(
      { vite: `npm:@voidzero-dev/vite-plus-core@${version}`, vitest: '4.1.11' },
      cli,
    );
    expect(result.dependencies).toEqual({ 'vite-plus': `file:${cli}`, vite: `file:${core}` });
    expect(result.overrides).toEqual({ vite: '$vite' });
  });

  it('keeps canonical dependencies compatible with older packed CLIs', () => {
    const core = join(directory, 'voidzero-dev-vite-plus-core-0.3.0.tgz');
    writeFileSync(core, '');
    const result = createCiInstallPackage(
      { '@voidzero-dev/vite-plus-core': '0.3.0' },
      join(directory, 'vite-plus-0.3.0.tgz'),
    );
    expect(result.dependencies['@voidzero-dev/vite-plus-core']).toBe(`file:${core}`);
    expect(result.overrides).toEqual({
      '@voidzero-dev/vite-plus-core': '$@voidzero-dev/vite-plus-core',
    });
  });
});
