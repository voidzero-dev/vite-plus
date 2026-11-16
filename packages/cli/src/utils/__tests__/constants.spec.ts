import { afterEach, describe, expect, it, vi } from 'vitest';

import corePkg from '../../../../core/package.json' with { type: 'json' };
import cliPkg from '../../../package.json' with { type: 'json' };

describe('Vite+ dependency versions', () => {
  afterEach(() => {
    vi.unstubAllEnvs();
    vi.resetModules();
  });

  it('uses the concrete CLI and core versions by default', async () => {
    vi.stubEnv('VP_VERSION', '');
    vi.stubEnv('VP_OVERRIDE_PACKAGES', '');
    vi.resetModules();

    const { VITE_PLUS_CORE_VERSION, VITE_PLUS_OVERRIDE_PACKAGES, VITE_PLUS_VERSION } =
      await import('../constants.js');

    expect(VITE_PLUS_VERSION).toBe(cliPkg.version);
    expect(VITE_PLUS_CORE_VERSION).toBe(corePkg.version);
    expect(VITE_PLUS_OVERRIDE_PACKAGES.vite).toBe(
      `npm:@voidzero-dev/vite-plus-core@${corePkg.version}`,
    );
  });

  it.each(['1.0.1', '0.0.0', '0.0.0-commit.0c515e3'])(
    'keeps the core manifest version for VP_VERSION %s',
    async (version) => {
      vi.stubEnv('VP_VERSION', version);
      vi.stubEnv('VP_OVERRIDE_PACKAGES', '');
      vi.resetModules();

      const { VITE_PLUS_CORE_VERSION } = await import('../constants.js');

      expect(VITE_PLUS_CORE_VERSION).toBe(corePkg.version);
    },
  );

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
