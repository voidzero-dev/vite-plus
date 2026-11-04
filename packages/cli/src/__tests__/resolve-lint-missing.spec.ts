import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Mock } from 'vitest';

vi.mock('../utils/constants.ts', async (importOriginal) => {
  const mod = await importOriginal<typeof import('../utils/constants.ts')>();
  return {
    ...mod,
    resolve: vi.fn(),
  };
});

vi.mock('../utils/tsgolint-path.ts', () => ({
  resolveTsgolintExecutable: vi.fn(),
  resolveWindowsTsgolintExecutable: vi.fn(),
}));

import { lint } from '../resolve-lint.ts';
import { resolve } from '../utils/constants.ts';
import { resolveTsgolintExecutable } from '../utils/tsgolint-path.ts';

describe('lint resolver when oxlint-tsgolint is absent', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    delete process.env.OXLINT_TSGOLINT_PATH;
  });

  afterEach(() => {
    delete process.env.OXLINT_TSGOLINT_PATH;
  });

  it('passes through an already-set OXLINT_TSGOLINT_PATH and skips bundled resolution', async () => {
    process.env.OXLINT_TSGOLINT_PATH = '/custom/tsgolint';
    (resolve as Mock).mockImplementation((request: string) => {
      if (request === 'oxlint') {
        return '/project/node_modules/oxlint/dist/index.js';
      }
      throw new Error(`unexpected resolve request: ${request}`);
    });

    const result = await lint();

    expect(result.binPath).toBe('/project/node_modules/oxlint/bin/oxlint');
    expect(result.envs.OXLINT_TSGOLINT_PATH).toBe('/custom/tsgolint');
    expect(result.envs.VP_RESOLVING_CONFIG_METADATA).toBe('1');
    expect(resolveTsgolintExecutable).not.toHaveBeenCalled();
  });

  it('resolves the bundled oxlint-tsgolint binary when no env var is set', async () => {
    (resolve as Mock).mockImplementation((request: string) => {
      if (request === 'oxlint') {
        return '/project/node_modules/oxlint/dist/index.js';
      }
      if (request === 'oxlint-tsgolint/bin/tsgolint') {
        return '/project/node_modules/oxlint-tsgolint/bin/tsgolint';
      }
      throw new Error(`unexpected resolve request: ${request}`);
    });
    (resolveTsgolintExecutable as Mock).mockImplementation((path: string) => path);

    const result = await lint();

    expect(result.envs.OXLINT_TSGOLINT_PATH).toBe(
      '/project/node_modules/oxlint-tsgolint/bin/tsgolint',
    );
  });

  it('omits OXLINT_TSGOLINT_PATH when the bundled package is not available', async () => {
    (resolve as Mock).mockImplementation((request: string) => {
      if (request === 'oxlint') {
        return '/project/node_modules/oxlint/dist/index.js';
      }
      throw new Error("Cannot find module 'oxlint-tsgolint/bin/tsgolint'");
    });
    (resolveTsgolintExecutable as Mock).mockImplementation(() => {
      throw new Error('Unable to resolve oxlint-tsgolint executable');
    });

    const result = await lint();

    expect(result.binPath).toBe('/project/node_modules/oxlint/bin/oxlint');
    expect(result.envs).not.toHaveProperty('OXLINT_TSGOLINT_PATH');
    expect(result.envs.VP_RESOLVING_CONFIG_METADATA).toBe('1');
  });
});
