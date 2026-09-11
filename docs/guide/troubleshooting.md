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

## Nested lint or format config is not applied

Vite+ does not currently support nested lint or format configuration. When running `vp lint`, `vp fmt`, or `vp check` from the workspace root, do not rely on configs in subdirectories or on `lint` and `fmt` blocks in package-level `vite.config.ts` files to override the root settings.

We're holding off on nested config support for now. Some of the factors we're considering are how implicit config discovery affects the predictability of linting and formatting, what context AI agents need to understand the settings that apply, and the potential performance cost of finding and loading multiple configs. At the same time, we recognize that keeping package-specific context close to the code may have benefits. The use cases we've heard so far haven't given us a strong enough reason to commit to those semantics. Waiting leaves room to add support later, and we'd like to hear why your project needs nested configs, especially where root-level overrides fall short.

Keep lint and format settings in the root `vite.config.ts`. Use [`lint.overrides`](/guide/monorepo#root-config-with-overrides) and [`fmt.overrides`](/guide/monorepo#format-overrides) for file- or package-specific settings. You can also [import configuration objects](/guide/monorepo#composing-configuration-files) into the root config to keep settings in separate files.

For format-on-save, [disable nested formatter configs](/guide/fmt#configuration) in the editor so it uses the root Vite+ `fmt` block.

Do you need nested configs? [Share your use case and opinion on GitHub](#asking-for-help), including your project structure, the reason why you want them and whether root-level overrides meet your needs.

We sincerely hope to hear your feedback. This will help us decide whether to improve the current situation in the future.

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
