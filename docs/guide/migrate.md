# Migrate to Vite+

`vp migrate` helps move existing projects onto Vite+.

## Overview

This command is the starting point for consolidating separate Vite, Vitest, Oxlint, Oxfmt, ESLint, and Prettier setups into Vite+.

Use it when you want to take an existing project and move it onto the Vite+ defaults instead of wiring each tool by hand.

## Usage

```bash
vp migrate
vp migrate <path>
vp migrate --no-interactive
```

## Target Path

The positional `PATH` argument is optional.

- If omitted, `vp migrate` migrates the current directory
- If provided, it migrates that target directory instead

```bash
vp migrate
vp migrate my-app
```

## Options

- `--agent <name>` writes agent instructions into the project
- `--no-agent` skips agent instruction setup
- `--editor <name>` writes editor config files into the project
- `--no-editor` skips editor config setup
- `--hooks` sets up pre-commit hooks
- `--no-hooks` skips hook setup
- `--no-interactive` runs the migration without prompts

## Migration Flow

The `migrate` command is designed to move existing projects onto Vite+ quickly. Here is what the command does:

- Updates project dependencies
- Rewrites imports where needed
- Merges tool-specific config into `vite.config.ts`
- Updates scripts to the Vite+ command surface
- Can set up commit hooks
- Can write agent and editor configuration files
- Formats the migrated project

See [Migration Rules](./migrate-rules.md) for the exact dependency, source
rewrite, and package-manager behavior.

Most projects will require further manual adjustments after running `vp migrate`.

## Recommended Workflow

Before running the migration:

- Upgrade to Vite 8+ and Vitest 4.1+ first
- Make sure you understand any existing lint, format, or test setup that should be preserved

After running the migration:

- Run `vp install`
- Run `vp check`
- Run `vp test`
- Run `vp build`

## Migration Prompt

If you want to hand this work to a coding agent (or the reader is a coding agent!), use this migration prompt:

```md
Migrate this project to Vite+. Vite+ replaces the current split tooling around runtime management, package management, dev/build/test commands, linting, formatting, and packaging. Run `vp help` to understand Vite+ capabilities and `vp help migrate` before making changes. Use `vp migrate --no-interactive` in the workspace root. Make sure the project is using Vite 8+ and Vitest 4.1+ before migrating.

After the migration:

- Confirm `vite` imports were rewritten to `vite-plus` where needed
- Confirm `vitest` imports were rewritten to `vite-plus/test` (and `@vitest/browser*` to `vite-plus/test/browser*`) where needed
- Remove old `vite`, `vitest`, and `@vitest/browser*` dependencies only after those rewrites are confirmed — `vite-plus` ships them as direct deps
- Move remaining tool-specific config into the appropriate blocks in `vite.config.ts`

Command mapping to keep in mind:

- `vp run <script>` is the equivalent of `pnpm run <script>`
- `vp test` runs the built-in test command, while `vp run test` runs the `test` script from `package.json`
- `vp install`, `vp add`, and `vp remove` delegate through the package manager declared by `packageManager`
- `vp dev`, `vp build`, `vp preview`, `vp lint`, `vp fmt`, `vp check`, and `vp pack` replace the corresponding standalone tools
- Prefer `vp check` for validation loops

Finally, verify the migration by running: `vp install`, `vp check`, `vp test`, and `vp build`

Summarize the migration at the end and report any manual follow-up still required.
```

## Tool-Specific Migrations

### Vitest

Vitest is automatically migrated through `vp migrate`. `vite-plus` re-exports upstream `vitest@4.x` under `vite-plus/test*`, so for node-mode tests a single `vite-plus` install is enough — you no longer need to install `vitest` directly.

Browser mode is more nuanced. `vite-plus` bundles the base browser runtime (`@vitest/browser`) and the preview provider (`@vitest/browser-preview`), but the **Playwright** and **WebdriverIO** providers stay opt-in: `@vitest/browser-playwright` (with its `playwright` peer) and `@vitest/browser-webdriverio` (with its `webdriverio` peer) are **not** shipped with `vite-plus`, so non-browser projects never pull them in. `vp migrate` detects the provider you actually use and adds it — pinned to the bundled vitest version — together with its framework. If you migrate manually and use one of these providers, install the provider package and its framework yourself so `vite-plus/test/browser-playwright` / `vite-plus/test/browser-webdriverio` can resolve.

If you are migrating manually, update all the imports to `vite-plus/test*` instead:

```ts
// before
import { defineConfig } from 'vitest/config';
import { describe, expect, it, vi } from 'vitest';
import { playwright } from '@vitest/browser-playwright';

const { page } = await import('@vitest/browser/context');

// after
import { defineConfig } from 'vite-plus';
import { describe, expect, it, vi } from 'vite-plus/test';
import { playwright } from 'vite-plus/test/browser-playwright';

const { page } = await import('vite-plus/test/browser/context');
```

`declare module 'vitest'` / `declare module '@vitest/browser*'` augmentations are intentionally **not** rewritten — `vite-plus/test*` is a thin re-export of upstream `vitest*`, so type augmentations have to target the upstream module identity to merge correctly. Leave those `declare module` statements pointing at `'vitest'` / `'@vitest/browser*'`.

### tsdown

If your project uses a `tsdown.config.ts`, move its options into the `pack` block in `vite.config.ts`:

```ts [tsdown.config.ts] {4-6}
import { defineConfig } from 'tsdown';

export default defineConfig({
  entry: ['src/index.ts'],
  dts: true,
  format: ['esm', 'cjs'],
});
```

```ts [vite.config.ts] {4-8}
import { defineConfig } from 'vite-plus';

export default defineConfig({
  pack: {
    entry: ['src/index.ts'],
    dts: true,
    format: ['esm', 'cjs'],
  },
});
```

After merging, delete `tsdown.config.ts`. See the [Pack guide](/guide/pack) for the full configuration reference.

### lint-staged

Vite+ replaces lint-staged with its own `staged` block in `vite.config.ts`. Only the `staged` config format is supported. Standalone `.lintstagedrc` in non-JSON format and `lint-staged.config.*` are not migrated automatically.

Move your lint-staged rules into the `staged` block:

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    '*.{js,ts,tsx,vue,svelte}': 'vp check --fix',
  },
});
```

After migrating, remove lint-staged from your dependencies and delete any lint-staged config files. See the [Commit hooks guide](/guide/commit-hooks) and [Staged config reference](/config/staged) for details.

### Git hook tools

The `vp migrate` command can set up Vite+ commit hooks for you, but it doesn't automatically migrate every type of Git hook tool. This automatic migration path is specifically designed to handle Husky v9+ and lint-staged-style setups. Projects using Husky versions older than 9.0.0 are skipped and should upgrade to Husky v9 before using the automatic migration path.

If your project currently uses `lefthook`, `simple-git-hooks`, or `yorkie`, `vp migrate` will leave your existing configuration alone and show a warning. This happens even if you choose to set up hooks during the prompt or include the `--hooks` flag.

If you want to move one of those tools over to Vite+ manually, you can follow these steps. First, move your staged-file commands into the `staged` block within `vite.config.ts`. Then, update your lifecycle script so it runs `vp config`. You will also need to create a Vite+ hook at `.vite-hooks/pre-commit` that runs `vp staged`. Finally, once you have confirmed that the Vite+ hook is working as expected, you can remove the old tool's configuration and dependency.

You can find more details about the full Vite+ hook setup in the [Commit hooks guide](/guide/commit-hooks).

## Examples

```bash
# Migrate the current project
vp migrate

# Migrate a specific directory
vp migrate my-app

# Run without prompts
vp migrate --no-interactive

# Write agent and editor setup during migration
vp migrate --agent claude --editor zed
```
