import { afterEach, describe, expect, it, vi } from 'vitest';

import { checkNpmPackageExists } from '../package.js';

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
