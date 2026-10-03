import { afterEach, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
  run: vi.fn(),
  resolverImports: 0,
  resolveUniversalViteConfig: vi.fn(),
}));

vi.mock('../../binding/index.js', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../binding/index.js')>()),
  ensureBlockingStdio: vi.fn(),
  run: mocks.run,
}));

vi.mock('../resolve-vite-config.js', () => {
  mocks.resolverImports++;
  return { resolveUniversalViteConfig: mocks.resolveUniversalViteConfig };
});

afterEach(() => {
  vi.restoreAllMocks();
  mocks.run.mockReset();
  mocks.resolveUniversalViteConfig.mockReset();
  mocks.resolverImports = 0;
});

it('loads the config resolver only when Rust requests metadata', async () => {
  const originalArgv = process.argv;
  process.argv = [process.execPath, '/path/to/vp', 'dev'];
  mocks.run.mockResolvedValue(1);
  mocks.resolveUniversalViteConfig.mockResolvedValue('{"run":{}}');
  const exit = vi.spyOn(process, 'exit').mockImplementation(() => {
    throw new Error('CLI exited');
  });

  try {
    await expect(import('../bin.ts')).rejects.toThrow('CLI exited');
    expect(mocks.run).toHaveBeenCalledOnce();
    expect(mocks.resolverImports).toBe(0);

    const options = mocks.run.mock.calls[0][0];
    await expect(options.resolveUniversalViteConfig(null, '/project')).resolves.toBe('{"run":{}}');
    expect(mocks.resolverImports).toBe(1);
    expect(mocks.resolveUniversalViteConfig).toHaveBeenCalledWith(null, '/project');
    expect(exit).toHaveBeenCalledWith(1);
  } finally {
    process.argv = originalArgv;
  }
});
