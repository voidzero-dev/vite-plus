# Nested Configuration

Vite+ supports multiple `vite.config.ts` files in the same repository, so packages in a monorepo can have their own lint and format settings while sharing a baseline.

`vp lint` and `vp fmt` resolve configuration from the **current working directory** (cwd), but with one subtle difference:

- **`vp lint`** — cwd-only. Uses `<cwd>/vite.config.ts` if it exists. If not, falls back to Oxlint's built-in defaults — it does **not** walk up to find an ancestor config.
- **`vp fmt`** — cwd walk-up. Walks up from cwd and uses the first `vite.config.ts` it finds. If none is found anywhere up to the filesystem root, falls back to Oxfmt defaults.

In both cases, the selected config applies to every path in the run.

If you only need to exclude files or folders, use [`lint.ignorePatterns`](/config/lint) or [`fmt.ignorePatterns`](/config/fmt) instead.

## How it works

Given the following structure:

```
my-project/
├── vite.config.ts
├── src/
│   └── index.ts
├── package1/
│   ├── vite.config.ts
│   └── src/index.ts
└── package2/
    └── src/index.ts
```

`vp lint`:

- From `my-project/` → uses `my-project/vite.config.ts` for every file (including files under `package1/` and `package2/`).
- From `my-project/package1/` → uses `my-project/package1/vite.config.ts`.
- From `my-project/package2/` → no local `vite.config.ts`, so Oxlint's built-in defaults are used.
- From `my-project/package1/src/` → no local `vite.config.ts`, so Oxlint's built-in defaults are used even though `package1/vite.config.ts` exists one level up.

`vp fmt`:

- From `my-project/` → uses `my-project/vite.config.ts`.
- From `my-project/package1/` → uses `my-project/package1/vite.config.ts`.
- From `my-project/package2/` → walks up past `package2/` and uses `my-project/vite.config.ts`.
- From `my-project/package1/src/` → walks up past `src/` and uses `my-project/package1/vite.config.ts`.

If your monorepo needs different settings per package, run `vp lint` / `vp fmt` from each package directory (for example, via a `vp run -r lint` task), or pin a specific config with `-c`.

## What to expect

Configuration files are not automatically merged. When a file is selected, it fully replaces any other config — there is no parent/child layering. To share settings, import the parent config and spread it; see the [monorepo pattern](#monorepo-pattern-share-a-base-config) below.

Command-line options override configuration files.

Passing an explicit config file location using `-c` or `--config` bypasses cwd-based resolution entirely, and only that single configuration file is used — for both `vp lint` and `vp fmt`.

For lint, you can also pass `--disable-nested-config` to stop Oxlint from picking up any stray legacy config files that may exist in the tree:

```bash
vp lint --disable-nested-config
vp check --disable-nested-config
```

There is no equivalent flag for `vp fmt`; pass `-c` if you need to pin a single format config.

`options.typeAware` and `options.typeCheck` are root-config-only. If either is set in a nested `vite.config.ts` that ends up being selected as the lint config, `vp lint` reports an error.

::: tip Breaking change since the April 2026 release

Earlier versions of Vite+ always injected the workspace-root `vite.config.ts` into every `vp lint` / `vp fmt` invocation, regardless of cwd. Vite+ now lets cwd-based resolution select the config, so running `vp lint` / `vp fmt` from inside a sub-package picks up that sub-package's own `vite.config.ts`. See [#1378](https://github.com/voidzero-dev/vite-plus/pull/1378) for the migration notes.

:::

## Monorepo pattern: share a base config

In a monorepo, you often want one shared baseline at the root and small package-specific adjustments. Import the root `vite.config.ts` from the nested one and spread it:

```ts [my-project/vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  lint: {
    rules: {
      'no-debugger': 'error',
    },
  },
});
```

```ts [my-project/package1/vite.config.ts]
import { defineConfig } from 'vite-plus';
import baseConfig from '../vite.config.ts';

export default defineConfig({
  ...baseConfig,
  lint: {
    ...baseConfig.lint,
    rules: {
      ...baseConfig.lint?.rules,
      'no-console': 'off',
    },
  },
});
```

This keeps the shared baseline in one place and makes package configs small and focused.
