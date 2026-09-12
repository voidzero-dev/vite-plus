import { existsSync, readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { defineConfig, defineProject } from 'vite-plus';
import { playwright } from 'vite-plus/test/browser-playwright';

const require = createRequire(import.meta.resolve('vite-plus/package.json'));
const executablePath = existsSync('chromium-path.json')
  ? JSON.parse(readFileSync('chromium-path.json', 'utf8'))
  : require('playwright').chromium.executablePath();

export function project(name, browser = true) {
  return {
    define: {
      __VP_STRING_DEFINE__: JSON.stringify('/messages?locale="en"'),
      __VP_FALSE_DEFINE__: 'false',
      __VP_TRUE_DEFINE__: 'true',
      __VP_NUMBER_DEFINE__: '0',
      __VP_OBJECT_DEFINE__: JSON.stringify({ url: '/messages', enabled: false }),
      __VP_EXPRESSION_DEFINE__: '1 + 2',
      'globalThis.__VP_DOTTED_DEFINE__': JSON.stringify('dotted'),
    },
    test: {
      name,
      slowTestThreshold: 60000,
      include: ['defines.test.js'],
      ...(browser ? {
        browser: {
          enabled: true,
          headless: true,
          provider: playwright({ launchOptions: { executablePath } }),
          instances: [{ browser: 'chromium' }],
        },
      } : {}),
    },
  };
}

export function configFor(name) {
  switch (name) {
    case 'injected-hook': {
      const config = project('root');
      config.plugins = [{
        name: 'inject-browser-project',
        configureVitest: {
          order: 'pre',
          async handler({ injectTestProjects }) {
            await injectTestProjects({ extends: false, ...project('hook-injected') });
          },
        },
      }];
      return config;
    }
    case 'injected': return project('root');
    case 'raw': return project('raw');
    case 'helper': return defineConfig(project('helper'));
    case 'shared': {
      const config = project('shared');
      config.test.browser.instances = [
        { name: 'first-instance', browser: 'chromium' },
        { name: 'second-instance', browser: 'chromium' },
      ];
      return config;
    }
    case 'shared-node': {
      const config = project('root', false);
      config.test.projects = [{ test: { name: 'first' } }, { test: { name: 'second' } }];
      return config;
    }
    case 'inherited': {
      const config = project('root');
      config.test.projects = [
        { test: project('shared').test },
        { extends: true, test: project('inherited').test },
      ];
      return config;
    }
    case 'separate': {
      const config = project('root');
      config.test.sharedViteServer = false;
      config.test.projects = [{ test: project('first').test }, { test: project('second').test }];
      return config;
    }
    case 'independent': return {
      test: { projects: [
        { extends: false, ...project('raw-independent') },
        defineProject({ extends: false, ...project('helper-independent') }),
        { extends: false, ...project('node', false) },
      ] },
    };
    case 'referenced': return { test: { projects: ['./vitest.raw.config.mjs', './vitest.helper.config.mjs'] } };
    case 'nested': return { test: { projects: ['./vitest.nested.config.mjs'] } };
    default: throw new Error(`Unknown config ${name}`);
  }
}
