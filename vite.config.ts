import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    '*.@(js|ts|tsx|md|yaml|yml)': 'vp check --fix',
    '*.rs': 'cargo fmt --',
  },
  lint: {
    options: {
      typeAware: true,
      typeCheck: true,
    },
    plugins: ['unicorn', 'typescript', 'oxc'],
    categories: {
      correctness: 'error',
      perf: 'error',
      suspicious: 'error',
    },
    rules: {
      'eslint/no-await-in-loop': 'off',
      'no-console': ['error', { allow: ['error'] }],
      'no-shadow': 'off',
      'typescript/no-unnecessary-boolean-literal-compare': 'off',
      'typescript/no-unnecessary-type-arguments': 'off',
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
          'packages/tools/**/*.ts',
        ],
        rules: {
          'no-console': 'off',
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
      // PTY snapshot fixtures; also excluded in test/fmt below and tsconfig.json
      'crates/vp_cli_snapshots/tests/cli_snapshots/fixtures/**',
      'packages/*/binding/**',
    ],
  },
  test: {
    exclude: [
      './ecosystem-ci/**',
      './vite/**',
      './rolldown/**',
      '**/node_modules/**',
      // PTY snapshot fixtures; also excluded in lint/fmt here and tsconfig.json
      'crates/vp_cli_snapshots/tests/cli_snapshots/fixtures/**',
      // FIXME: Error: failed to prepare the command for injection: Invalid argument (os error 22)
      'packages/*/binding/__tests__/',
    ],
  },
  fmt: {
    ignorePatterns: [
      '**/tmp/**',
      // PTY snapshot fixtures; also excluded in lint/test above and tsconfig.json
      'crates/vp_cli_snapshots/tests/cli_snapshots/fixtures/**',
      'ecosystem-ci/*/**',
      'packages/cli/src/run-config.ts',
      'vite',
      'rolldown',
    ],
    singleQuote: true,
    semi: true,
    sortPackageJson: true,
    sortImports: {
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
});
