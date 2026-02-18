import { defineConfig } from 'vite-plus';

export default defineConfig({
  lint: {
    plugins: ['unicorn', 'typescript', 'oxc'],
    categories: {
      correctness: 'error',
      perf: 'error',
      suspicious: 'error',
    },
    rules: {
      'eslint/no-await-in-loop': 'off',
      'no-console': ['error', { allow: ['error'] }],
      'typescript/no-unnecessary-boolean-literal-compare': 'off',
      'typescript/no-unsafe-type-assertion': 'off',
      curly: 'error',
    },
    overrides: [
      {
        files: [
          '.github/**/*',
          'bench/**/*.ts',
          'ecosystem-ci/**/*',
          'packages/*/build.ts',
          'packages/core/rollupLicensePlugin.ts',
          'packages/core/vite-rolldown.config.ts',
          'packages/tools/**/*.ts',
        ],
        rules: {
          'no-console': 'off',
          // Allow variable shadowing in build scripts where it's common and intentional
          'no-shadow': 'off',
        },
      },
      {
        files: [
          'packages/cli/src/oxlint-config.ts',
          // Allow no-shadow in prompts package as it's common pattern in callback-heavy UI code
          'packages/prompts/src/*.ts',
        ],
        rules: {
          'typescript/no-explicit-any': 'off',
          'typescript/no-extraneous-class': 'off',
          'no-shadow': 'off',
        },
      },
      {
        files: ['packages/cli/src/__tests__/index.spec.ts'],
        rules: {
          'typescript/await-thenable': 'off',
        },
      },
    ],
    ignorePatterns: [
      '**/snap-tests/**',
      '**/snap-tests-todo/**',
      'packages/core/rollupLicensePlugin.ts',
      'packages/core/vite-rolldown.config.ts',
      'packages/*/binding/index.d.cts',
      'packages/*/binding/index.d.ts',
    ],
  },
  test: {
    exclude: [
      './ecosystem-ci/**',
      './rolldown-vite/**',
      './rolldown/**',
      '**/node_modules/**',
      '**/snap-tests/**',
      // FIXME: Error: failed to prepare the command for injection: Invalid argument (os error 22)
      'packages/*/binding/__tests__/',
    ],
  },
  fmt: {
    ignorePatterns: [
      '**/tmp/**',
      'packages/cli/snap-tests/fmt-ignore-patterns/src/ignored',
      'ecosystem-ci/*/**',
      'packages/test/**.cjs',
      'packages/test/**.cts',
      'packages/test/**.d.mjs',
      'packages/test/**.d.ts',
      'packages/test/**.mjs',
      'packages/test/browser/',
      'rolldown-vite',
      'rolldown',
    ],
    singleQuote: true,
    semi: true,
    experimentalSortPackageJson: true,
    experimentalSortImports: {
      groups: [
        ['type-import'],
        ['type-builtin', 'value-builtin'],
        ['type-external', 'value-external', 'type-internal', 'value-internal'],
        [
          'type-parent',
          'type-sibling',
          'type-index',
          'value-parent',
          'value-sibling',
          'value-index',
        ],
        ['unknown'],
      ],
      newlinesBetween: true,
      order: 'asc',
    },
  },
  run: {
    tasks: {
      'build:src': {
        command: [
          'vp run @rolldown/pluginutils#build',
          'vp run rolldown#build-binding:release',
          'vp run rolldown#build-node',
          'vp run vite#build-types',
          'vp run @voidzero-dev/vite-plus-core#build',
          'vp run @voidzero-dev/vite-plus-test#build',
          'vp run vite-plus#build',
          'vp run vite-plus-cli#build',
        ].join(' && '),
      },
    },
  },
});
