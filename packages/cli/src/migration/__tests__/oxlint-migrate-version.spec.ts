import { afterEach, describe, expect, it, vi } from 'vitest';

import { resolveOxlintMigrateVersion } from '../migrator/eslint.ts';

// Pin the registry: getNpmRegistry reads the developer's real `.npmrc`, so
// the URL assertions below would fail for anyone using a mirror registry.
vi.mock('../../utils/npm-config.js', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../utils/npm-config.js')>();
  return {
    ...actual,
    getNpmRegistry: () => 'https://registry.npmjs.org',
  };
});

function packumentResponse(packument: unknown): Response {
  return {
    status: 200,
    ok: true,
    json: () => Promise.resolve(packument),
  } as Response;
}

describe('resolveOxlintMigrateVersion', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('keeps the pin when the registry publishes that exact version', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      packumentResponse({
        'dist-tags': { latest: '1.85.0' },
        versions: { '1.84.0': {}, '1.85.0': {} },
      }),
    );
    expect(await resolveOxlintMigrateVersion('1.85.0')).toBe('1.85.0');
  });

  it('falls back to the latest dist-tag when the pin is not published', async () => {
    const mockFetch = vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      packumentResponse({
        'dist-tags': { latest: '1.84.0' },
        versions: { '1.83.0': {}, '1.84.0': {} },
      }),
    );
    expect(await resolveOxlintMigrateVersion('1.85.0')).toBe('1.84.0');
    expect(mockFetch).toHaveBeenCalledWith(
      'https://registry.npmjs.org/@oxlint/migrate',
      expect.anything(),
    );
  });

  it('keeps the pin when the registry request fails', async () => {
    vi.spyOn(globalThis, 'fetch').mockRejectedValue(new TypeError('fetch failed'));
    expect(await resolveOxlintMigrateVersion('1.85.0')).toBe('1.85.0');
  });

  it('keeps the pin when the registry responds non-ok', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue({ status: 500, ok: false } as Response);
    expect(await resolveOxlintMigrateVersion('1.85.0')).toBe('1.85.0');
  });

  it('keeps the pin when the packument has no usable fallback', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(
      packumentResponse({ versions: { '1.84.0': {} } }),
    );
    expect(await resolveOxlintMigrateVersion('1.85.0')).toBe('1.85.0');
  });
});
