import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { configDefaults } from 'vitest/config';
import { createVitest, type TestUserConfig } from 'vitest/node';

/** The branded checkout deliberately suppresses this notice and aliases Vite's
 * package name. Check those contracts without changing the upstream checkout
 * or accepting unrelated snapshot changes. Fail if the pinned tests drift. */
export function adaptBrandedViteAssertions(source: string): string {
  let notices = 0;
  const result = source.replace(
    /expect\(messages\)\.toMatchInlineSnapshot\(`(?:\\[\s\S]|[^`\\])*`\)/g,
    (snapshot) => {
      if (!snapshot.includes('VITE_CONFIG_NATIVE_IGNORE_WARNING')) {
        return snapshot;
      }
      notices++;
      return 'expect(messages).toEqual([])';
    },
  );
  const packageNames = result.match(/"jsonValue": "vite"/g)?.length ?? 0;
  if (notices !== 8 || packageNames !== 1) {
    throw new Error('Review the vendored Vite branding assertions after updating its revision.');
  }
  return result.replace('"jsonValue": "vite"', '"jsonValue": "@voidzero-dev/vite-plus-core"');
}

const suites: Record<string, { directory: string; config: string; options?: TestUserConfig }> = {
  vite: { directory: 'vite', config: 'vitest.config.ts' },
  rolldown: {
    directory: 'rolldown/packages/rolldown/tests',
    config: 'vitest.config.mts',
    options: { exclude: [...configDefaults.exclude, '**/watch.test.ts', '**/dev-watch.test.ts'] },
  },
  'rolldown-watch': {
    directory: 'rolldown/packages/rolldown/tests',
    config: 'vitest.config.mts',
    options: { include: ['**/watch.test.ts', '**/dev-watch.test.ts'] },
  },
  'rolldown-dev-server': {
    directory: 'rolldown/packages/test-dev-server/tests',
    config: 'vitest.config.fixtures.mts',
  },
};

async function main() {
  const root = fileURLToPath(new URL('../../../', import.meta.url));
  const selected = process.argv.slice(2);
  for (const name of selected.length ? selected : Object.keys(suites)) {
    const suite = suites[name];
    if (!suite) {
      throw new Error(`Unknown vendored suite: ${name}. Use ${Object.keys(suites).join(', ')}.`);
    }
    const directory = path.join(root, suite.directory);
    process.chdir(directory);
    console.log(`\nRunning vendored ${name} with the synchronized Vitest v5 graph`);
    const runner = await createVitest(
      {
        config: suite.config,
        watch: false,
        maxWorkers: 4,
        // The upstream configs predate v5. Keep their explicit v4 mock history
        // contract in this test-only bridge until upstream adopts the default.
        clearMocks: false,
        env: {
          ROLLDOWN_TEST: '1',
          RUST_BACKTRACE: '1',
          // The root install already supplies the v5 bridge. A nested pnpm
          // command must not reinstall the vendored workspace's v4 catalog.
          PNPM_CONFIG_VERIFY_DEPS_BEFORE_RUN: 'false',
          pnpm_config_verify_deps_before_run: 'false',
          NPM_CONFIG_WORKSPACE_DIR: root,
          // Match the package-local PATH supplied by the upstream pnpm scripts.
          PATH: [
            path.join(directory, 'node_modules/.bin'),
            path.join(root, 'node_modules/.bin'),
            // pnpm scripts expose the real manager here. Keep child commands
            // off user-installed vp/Corepack shims with independent runtimes.
            process.env.npm_execpath && path.dirname(process.env.npm_execpath),
            process.env.PATH,
          ]
            .filter(Boolean)
            .join(path.delimiter),
        },
        ...suite.options,
      },
      {
        plugins:
          name === 'vite'
            ? [
                {
                  name: 'vite-plus:vendored-branding-assertions',
                  enforce: 'pre',
                  transform(source, id) {
                    if (
                      id.split('?')[0] ===
                      path
                        .join(directory, 'packages/vite/src/node/__tests__/config.spec.ts')
                        .replaceAll('\\', '/')
                    ) {
                      return { code: adaptBrandedViteAssertions(source), map: null };
                    }
                    return null;
                  },
                },
              ]
            : [],
      },
    );
    try {
      const result = await runner.start();
      if (result.unhandledErrors.length || runner.state.getCountOfFailedTests()) {
        process.exitCode = 1;
      }
    } finally {
      await runner.close();
      process.chdir(root);
    }
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main();
}
