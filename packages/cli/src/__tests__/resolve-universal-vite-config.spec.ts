import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { VITE_CONFIG_FILES } from '../utils/constants.ts';

describe('resolveUniversalViteConfig', () => {
  let root: string;
  const resolveConfig = vi.fn();
  const metadataImport = vi.fn();
  const publicImport = vi.fn();

  beforeEach(() => {
    root = mkdtempSync(path.join(tmpdir(), 'vp-config-metadata-'));
    vi.resetModules();
    resolveConfig.mockReset();
    metadataImport.mockClear();
    publicImport.mockClear();
    vi.doMock('../define-config.ts', () => {
      metadataImport();
      return { withConfigMetadataResolution: (fn: () => Promise<unknown>) => fn() };
    });
    vi.doMock('../index.js', () => {
      publicImport();
      return { resolveConfig };
    });
  });

  afterEach(() => {
    vi.doUnmock('../define-config.ts');
    vi.doUnmock('../index.js');
    vi.resetModules();
    rmSync(root, { recursive: true, force: true });
  });

  it('returns empty metadata without importing the Vite or Vitest entry points', async () => {
    const { resolveUniversalViteConfig } = await import('../resolve-vite-config.ts');
    expect(await resolveUniversalViteConfig(null, root)).toBe('{}');
    expect(metadataImport).not.toHaveBeenCalled();
    expect(publicImport).not.toHaveBeenCalled();
    expect(resolveConfig).not.toHaveBeenCalled();
  });

  it('does not select a config above the supplied workspace root', async () => {
    writeFileSync(path.join(root, 'vite.config.ts'), 'export default {};');
    const child = path.join(root, 'workspace');
    mkdirSync(child);
    const { resolveUniversalViteConfig } = await import('../resolve-vite-config.ts');
    expect(await resolveUniversalViteConfig(null, child)).toBe('{}');
    expect(publicImport).not.toHaveBeenCalled();
  });

  it.each(VITE_CONFIG_FILES)('retains runtime resolution for %s', async (filename) => {
    const configFile = path.join(root, filename);
    writeFileSync(configFile, 'export default {};');
    resolveConfig.mockResolvedValue({ configFile, check: { lint: false } });
    const { resolveUniversalViteConfig } = await import('../resolve-vite-config.ts');
    expect(JSON.parse(await resolveUniversalViteConfig(null, root))).toEqual({
      configFile,
      check: { lint: false },
    });
    expect(resolveConfig).toHaveBeenCalledWith({ root }, 'build');
  });

  it('observes config creation, changes and removal between calls', async () => {
    const { resolveUniversalViteConfig } = await import('../resolve-vite-config.ts');
    const configFile = path.join(root, 'vite.config.ts');
    expect(await resolveUniversalViteConfig(null, root)).toBe('{}');
    writeFileSync(configFile, 'export default { check: { fmt: false } };');
    resolveConfig.mockResolvedValueOnce({ configFile, check: { fmt: false } });
    expect(JSON.parse(await resolveUniversalViteConfig(null, root)).check).toEqual({ fmt: false });
    writeFileSync(configFile, 'export default { check: { lint: false } };');
    resolveConfig.mockResolvedValueOnce({ configFile, check: { lint: false } });
    expect(JSON.parse(await resolveUniversalViteConfig(null, root)).check).toEqual({ lint: false });
    rmSync(configFile);
    expect(await resolveUniversalViteConfig(null, root)).toBe('{}');
    expect(resolveConfig).toHaveBeenCalledTimes(2);
  });

  it('preserves the callback error even without a config', async () => {
    const { resolveUniversalViteConfig } = await import('../resolve-vite-config.ts');
    const error = new Error('callback failed');
    await expect(resolveUniversalViteConfig(error, root)).rejects.toBe(error);
    expect(publicImport).not.toHaveBeenCalled();
  });

  it('retains errors from runtime config evaluation', async () => {
    writeFileSync(path.join(root, 'vite.config.ts'), 'throw new Error("invalid config");');
    const error = new Error('invalid config');
    resolveConfig.mockRejectedValue(error);
    const log = vi.spyOn(console, 'error').mockImplementation(() => {});
    try {
      const { resolveUniversalViteConfig } = await import('../resolve-vite-config.ts');
      await expect(resolveUniversalViteConfig(null, root)).rejects.toBe(error);
      expect(log).toHaveBeenCalledWith('[Vite+] resolve universal vite config error:', error);
    } finally {
      log.mockRestore();
    }
  });
});
