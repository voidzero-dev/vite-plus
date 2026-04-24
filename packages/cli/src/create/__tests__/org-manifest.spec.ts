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
    expect(parseOrgScopedSpec('@your-org@latest')).toEqual({ scope: '@your-org' });
  });

  it('parses @scope/name', () => {
    expect(parseOrgScopedSpec('@your-org/web')).toEqual({ scope: '@your-org', name: 'web' });
  });

  it('parses @scope/name@version', () => {
    expect(parseOrgScopedSpec('@your-org/web@1.2.3')).toEqual({ scope: '@your-org', name: 'web' });
  });

  it('treats a trailing slash as scope-only', () => {
    expect(parseOrgScopedSpec('@your-org/')).toEqual({ scope: '@your-org' });
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

function packument(vpTemplates: unknown, extra: Record<string, unknown> = {}) {
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
});
