import { mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { resolveConfig, type UserConfig } from 'vite';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { AUTO_INLINE_DEPS, defineConfig } from '../define-config.ts';

const probes = vi.hoisted(() => [] as { from: string; id: string }[]);

vi.mock('node:module', async (importOriginal) => {
  const actual = await importOriginal<typeof import('node:module')>();
  return Object.assign({}, actual, {
    createRequire(from: string | URL) {
      const require = actual.createRequire(from);
      const resolve = require.resolve;
      require.resolve = Object.assign(
        (id: string, options?: { paths?: string[] }) => {
          probes.push({ from: String(from), id });
          return resolve(id, options);
        },
        { paths: (id: string) => resolve.paths(id) },
      );
      return require;
    },
  });
});

vi.mock('node:fs', async (importOriginal) => {
  const actual = await importOriginal<typeof import('node:fs')>();
  return { ...actual, readFileSync: vi.fn(actual.readFileSync) };
});

function loadVitestConfig(root: string) {
  return resolveConfig(
    {
      ...defineConfig({ root, environments: { __vitest__: {} } }),
      configFile: false,
    },
    'serve',
  );
}

describe('Vitest config runtime caches', () => {
  let root: string;

  beforeEach(() => {
    root = realpathSync(mkdtempSync(join(tmpdir(), 'vp-config-cache-')));
    probes.length = 0;
  });

  afterEach(() => rmSync(root, { recursive: true, force: true }));

  function installMatcher(name: string) {
    const directory = join(root, 'node_modules', name);
    mkdirSync(directory, { recursive: true });
    writeFileSync(join(directory, 'package.json'), JSON.stringify({ name, main: 'index.js' }));
    writeFileSync(join(directory, 'index.js'), '');
  }

  function matcherProbes() {
    return probes.filter(
      ({ from, id }) => from === `${root}/package.json` && AUTO_INLINE_DEPS.includes(id),
    );
  }

  it.each(['serve', 'build'] as const)(
    'skips matcher detection in application %s configs even in test mode',
    async (command) => {
      installMatcher('jest-extended');
      const config = await resolveConfig(
        {
          ...defineConfig({
            root,
            mode: 'test',
            resolve: { noExternal: ['app-dependency'] },
            test: { browser: { enabled: true } },
          }),
          configFile: false,
        },
        command,
      );
      expect(matcherProbes()).toEqual([]);
      expect(config).not.toHaveProperty('test.server');
      for (const environment of Object.values(config.environments)) {
        expect(environment.resolve.noExternal).toEqual(['app-dependency']);
      }
    },
  );

  it('skips matcher detection in builds with a Vitest environment', async () => {
    installMatcher('jest-extended');
    const config = await resolveConfig(
      {
        ...defineConfig({ root, environments: { __vitest__: {} } }),
        configFile: false,
      },
      'build',
    );
    expect(matcherProbes()).toEqual([]);
    expect(config).not.toHaveProperty('test');
  });

  it('shares installed and missing package lookups across environment and resolved-config hooks', async () => {
    installMatcher('jest-extended');
    const config = await loadVitestConfig(root);
    expect(matcherProbes().map(({ id }) => id)).toEqual(AUTO_INLINE_DEPS);
    expect(config).toHaveProperty('test.server.deps.inline', ['jest-extended']);
    for (const environment of Object.values(config.environments)) {
      expect(environment.resolve.noExternal).toEqual(['jest-extended']);
    }
  });

  it('refreshes missing package lookups when the same plugins reload the config', async () => {
    const plugins = defineConfig({}).plugins;
    const load = () =>
      resolveConfig(
        { root, plugins, configFile: false, environments: { __vitest__: {} } },
        'serve',
      );
    const first = await load();
    expect(first).not.toHaveProperty('test');
    installMatcher('jest-extended');
    const second = await load();
    expect(second).toHaveProperty('test.server.deps.inline', ['jest-extended']);
    expect(matcherProbes().map(({ id }) => id)).toEqual([...AUTO_INLINE_DEPS, ...AUTO_INLINE_DEPS]);
  });

  it('does not reuse installed packages for another project root', async () => {
    installMatcher('jest-extended');
    // Resolve the second project outside the first project's node_modules ancestry.
    const secondRoot = realpathSync(mkdtempSync(join(tmpdir(), 'vp-config-other-')));
    try {
      const first = await loadVitestConfig(root);
      const second = await loadVitestConfig(secondRoot);
      expect(first).toHaveProperty('test.server.deps.inline', ['jest-extended']);
      expect(second).not.toHaveProperty('test');
      expect(
        probes.filter(
          ({ from, id }) => from === `${secondRoot}/package.json` && AUTO_INLINE_DEPS.includes(id),
        ),
      ).toHaveLength(3);
    } finally {
      rmSync(secondRoot, { recursive: true, force: true });
    }
  });

  it('reads browser export metadata once across entry points and project configs', async () => {
    const manifest = fileURLToPath(import.meta.resolve('vitest/package.json'));
    vi.mocked(readFileSync).mockClear();
    for (let index = 0; index < 2; index++) {
      const config: UserConfig = {
        root,
        environments: { __vitest__: {} },
        test: { browser: { enabled: true } },
      };
      const resolved = await resolveConfig({ ...defineConfig(config), configFile: false }, 'serve');
      for (const id of ['vitest', 'vitest/internal/browser', 'vitest/internal/traces']) {
        expect(resolved.resolve.alias).toContainEqual({
          find: new RegExp(`^${id}$`),
          replacement: fileURLToPath(import.meta.resolve(id)),
        });
      }
    }
    expect(vi.mocked(readFileSync).mock.calls.filter(([file]) => file === manifest)).toHaveLength(
      1,
    );
  });
});
