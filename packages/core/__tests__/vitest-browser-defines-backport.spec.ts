import fs from 'node:fs';

import MagicString from 'magic-string';
import { describe, expect, it, vi } from 'vitest';

import { vitestBrowserDefinesBackportBuildPlugin } from '../build-support/vitest-browser-defines-backport.ts';
import { vitestBrowserDefinesBackportPlugin } from '../src/vitest-browser-defines-backport.ts';

function runner(version = '5.0.0') {
  return { version, injectTestProject: vi.fn(async (_config: unknown) => []) };
}

function inject(
  code: string,
  id = '/checkout/vite/src/node/plugins/index.ts',
  withMagicString = true,
) {
  const plugin = vitestBrowserDefinesBackportBuildPlugin('5.0.0');
  if (!plugin || typeof plugin.transform !== 'function') {
    throw new Error('Expected a backport transform');
  }
  const magicString = withMagicString ? new MagicString(code) : undefined;
  return plugin.transform.call({} as never, code, id, { magicString } as never);
}

describe('temporary browser define build injection', () => {
  const input = `export function resolvePlugins(config) {
  const isBuild = config.command === 'build'
  const isWorker = config.isWorker
  return [
    optimizedDepsPlugin(),
  ]
}`;

  it.each([
    '/checkout/vite/src/node/plugins/index.ts',
    'C:\\checkout\\vite\\src\\node\\plugins\\index.ts',
  ])('injects the guarded plugin at the exact anchor in %s', async (id) => {
    const result = await inject(input, id);
    expect(result && typeof result === 'object' && result.code?.toString()).toContain(
      '    optimizedDepsPlugin(),\n    !isBuild && !isWorker ? vitestBrowserDefinesBackportPlugin() : null,',
    );
    expect(result && typeof result === 'object' && result.code?.toString()).toMatch(
      /^import \{ vitestBrowserDefinesBackportPlugin \} from .*vitest-browser-defines-backport\.ts";/,
    );
  });

  it('ignores unrelated files before checking the anchor', async () => {
    expect(await inject('export {};', '/checkout/vite/src/node/plugins/other.ts')).toBeUndefined();
  });

  it.each([
    input.replace('    optimizedDepsPlugin(),', ''),
    input.replace(
      '    optimizedDepsPlugin(),',
      '    optimizedDepsPlugin(),\n    optimizedDepsPlugin(),',
    ),
    input.replaceAll('isBuild', 'building'),
    input.replaceAll('isWorker', 'worker'),
  ])('fails closed on upstream injection drift', (code) => {
    expect(() => inject(code)).toThrow('Cannot inject the temporary Vitest #11198 backport');
  });

  it('requires the bundler edit buffer', () => {
    expect(() => inject(input, undefined, false)).toThrow(
      'Cannot inject the temporary Vitest #11198 backport',
    );
  });

  it('matches the current vendored Vite source', async () => {
    const input = fs.readFileSync(
      new URL('../../../vite/packages/vite/src/node/plugins/index.ts', import.meta.url),
      'utf8',
    );
    const result = await inject(input);
    expect(result && typeof result === 'object' && result.code?.toString()).toContain(
      '!isBuild && !isWorker ? vitestBrowserDefinesBackportPlugin() : null',
    );
  });
});

describe('temporary Vitest #11198 backport', () => {
  it('leaves browser define initialization to Vite without mutating shared maps', () => {
    const defines = { STRING: '"/messages"', BOOL: 'false', EXPRESSION: '1 + 2' };
    const config = { browser: { enabled: true }, defines };
    vitestBrowserDefinesBackportPlugin().configureVitest.handler({
      vitest: runner(),
      project: { config },
    });
    expect(config.defines).toEqual({});
    expect(config.defines).not.toBe(defines);
    expect(defines).toEqual({ STRING: '"/messages"', BOOL: 'false', EXPRESSION: '1 + 2' });
  });

  it.each([undefined, { enabled: false }])('preserves Node defines with browser %j', (browser) => {
    const defines = { STRING: '/messages', BOOL: false };
    const config = { browser, defines };
    vitestBrowserDefinesBackportPlugin().configureVitest.handler({
      vitest: runner(),
      project: { config },
    });
    expect(config.defines).toBe(defines);
  });

  it.each(['4.1.9', '5.0.0-beta.1', '5.0.1', '5.1.0', '6.0.0'])(
    'does not patch or ship for Vitest %s',
    (version) => {
      const defines = { STRING: '"/messages"' };
      const config = { browser: { enabled: true }, defines };
      const vitest = runner(version);
      const injectTestProject = vitest.injectTestProject;
      vitestBrowserDefinesBackportPlugin().configureVitest.handler({
        vitest,
        project: { config },
      });
      expect(config.defines).toBe(defines);
      expect(vitest.injectTestProject).toBe(injectTestProject);
      expect(vitestBrowserDefinesBackportBuildPlugin(version)).toBe(false);
    },
  );

  it('includes the build injection only for the affected pinned release', () => {
    expect(vitestBrowserDefinesBackportBuildPlugin('5.0.0')).toMatchObject({
      name: 'backport-vitest-browser-defines',
    });
  });

  it('covers injected projects once per runner and preserves the API result', async () => {
    const defines = { STRING: '"/messages"' };
    const browser = { config: { browser: { enabled: true }, defines } };
    const node = { config: { defines } };
    const projects = [browser, node];
    const original = vi.fn(async function (this: unknown, config: unknown) {
      expect(this).toBe(vitest);
      expect(config).toEqual({ test: { name: 'injected' } });
      return projects;
    });
    const vitest = { version: '5.0.0', injectTestProject: original };
    const plugin = vitestBrowserDefinesBackportPlugin();
    plugin.configureVitest.handler({ vitest, project: node });
    const wrapped = vitest.injectTestProject;
    plugin.configureVitest.handler({ vitest, project: node });
    expect(vitest.injectTestProject).toBe(wrapped);
    expect(await vitest.injectTestProject({ test: { name: 'injected' } })).toBe(projects);
    expect(original).toHaveBeenCalledTimes(1);
    expect(browser.config.defines).toEqual({});
    expect(node.config.defines).toBe(defines);
  });

  it('preserves errors from injected project resolution', async () => {
    const error = new Error('invalid project');
    const vitest = runner();
    vitest.injectTestProject.mockRejectedValue(error);
    vitestBrowserDefinesBackportPlugin().configureVitest.handler({
      vitest,
      project: { config: {} },
    });
    await expect(vitest.injectTestProject({})).rejects.toBe(error);
  });
});
