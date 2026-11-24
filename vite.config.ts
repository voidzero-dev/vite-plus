import cloudflare from '@cloudflare/oxlint-config/vite-plus';
import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    '*.@(js|ts|tsx|md|yaml|yml|toml)': 'vp check --fix',
    '*.rs': 'cargo fmt --',
  },
  lint: {
    extends: [cloudflare],
    env: cloudflare.env,
    options: {
      typeAware: true,
      typeCheck: true,
    },
    plugins: ['unicorn', 'typescript', 'oxc'],
    jsPlugins: ['@e18e/eslint-plugin'],
    categories: {
      correctness: 'error',
      perf: 'error',
      suspicious: 'error',
    },
    rules: {
      // e18e's recommended rules, with exceptions for existing integrations and regex style.
      'e18e/ban-dependencies': ['error', { allowed: ['cross-spawn', 'glob'] }],
      'e18e/prefer-array-at': 'error',
      'e18e/prefer-array-fill': 'error',
      'e18e/prefer-array-from-map': 'error',
      'e18e/prefer-array-some': 'error',
      'e18e/prefer-array-to-reversed': 'error',
      'e18e/prefer-array-to-sorted': 'error',
      'e18e/prefer-array-to-spliced': 'error',
      'e18e/prefer-date-now': 'error',
      'e18e/prefer-includes': 'error',
      'e18e/prefer-nullish-coalescing': 'error',
      'e18e/prefer-object-has-own': 'error',
      'e18e/prefer-regex-test': 'error',
      // Its concat autofix can nest array arguments without TypeScript ESLint's type information.
      'e18e/prefer-spread-syntax': 'off',
      'e18e/prefer-static-regex': 'off',
      'e18e/prefer-string-fromcharcode': 'error',
      'e18e/prefer-timer-args': 'error',
      'e18e/prefer-url-canparse': 'error',
      'eslint/no-await-in-loop': 'off',
      'no-console': ['error', { allow: ['error'] }],
      'no-shadow': 'off',
      'typescript/no-unnecessary-boolean-literal-compare': 'off',
      'typescript/no-unnecessary-type-arguments': 'off',
      curly: 'error',
      // Vitest accepts an optional message as the second argument to expect.
      'vitest/valid-expect': ['error', { maxArgs: 2 }],
    },
    overrides: [
      {
        files: ['packages/cli/src/__tests__/{js-command-args,oxlint-plugin}.spec.ts'],
        rules: {
          'vitest/expect-expect': [
            'error',
            { assertFunctionNames: ['expect', 'expectExit', 'checkImports'] },
          ],
        },
      },
      {
        // The theme loads its styles through side-effect imports.
        files: ['docs/.vitepress/theme/index.ts'],
        rules: {
          'import/no-unassigned-import': ['error', { allow: ['**/*.css', 'virtual:*.css'] }],
        },
      },
      {
        // These registry fixtures run as CommonJS executables on older Node versions.
        files: ['crates/vp_cli_snapshots/tests/cli_snapshots/registry/packages/**/*.cjs'],
        rules: { 'typescript/no-require-imports': 'off' },
      },
      {
        // Keep the checks Vite+ already enforces on TypeScript files.
        files: ['**/*.{ts,tsx,mts,cts}'],
        rules: {
          'constructor-super': 'error',
          'getter-return': 'error',
          'no-class-assign': 'error',
          'no-const-assign': 'error',
          'no-dupe-class-members': 'error',
          'no-dupe-keys': 'error',
          'no-func-assign': 'error',
          'no-import-assign': 'error',
          'no-new-native-nonconstructor': 'error',
          'no-obj-calls': 'error',
          'no-setter-return': 'error',
          'no-this-before-super': 'error',
          'no-unreachable': 'error',
          'no-unsafe-negation': 'error',
          'no-with': 'error',
          'typescript/unbound-method': 'error',
        },
      },
      {
        files: ['docs/.vitepress/theme/**/*.vue'],
        globals: {
          defineEmits: 'readonly',
          defineProps: 'readonly',
          withDefaults: 'readonly',
          __DOCS_INSTALL_PS1_URL__: 'readonly',
          __DOCS_INSTALL_SH_URL__: 'readonly',
        },
      },
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
      ...cloudflare.ignorePatterns,
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
      './oxc-node/**',
      '**/node_modules/**',
      // PTY snapshot fixtures; also excluded in lint/fmt here and tsconfig.json
      'crates/vp_cli_snapshots/tests/cli_snapshots/fixtures/**',
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
