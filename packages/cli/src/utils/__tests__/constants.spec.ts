import { afterEach, describe, expect, it, vi } from 'vitest';

import cliPkg from '../../../package.json' with { type: 'json' };

describe('Vite+ dependency versions', () => {
  afterEach(() => {
    vi.unstubAllEnvs();
    vi.resetModules();
  });

  it('uses the concrete CLI version for vite-plus and vite-plus-core by default', async () => {
    vi.stubEnv('VP_VERSION', '');
    vi.stubEnv('VP_OVERRIDE_PACKAGES', '');
    vi.resetModules();

    const { VITE_PLUS_OVERRIDE_PACKAGES, VITE_PLUS_VERSION } = await import('../constants.js');

    expect(VITE_PLUS_VERSION).toBe(cliPkg.version);
    expect(VITE_PLUS_OVERRIDE_PACKAGES.vite).toBe(
      `npm:@voidzero-dev/vite-plus-core@${cliPkg.version}`,
    );
  });

  it('preserves explicit prerelease overrides', async () => {
    const vitePlusUrl = 'https://pkg.pr.new/voidzero-dev/vite-plus@1891';
    const viteCoreUrl =
      'https://pkg.pr.new/voidzero-dev/vite-plus/@voidzero-dev/vite-plus-core@1891';
    vi.stubEnv('VP_VERSION', vitePlusUrl);
    vi.stubEnv('VP_OVERRIDE_PACKAGES', JSON.stringify({ vite: viteCoreUrl, vitest: '4.1.9' }));
    vi.resetModules();

    const { VITE_PLUS_OVERRIDE_PACKAGES, VITE_PLUS_VERSION } = await import('../constants.js');

    expect(VITE_PLUS_VERSION).toBe(vitePlusUrl);
    expect(VITE_PLUS_OVERRIDE_PACKAGES.vite).toBe(viteCoreUrl);
  });
});
