import type { UserConfig } from 'vite-plus';

export default (<UserConfig>{
  lint: {
    rules: {
      'no-console': 'warn',
    },
    ignorePatterns: ['**/snap-tests/**', '**/snap-tests-todo/**'],
  },
  test: {
    exclude: [
      '**/node_modules/**',
      '**/snap-tests/**',
      './ecosystem-ci/**',
      './rolldown/**',
      './rolldown-vite/**',
      // FIXME: Error: failed to prepare the command for injection: Invalid argument (os error 22)
      'packages/*/binding/__tests__/',
    ],
  },
  fmt: {
    ignorePatterns: [
      '**/tmp/**',
      '/ecosystem-ci/*/**',
      '/packages/test/**.cjs',
      '/packages/test/**.cts',
      '/packages/test/**.d.mjs',
      '/packages/test/**.d.ts',
      '/packages/test/**.mjs',
      '/packages/test/browser/',
      '/rolldown-vite',
      '/rolldown',
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
        ['ts-equals-import'],
        ['unknown'],
      ],
      newlinesBetween: true,
      order: 'asc',
    },
  },
});
