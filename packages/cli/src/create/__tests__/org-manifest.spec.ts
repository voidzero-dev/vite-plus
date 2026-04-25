import { afterEach, describe, expect, it, vi } from 'vitest';

import {
  filterManifestForContext,
  OrgManifestSchemaError,
  parseOrgScopedSpec,
  readOrgManifest,
  type OrgTemplateEntry,
} from '../org-manifest.js';

describe('parseOrgScopedSpec', () => {
  it('returns null for non-scoped specs', () => {
    expect(parseOrgScopedSpec('create-vite')).toBeNull();
    expect(parseOrgScopedSpec('vite')).toBeNull();
    expect(parseOrgScopedSpec('./local')).toBeNull();
    expect(parseOrgScopedSpec('')).toBeNull();
  });

  it('parses @scope without a name', () => {
    expect(parseOrgScopedSpec('@your-org')).toEqual({ scope: '@your-org' });
  });

  it('parses @scope@version without a name', () => {
    expect(parseOrgScopedSpec('@your-org@latest')).toEqual({
      scope: '@your-org',
      version: 'latest',
    });
  });

  it('parses @scope:name', () => {
    expect(parseOrgScopedSpec('@your-org:web')).toEqual({ scope: '@your-org', name: 'web' });
  });

  it('parses @scope:name@version', () => {
    expect(parseOrgScopedSpec('@your-org:web@1.2.3')).toEqual({
      scope: '@your-org',
      name: 'web',
      version: '1.2.3',
    });
  });

  it('treats @scope: (empty name) as scope-only', () => {
    expect(parseOrgScopedSpec('@your-org:')).toEqual({ scope: '@your-org' });
  });

  it('returns null for the @scope/name slash form (reserved for existing shorthand)', () => {
    expect(parseOrgScopedSpec('@your-org/web')).toBeNull();
    expect(parseOrgScopedSpec('@your-org/create-web')).toBeNull();
    expect(parseOrgScopedSpec('@your-org/')).toBeNull();
  });
});

describe('filterManifestForContext', () => {
  const templates: OrgTemplateEntry[] = [
    { name: 'monorepo', description: 'root', template: './m', monorepo: true },
    { name: 'web', description: 'web', template: './w' },
    { name: 'library', description: 'lib', template: './l' },
  ];

  it('keeps all entries when not inside a monorepo', () => {
    expect(filterManifestForContext(templates, false)).toEqual(templates);
  });

  it('drops monorepo:true entries when inside a monorepo', () => {
    const filtered = filterManifestForContext(templates, true);
    expect(filtered.map((e) => e.name)).toEqual(['web', 'library']);
  });
});

function packument(
  vpTemplates: unknown,
  extra: Record<string, unknown> = {},
  extraVersions: Record<string, unknown> = {},
) {
  return {
    name: '@your-org/create',
    'dist-tags': { latest: '1.0.0' },
    versions: {
      '1.0.0': {
        version: '1.0.0',
        dist: {
          tarball: 'https://registry.npmjs.org/@your-org/create/-/create-1.0.0.tgz',
          integrity: 'sha512-fake',
        },
        createConfig: vpTemplates !== undefined ? { templates: vpTemplates } : undefined,
        ...extra,
      },
      ...extraVersions,
    },
  };
}

function mockFetchJson(body: unknown, status = 200): ReturnType<typeof vi.spyOn> {
  return vi.spyOn(globalThis, 'fetch').mockResolvedValue({
    status,
    ok: status >= 200 && status < 300,
    async json() {
      return body;
    },
  } as unknown as Response);
}

describe('readOrgManifest', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('returns null on 404', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue({ status: 404, ok: false } as Response);
    expect(await readOrgManifest('@your-org')).toBeNull();
  });

  it('returns null when the package has no createConfig.templates field', async () => {
    mockFetchJson(packument(undefined));
    expect(await readOrgManifest('@your-org')).toBeNull();
  });

  it('returns null when createConfig.templates is an empty array', async () => {
    mockFetchJson(packument([]));
    expect(await readOrgManifest('@your-org')).toBeNull();
  });

  it('parses a valid manifest', async () => {
    mockFetchJson(
      packument([
        { name: 'web', description: 'Web app', template: '@your-org/template-web' },
        { name: 'demo', description: 'Demo', template: './templates/demo', monorepo: true },
      ]),
    );
    const manifest = await readOrgManifest('@your-org');
    expect(manifest).not.toBeNull();
    expect(manifest?.packageName).toBe('@your-org/create');
    expect(manifest?.version).toBe('1.0.0');
    expect(manifest?.tarballUrl).toMatch(/create-1\.0\.0\.tgz$/);
    expect(manifest?.integrity).toBe('sha512-fake');
    expect(manifest?.templates).toHaveLength(2);
    expect(manifest?.templates[1].monorepo).toBe(true);
  });

  it('throws on non-array createConfig.templates', async () => {
    mockFetchJson(packument('nope'));
    await expect(readOrgManifest('@your-org')).rejects.toBeInstanceOf(OrgManifestSchemaError);
  });

  it('throws on an entry missing required fields', async () => {
    mockFetchJson(packument([{ name: 'web', description: 'no template yet' }]));
    await expect(readOrgManifest('@your-org')).rejects.toThrow(
      /createConfig\.templates\[0]\.template/,
    );
  });

  it('throws on duplicate entry names', async () => {
    mockFetchJson(
      packument([
        { name: 'web', description: 'one', template: '@a/one' },
        { name: 'web', description: 'two', template: '@a/two' },
      ]),
    );
    await expect(readOrgManifest('@your-org')).rejects.toThrow(/duplicates an earlier entry/);
  });

  it('throws when a bundled path escapes the package root', async () => {
    mockFetchJson(packument([{ name: 'demo', description: 'x', template: '../outside' }]));
    await expect(readOrgManifest('@your-org')).rejects.toThrow(/escapes the package root/);
  });

  it('throws when an entry name uses the reserved `__vp_` prefix', async () => {
    mockFetchJson(
      packument([{ name: '__vp_builtin_escape__', description: 'collides', template: '@a/b' }]),
    );
    await expect(readOrgManifest('@your-org')).rejects.toThrow(/uses the reserved `__vp_` prefix/);
  });

  it('throws on non-boolean monorepo field', async () => {
    mockFetchJson(
      packument([
        {
          name: 'web',
          description: 'x',
          template: '@a/b',
          monorepo: 'yes',
        },
      ]),
    );
    await expect(readOrgManifest('@your-org')).rejects.toThrow(/monorepo must be a boolean/);
  });

  it('throws when dist.tarball is missing', async () => {
    mockFetchJson({
      name: '@your-org/create',
      'dist-tags': { latest: '1.0.0' },
      versions: {
        '1.0.0': {
          version: '1.0.0',
          dist: {},
          createConfig: { templates: [{ name: 'a', description: 'a', template: '@a/a' }] },
        },
      },
    });
    await expect(readOrgManifest('@your-org')).rejects.toThrow(/missing dist\.tarball/);
  });

  it('throws when the registry responds with a non-404 error', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue({
      status: 500,
      ok: false,
    } as Response);
    await expect(readOrgManifest('@your-org')).rejects.toThrow(/500/);
  });

  it('honors NPM_CONFIG_REGISTRY when fetching the packument', async () => {
    const original = process.env.NPM_CONFIG_REGISTRY;
    process.env.NPM_CONFIG_REGISTRY = 'https://registry.example.com/';
    try {
      const mockFetch = mockFetchJson(
        packument([{ name: 'a', description: 'a', template: '@a/a' }]),
      );
      await readOrgManifest('@your-org');
      expect(mockFetch).toHaveBeenCalledWith(
        'https://registry.example.com/@your-org/create',
        expect.any(Object),
      );
    } finally {
      if (original === undefined) {
        delete process.env.NPM_CONFIG_REGISTRY;
      } else {
        process.env.NPM_CONFIG_REGISTRY = original;
      }
    }
  });

  it('honors scope-specific npm_config_@scope:registry env', async () => {
    const key = 'npm_config_@your-org:registry';
    const original = process.env[key];
    process.env[key] = 'https://private.example.com/';
    try {
      const mockFetch = mockFetchJson(
        packument([{ name: 'a', description: 'a', template: '@a/a' }]),
      );
      await readOrgManifest('@your-org');
      expect(mockFetch).toHaveBeenCalledWith(
        'https://private.example.com/@your-org/create',
        expect.any(Object),
      );
    } finally {
      if (original === undefined) {
        delete process.env[key];
      } else {
        process.env[key] = original;
      }
    }
  });

  it('retries with Bearer auth after a 401 when a matching _authToken is configured', async () => {
    const registryKey = 'npm_config_@your-org:registry';
    const tokenKey = 'npm_config_//private.example.com/:_authToken';
    const originals = {
      [registryKey]: process.env[registryKey],
      [tokenKey]: process.env[tokenKey],
    };
    process.env[registryKey] = 'https://private.example.com/';
    process.env[tokenKey] = 'SECRET-TOKEN';
    try {
      const body = packument([{ name: 'a', description: 'a', template: '@a/a' }]);
      const mockFetch = vi
        .spyOn(globalThis, 'fetch')
        .mockResolvedValueOnce({ status: 401, ok: false } as Response)
        .mockResolvedValueOnce({
          status: 200,
          ok: true,
          async json() {
            return body;
          },
        } as unknown as Response);
      await readOrgManifest('@your-org');
      expect(mockFetch).toHaveBeenCalledTimes(2);
      const [, firstInit] = mockFetch.mock.calls[0] as [string, RequestInit];
      expect(
        (firstInit.headers as Record<string, string> | undefined)?.authorization,
      ).toBeUndefined();
      const [, secondInit] = mockFetch.mock.calls[1] as [string, RequestInit];
      expect((secondInit.headers as Record<string, string>).authorization).toBe(
        'Bearer SECRET-TOKEN',
      );
    } finally {
      for (const [k, v] of Object.entries(originals)) {
        if (v === undefined) {
          delete process.env[k];
        } else {
          process.env[k] = v;
        }
      }
    }
  });

  it('does not send auth when the first request succeeds', async () => {
    const mockFetch = mockFetchJson(packument([{ name: 'a', description: 'a', template: '@a/a' }]));
    await readOrgManifest('@your-org');
    expect(mockFetch).toHaveBeenCalledTimes(1);
    const [, init] = mockFetch.mock.calls[0] as [string, RequestInit];
    expect((init.headers as Record<string, string>).authorization).toBeUndefined();
  });

  it('resolves a pinned version instead of dist-tags.latest', async () => {
    const body = packument(
      [{ name: 'web', description: 'v1', template: '@your-org/template-web' }],
      {},
      {
        '2.0.0-beta.1': {
          version: '2.0.0-beta.1',
          dist: {
            tarball: 'https://registry.npmjs.org/@your-org/create/-/create-2.0.0-beta.1.tgz',
            integrity: 'sha512-beta',
          },
          createConfig: {
            templates: [
              { name: 'web', description: 'beta v2', template: '@your-org/template-web' },
            ],
          },
        },
      },
    );
    mockFetchJson(body);
    const manifest = await readOrgManifest('@your-org', '2.0.0-beta.1');
    expect(manifest?.version).toBe('2.0.0-beta.1');
    expect(manifest?.templates[0].description).toBe('beta v2');
    expect(manifest?.tarballUrl).toMatch(/create-2\.0\.0-beta\.1\.tgz$/);
  });

  it('resolves a dist-tag alias when passed as a version', async () => {
    const body = packument(
      [{ name: 'web', description: 'v1', template: '@your-org/template-web' }],
      {},
    );
    (body as { 'dist-tags': Record<string, string> })['dist-tags'].next = '1.0.0';
    mockFetchJson(body);
    const manifest = await readOrgManifest('@your-org', 'next');
    expect(manifest?.version).toBe('1.0.0');
  });

  it('throws when a pinned version is unknown', async () => {
    mockFetchJson(
      packument([{ name: 'web', description: 'v1', template: '@your-org/template-web' }]),
    );
    await expect(readOrgManifest('@your-org', '9.9.9')).rejects.toThrow(
      /version "9\.9\.9" not found/,
    );
  });
});
