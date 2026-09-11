import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { createVitest } from 'vitest/node';

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

async function main() {
  const root = fileURLToPath(new URL('../../../', import.meta.url));
  if (process.argv.slice(2).some((name) => name !== 'vite')) {
    throw new Error('Only the vendored Vite suite is supported. Use pnpm test:vendored.');
  }
  const directory = path.join(root, 'vite');
  process.chdir(directory);
  console.log('\nRunning vendored Vite with the synchronized Vitest v5 graph');
  try {
    const runner = await createVitest(
      {
        config: 'vitest.config.ts',
        watch: false,
        maxWorkers: 4,
        // The upstream configs predate v5. Keep their explicit v4 mock history
        // contract in this test-only bridge until upstream adopts the default.
        clearMocks: false,
        env: {
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
      },
      {
        plugins: [
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
        ],
      },
    );
    try {
      const result = await runner.start();
      if (result.unhandledErrors.length || runner.state.getCountOfFailedTests()) {
        process.exitCode = 1;
      }
    } finally {
      await runner.close();
    }
  } finally {
    process.chdir(root);
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main();
}
