# Troubleshooting

Use this page when something in Vite+ is not behaving the way you expect.

::: info
Vite+ is in beta: stable, but not yet complete. We are adding features on the road to 1.0 and prioritize community feedback, so please [reach out](#asking-for-help) if something does not work as expected.
:::

## Supported Tool Versions

Vite+ expects modern upstream tool versions.

- Vite 8 or newer
- Vitest 4.1 or newer

If you are migrating an existing project and it still depends on older Vite or Vitest versions, upgrade those first before adopting Vite+.

Run `vp toolchain` to show the versions from the local Vite+ package.
Run `vp toolchain --global` to show the versions from the global Vite+ release.

## `vp check` does not run type-aware lint rules or type checks

- Confirm that `lint.options.typeAware` and `lint.options.typeCheck` are enabled in `vite.config.ts`
- Check whether your `tsconfig.json` still uses `compilerOptions.baseUrl`

The Oxlint type checker path powered by `tsgolint` does not support `baseUrl`.
`vp migrate` and `vp lint --init` try to run the `vp dlx @andrewbranch/ts5to6 --fixBaseUrl .`
fix before enabling type-aware linting. If that fix fails or is declined, Vite+
skips `typeAware` and `typeCheck`.

## VS Code extension does not read `vite.config.ts`

If VS Code has multiple folders open, the shared Oxc language server may pick a different workspace than expected. That can make it look like `vite.config.ts` support is missing.

- Confirm the extension is using the intended workspace.

## `vp dev` or `vp build` does not run my script

Unlike package managers, built-in commands cannot be overwritten. If you are trying to run a `package.json` script use `vp run <script>` instead.

For example:

- `vp dev` always starts the built-in Vite dev server
- `vp build` always runs the built-in Vite build
- `vp test` always runs the built-in Vitest command
- `vp run dev`, `vp run build`, and `vp run test` run the matching `package.json` scripts instead

See [Built-in Commands vs Scripts](/guide/run#built-in-commands-vs-scripts) for when to prefer each path.

::: info
You can also run custom tasks defined in `vite.config.ts` and migrate away from `package.json` scripts entirely.
:::

## Staged Checks and Commit Hooks

If `vp staged` fails or your pre-commit hook does not run:

- make sure `vite.config.ts` contains a `staged` block
- make sure the project-owned pre-commit hook runs `vp staged` (for example `.vite-hooks/pre-commit`)
- run `vp hooks status` to see preference, `core.hooksPath`, and whether the dispatcher is installed
- run `vp hooks enable` (or `vp config`) to install the hook dispatcher
- if status shows `Preference: disabled (local)`, re-enable with `vp hooks enable`
- check whether hooks were skipped intentionally through `VP_GIT_HOOKS=0`

To stop hooks in this clone without deleting project policy files, run `vp hooks disable`.
See the [Commit hooks guide](/guide/commit-hooks) for the full workflow.

A minimal staged config looks like this:

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    '*': 'vp check --fix',
  },
});
```

## Slow config loading caused by heavy plugins

When `vite.config.ts` imports plugins at the top level, they are evaluated for every command, including `vp lint`, `vp fmt`, editor integrations, and long-lived background processes. This can make config loading slow and may trigger plugin setup side effects, such as reading files, starting watchers, or connecting to services.

Use `lazyPlugins` to skip the plugin factory when vite-plus loads your config only to read a metadata block (`lint`, `fmt`, `check`, `staged`, `pack`, `create`, the `run`/`cache` task lookup, and editor tooling). The plugins still load whenever Vite actually runs, `dev`, `build`, `test`, `preview`, and any build your own scripts spawn (a `vp run` task, `vp exec`):

```ts [vite.config.ts]
import { defineConfig, lazyPlugins } from 'vite-plus';
import myPlugin from 'vite-plugin-foo';

export default defineConfig({
  plugins: lazyPlugins(() => [myPlugin()]),
});
```

For heavy plugins that should be lazily imported, combine with dynamic `import()`:

```ts [vite.config.ts]
import { defineConfig, lazyPlugins } from 'vite-plus';

export default defineConfig({
  plugins: lazyPlugins(async () => {
    const { default: heavyPlugin } = await import('vite-plugin-heavy');
    return [heavyPlugin()];
  }),
});
```

## Asking for Help

If you are stuck, please reach out:

- [Discord](https://discord.gg/cAnsqHh5PX) for real-time discussion and troubleshooting help
- [GitHub](https://github.com/voidzero-dev/vite-plus) for issues, discussions, and bug reports

When reporting a problem, please include:

- The full output of `vp env current`, `vp --version`, and `vp toolchain`
- The package manager used by the project
- The exact steps needed to reproduce the problem and your `vite.config.ts`
- A minimal reproduction repository or runnable sandbox
