import { afterEach, describe, expect, it, vi } from 'vitest';

import { checkNpmPackageExists, hasPackageManagerDeclaration } from '../package.js';

// Pin the registry: getNpmRegistry reads the developer's real `.npmrc`, so
// the URL assertions below would fail for anyone using a mirror registry.
vi.mock(import('../npm-config.js'), async (importOriginal) => {
  const actual = await importOriginal();
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
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(null, { status: 200 }));
    expect(await checkNpmPackageExists('create-vite')).toBe(true);
  });

  it('returns false when package does not exist (404)', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(null, { status: 404 }));
    expect(await checkNpmPackageExists('create-vite-plus-app')).toBe(false);
  });

  it('returns true on network error', async () => {
    vi.spyOn(globalThis, 'fetch').mockRejectedValue(new TypeError('fetch failed'));
    expect(await checkNpmPackageExists('create-vite')).toBe(true);
  });

  it('strips version from unscoped package name', async () => {
    const mockFetch = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValue(new Response(null, { status: 200 }));
    await checkNpmPackageExists('create-vite@latest');
    expect(mockFetch).toHaveBeenCalledWith(
      'https://registry.npmjs.org/create-vite',
      expect.objectContaining({ method: 'HEAD' }),
    );
  });

  it('strips version from scoped package name', async () => {
    const mockFetch = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValue(new Response(null, { status: 200 }));
    await checkNpmPackageExists('@tanstack/create-start@latest');
    expect(mockFetch).toHaveBeenCalledWith(
      'https://registry.npmjs.org/@tanstack/create-start',
      expect.objectContaining({ method: 'HEAD' }),
    );
  });

  it('does not strip scope from scoped package without version', async () => {
    const mockFetch = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValue(new Response(null, { status: 200 }));
    await checkNpmPackageExists('@tanstack/create-start');
    expect(mockFetch).toHaveBeenCalledWith(
      'https://registry.npmjs.org/@tanstack/create-start',
      expect.objectContaining({ method: 'HEAD' }),
    );
  });
});

describe('hasPackageManagerDeclaration', () => {
  it('detects both supported package manager declarations', () => {
    expect(hasPackageManagerDeclaration({ packageManager: 'pnpm@10.17.1' })).toBe(true);
    expect(
      hasPackageManagerDeclaration({
        devEngines: { packageManager: { name: 'pnpm', version: '10.17.1' } },
      }),
    ).toBe(true);
  });

  it('reports a lockfile-only manifest as declaration-less', () => {
    expect(hasPackageManagerDeclaration({})).toBe(false);
    expect(hasPackageManagerDeclaration({ devEngines: { runtime: { name: 'node' } } })).toBe(false);
  });

  it('treats an existing field as a declaration even when its value is malformed', () => {
    expect(hasPackageManagerDeclaration({ packageManager: null })).toBe(true);
    expect(hasPackageManagerDeclaration({ devEngines: { packageManager: null } })).toBe(true);
  });
});
