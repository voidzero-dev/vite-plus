# Troubleshooting

Use this page when something in Vite+ is not behaving the way you expect.

::: warning
Vite+ is still in alpha. We are making frequent changes, adding features quickly, and we want feedback to help make it great.
:::

## Supported Tool Versions

Vite+ expects modern upstream tool versions.

- Vite 8 or newer
- Vitest 4.1 or newer

If you are migrating an existing project and it still depends on older Vite or Vitest versions, upgrade those first before adopting Vite+.

## `vp check` does not run type-aware lint rules or type checks

- Confirm that `lint.options.typeAware` and `lint.options.typeCheck` are enabled in `vite.config.ts`
- Check whether your `tsconfig.json` uses `compilerOptions.baseUrl`

The Oxlint type checker path powered by `tsgolint` does not support `baseUrl`, so Vite+ skips `typeAware` and `typeCheck` when that setting is present.

## `vp build` does not run my build script

Unlike package managers, built-in commands cannot be overwritten. If you are trying to run a `package.json` script use `vp run build` instead.

For example:

- `vp build` always runs the built-in Vite build
- `vp test` always runs the built-in Vitest command
- `vp run build` and `vp run test` run `package.json` scripts instead

::: info
You can also run custom tasks defined in `vite.config.ts` and migrate away from `package.json` scripts entirely.
:::

## Staged Checks and Commit Hooks

If `vp staged` fails or your pre-commit hook does not run:

- make sure `vite.config.ts` contains a `staged` block
- run `vp config` to install hooks
- check whether hook installation was skipped intentionally through `VITE_GIT_HOOKS=0`

A minimal staged config looks like this:

```ts
import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    '*': 'vp check --fix',
  },
});
```

## Asking for Help

If you are stuck, please reach out:

- [Discord](https://discord.gg/cAnsqHh5PX) for real-time discussion and troubleshooting help
- [GitHub](https://github.com/voidzero-dev/vite-plus) for issues, discussions, and bug reports

When reporting a problem, please include:

- The full output of `vp env current` and `vp --version`
- The package manager used by the project
- The exact steps needed to reproduce the problem and your `vite.config.ts`
- A minimal reproduction repository or runnable sandbox
