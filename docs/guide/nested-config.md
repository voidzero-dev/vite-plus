# Nested Configuration

Vite+ supports multiple `vite.config.ts` files in a repository, so packages in a monorepo can have their own lint and format settings while sharing a baseline.

## How `vp lint` and `vp fmt` pick a config

Config resolution is driven by the current working directory (cwd):

- **`vp lint`** uses `<cwd>/vite.config.ts`. If that file is missing, built-in defaults apply — `vp lint` does **not** walk up the directory tree looking for a parent `vite.config.ts`.
- **`vp fmt`** walks up from cwd and uses the first `vite.config.ts` it finds. If none is found, built-in defaults apply.

In both cases, the selected config applies to every path in the run — there is no per-file resolution, and configs are never merged.

If your monorepo needs different settings per package, run `vp lint` / `vp fmt` from each package directory (for example, via `vp run -r lint`), or pin a specific config with `-c`.

If you only want to exclude files or folders from an otherwise-shared config, use [`lint.ignorePatterns`](/config/lint) or [`fmt.ignorePatterns`](/config/fmt) instead.

::: tip Breaking change since the April 2026 release

Earlier versions of Vite+ pinned every `vp lint` / `vp fmt` invocation to the workspace-root `vite.config.ts`, regardless of cwd. Vite+ now lets cwd-based resolution select the config, so running from a sub-package picks up that sub-package's own `vite.config.ts`. See [#1378](https://github.com/voidzero-dev/vite-plus/pull/1378) for the migration notes.

:::

## Example

Given this layout:

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

| cwd                          | config used                         |
| ---------------------------- | ----------------------------------- |
| `my-project/`                | `my-project/vite.config.ts`         |
| `my-project/package1/`       | `my-project/package1/vite.config.ts`|
| `my-project/package1/src/`   | built-in defaults (no walk-up)      |
| `my-project/package2/`       | built-in defaults (no walk-up)      |

`vp fmt`:

| cwd                          | config used                          |
| ---------------------------- | ------------------------------------ |
| `my-project/`                | `my-project/vite.config.ts`          |
| `my-project/package1/`       | `my-project/package1/vite.config.ts` |
| `my-project/package1/src/`   | `my-project/package1/vite.config.ts` |
| `my-project/package2/`       | `my-project/vite.config.ts`          |

## Pinning a config with `-c`

`-c` / `--config` bypasses cwd-based resolution. The specified file is used for every path in the run:

```bash
vp lint -c vite.config.ts
vp fmt --check -c vite.config.ts
```

This also works when you need a one-off config, for example a permissive CI variant.

## `--disable-nested-config` (lint only)

`vp lint` accepts `--disable-nested-config` to stop any auto-loading of nested lint configuration files that may exist in the tree:

```bash
vp lint --disable-nested-config
vp check --disable-nested-config
```

This flag has no effect on `vite.config.ts` resolution, which is already cwd-only for `vp lint`. `vp fmt` has no equivalent flag; use `-c` to pin a single format config.

## Monorepo pattern: share a base config

Configs are never merged automatically — the selected config fully replaces any other. To share a baseline, import the parent config and spread it:

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
