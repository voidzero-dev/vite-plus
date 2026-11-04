import Module from 'node:module';
import path from 'node:path';

import { ignoreInput, ignoreOutput } from '@voidzero-dev/vite-task-client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vite-plus/test';

import { freezeScriptCache, initializeScriptCache, scriptCachePolicy } from '../script-cache.ts';
import { watchCacheArgs } from '../script-watch-cache.ts';

vi.mock('@voidzero-dev/vite-task-client', () => ({
  ignoreInput: vi.fn(),
  ignoreOutput: vi.fn(),
}));

vi.mock('node:module', async (importOriginal) => {
  const actual = await importOriginal<typeof import('node:module')>();
  return {
    default: {
      constants: actual.constants,
      enableCompileCache: vi.fn(() => ({ status: actual.constants.compileCacheStatus.ENABLED })),
      getCompileCacheDir: vi.fn(() => undefined),
    },
  };
});

beforeEach(() => {
  vi.clearAllMocks();
  for (const name of [
    'VPX_CACHE',
    'VPX_DISABLE_CACHE',
    'NODE_COMPILE_CACHE',
    'NODE_DISABLE_COMPILE_CACHE',
    'NODE_V8_COVERAGE',
    'NODE_OPTIONS',
  ]) {
    vi.stubEnv(name, undefined);
  }
  vi.mocked(Module.getCompileCacheDir).mockReturnValue(undefined);
  vi.mocked(Module.enableCompileCache).mockReturnValue({
    status: Module.constants.compileCacheStatus.ENABLED,
  });
});

afterEach(() => vi.unstubAllEnvs());

describe('script cache policy', () => {
  it('keeps the prototype opt-in and supports separate cache measurements', () => {
    expect(scriptCachePolicy({}, [], '22.23.4')).toEqual({
      transform: false,
      compile: false,
      runtimeVersion: '22.23.4',
    });
    expect(scriptCachePolicy({ VPX_CACHE: '1' }, [])).toMatchObject({
      transform: true,
      compile: true,
    });
    expect(scriptCachePolicy({ VPX_CACHE: 'transform' }, [])).toMatchObject({
      transform: true,
      compile: false,
    });
    expect(scriptCachePolicy({ VPX_CACHE: 'compile' }, [])).toMatchObject({
      transform: false,
      compile: true,
    });
    expect(scriptCachePolicy({ VPX_CACHE: '1', VPX_DISABLE_CACHE: '1' }, [])).toMatchObject({
      transform: false,
      compile: false,
    });
  });

  it.each([
    { NODE_COMPILE_CACHE: '/user-selected' },
    { NODE_DISABLE_COMPILE_CACHE: '1' },
    { NODE_V8_COVERAGE: '/coverage' },
    { NODE_OPTIONS: '--experimental-test-coverage' },
  ])('preserves Node settings and coverage: %j', (env) => {
    expect(scriptCachePolicy({ VPX_CACHE: '1', ...env }, [])).toMatchObject({
      transform: true,
      compile: false,
    });
  });

  it('does not automatically enable compile caching for command-line coverage', () => {
    expect(scriptCachePolicy({ VPX_CACHE: '1' }, ['--experimental_test_coverage'])).toMatchObject({
      transform: true,
      compile: false,
    });
  });
});

describe('script cache initialization', () => {
  const root = path.resolve('owned-cache', 'vpx');

  it('does no native or runner setup on an ordinary launch', () => {
    const binding = { initializeCache: vi.fn(() => root) };
    initializeScriptCache(binding);
    expect(binding.initializeCache).not.toHaveBeenCalled();
    expect(ignoreInput).not.toHaveBeenCalled();
    expect(Module.enableCompileCache).not.toHaveBeenCalled();
  });

  it('excludes owned artifacts before enabling compilation and freezes the result', () => {
    vi.stubEnv('VPX_CACHE', '1');
    const binding = { initializeCache: vi.fn(() => root) };
    const policy = initializeScriptCache(binding);
    expect(binding.initializeCache).toHaveBeenCalledWith(true, process.versions.node, undefined);
    expect(ignoreInput).toHaveBeenCalledExactlyOnceWith(root);
    expect(ignoreOutput).toHaveBeenCalledExactlyOnceWith(root);
    expect(Module.enableCompileCache).toHaveBeenCalledExactlyOnceWith(path.join(root, 'node'));
    expect(vi.mocked(ignoreOutput).mock.invocationCallOrder[0]).toBeLessThan(
      vi.mocked(Module.enableCompileCache).mock.invocationCallOrder[0],
    );
    expect(freezeScriptCache()).toEqual(policy);
    expect(policy).toMatchObject({ transform: true, compile: true, root });
  });

  it('uses the frozen main-thread policy and root in an ESM hooks thread', () => {
    vi.stubEnv('VPX_DISABLE_CACHE', '1');
    const binding = { initializeCache: vi.fn(() => root) };
    const frozen = { transform: true, compile: false, root, runtimeVersion: '22.18.0' };
    expect(initializeScriptCache(binding, frozen)).toEqual(frozen);
    expect(binding.initializeCache).toHaveBeenCalledWith(true, '22.18.0', root);
  });

  it('never excludes a user-selected Node cache, even if it is the workspace root', () => {
    vi.stubEnv('VPX_CACHE', '1');
    vi.stubEnv('NODE_COMPILE_CACHE', process.cwd());
    const policy = initializeScriptCache({ initializeCache: vi.fn(() => root) });
    expect(ignoreInput).toHaveBeenCalledExactlyOnceWith(root);
    expect(ignoreInput).not.toHaveBeenCalledWith(process.cwd());
    expect(Module.enableCompileCache).not.toHaveBeenCalled();
    expect(policy.compile).toBe(false);
  });

  it('keeps a compile cache enabled by an earlier preload', () => {
    vi.stubEnv('VPX_CACHE', 'compile');
    vi.mocked(Module.getCompileCacheDir).mockReturnValue(path.resolve('other-cache'));
    const binding = { initializeCache: vi.fn(() => root) };
    initializeScriptCache(binding);
    expect(binding.initializeCache).not.toHaveBeenCalled();
    expect(ignoreInput).not.toHaveBeenCalled();
    expect(Module.enableCompileCache).not.toHaveBeenCalled();
  });

  it('disables both caches when the native binding rejects the root', () => {
    vi.stubEnv('VPX_CACHE', '1');
    const policy = initializeScriptCache({ initializeCache: vi.fn(() => undefined) });
    expect(policy).toMatchObject({ transform: false, compile: false });
    expect(ignoreInput).not.toHaveBeenCalled();
    expect(Module.enableCompileCache).not.toHaveBeenCalled();
  });

  it('continues with transform caching if Node cannot enable its compile cache', () => {
    vi.stubEnv('VPX_CACHE', '1');
    vi.mocked(Module.enableCompileCache).mockReturnValue({
      status: Module.constants.compileCacheStatus.FAILED,
    });
    const policy = initializeScriptCache({ initializeCache: vi.fn(() => root) });
    expect(policy).toMatchObject({ transform: true, compile: false, root });
  });

  it('does not hide a connected runner reporting failure', () => {
    vi.stubEnv('VPX_CACHE', '1');
    vi.mocked(ignoreInput).mockImplementationOnce(() => {
      throw new Error('runner disconnected');
    });
    expect(() => initializeScriptCache({ initializeCache: vi.fn(() => root) })).toThrow(
      'runner disconnected',
    );
    expect(Module.enableCompileCache).not.toHaveBeenCalled();
  });
});

describe('watch cache arguments', () => {
  it('consumes only the watcher flag, leaving script arguments intact', () => {
    expect(watchCacheArgs(['--no-cache', './main.ts', '--no-cache'])).toEqual({
      disabled: true,
      args: ['./main.ts', '--no-cache'],
    });
    expect(watchCacheArgs(['./main.ts', '--no-cache'])).toEqual({
      disabled: false,
      args: ['./main.ts', '--no-cache'],
    });
  });

  it.each([
    ['--exclude', '--no-cache', './main.ts'],
    ['--require', '--no-cache', './main.ts'],
    ['--', '--no-cache'],
  ])('does not reinterpret option values or escaped entries: %j', (...args) => {
    expect(watchCacheArgs(args)).toEqual({ disabled: false, args });
  });

  it('handles inline/normalized flags and bare Node config flags', () => {
    expect(watchCacheArgs(['--no_cache=false', '--experimental-config-file', './main.ts'])).toEqual(
      {
        disabled: true,
        args: ['--experimental-config-file', './main.ts'],
      },
    );
    expect(
      watchCacheArgs(['--experimental-config-file', './node.json', '--no-cache', './main.ts']),
    ).toEqual({
      disabled: true,
      args: ['--experimental-config-file', './node.json', './main.ts'],
    });
  });
});
