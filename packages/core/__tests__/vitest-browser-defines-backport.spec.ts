import { describe, expect, it, vi } from 'vitest';

import { vitestBrowserDefinesBackportBuildPlugin } from '../build-support/vitest-browser-defines-backport.ts';
import { vitestBrowserDefinesBackportPlugin } from '../src/vitest-browser-defines-backport.ts';

function runner(version = '5.0.0') {
  return { version, injectTestProject: vi.fn(async (_config: unknown) => []) };
}

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
