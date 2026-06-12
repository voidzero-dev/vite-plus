import { afterEach, describe, expect, it, vi } from 'vitest';

// Pin the registry so URL assertions don't depend on the developer's npm
// config (a configured mirror registry would change the requested address).
vi.mock('../npm-config.js', async (importOriginal) => {
  const mod = await importOriginal<typeof import('../npm-config.js')>();
  return { ...mod, getNpmRegistry: () => 'https://registry.npmjs.org' };
});

const { checkNpmPackageExists } = await import('../package.js');

// Pin the registry: getNpmRegistry reads the developer's real `.npmrc`, so
// the URL assertions below would fail for anyone using a mirror registry.
vi.mock('../npm-config.js', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../npm-config.js')>();
  return {
    ...actual,
    getNpmRegistry: () => 'https://registry.npmjs.org',
  };
});

describe('checkNpmPackageExists', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('returns true when package exists (200)', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue({ status: 200, ok: true } as Response);
    expect(await checkNpmPackageExists('create-vite')).toBe(true);
  });

  it('returns false when package does not exist (404)', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue({ status: 404, ok: false } as Response);
    expect(await checkNpmPackageExists('create-vite-plus-app')).toBe(false);
  });

  it('returns true on network error', async () => {
    vi.spyOn(globalThis, 'fetch').mockRejectedValue(new TypeError('fetch failed'));
    expect(await checkNpmPackageExists('create-vite')).toBe(true);
  });

  it('strips version from unscoped package name', async () => {
    const mockFetch = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValue({ status: 200, ok: true } as Response);
    await checkNpmPackageExists('create-vite@latest');
    expect(mockFetch).toHaveBeenCalledWith(
      'https://registry.npmjs.org/create-vite',
      expect.objectContaining({ method: 'HEAD' }),
    );
  });

  it('strips version from scoped package name', async () => {
    const mockFetch = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValue({ status: 200, ok: true } as Response);
    await checkNpmPackageExists('@tanstack/create-start@latest');
    expect(mockFetch).toHaveBeenCalledWith(
      'https://registry.npmjs.org/@tanstack/create-start',
      expect.objectContaining({ method: 'HEAD' }),
    );
  });

  it('does not strip scope from scoped package without version', async () => {
    const mockFetch = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValue({ status: 200, ok: true } as Response);
    await checkNpmPackageExists('@tanstack/create-start');
    expect(mockFetch).toHaveBeenCalledWith(
      'https://registry.npmjs.org/@tanstack/create-start',
      expect.objectContaining({ method: 'HEAD' }),
    );
  });
});
