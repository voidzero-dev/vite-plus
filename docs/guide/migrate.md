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
- Confirm `vitest` imports were rewritten to `vite-plus/test` where needed
- Remove old `vite` and `vitest` dependencies only after those rewrites are confirmed
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

## tsdown

If your project uses a `tsdown.config.ts`, move its options into the `pack` block in `vite.config.ts`:

```ts
// before — tsdown.config.ts
import { defineConfig } from 'tsdown';

export default defineConfig({
  entry: ['src/index.ts'],
  dts: true,
  format: ['esm', 'cjs'],
});
```

```ts
// after — vite.config.ts
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

## lint-staged

Vite+ replaces lint-staged with its own `staged` block in `vite.config.ts`. Only the `staged` config format is supported — standalone `.lintstagedrc` in non-JSON format and `lint-staged.config.*` are not migrated automatically.

Move your lint-staged rules into the `staged` block:

```ts
// vite.config.ts
import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    '*.{js,ts,tsx,vue,svelte}': 'vp check --fix',
  },
});
```

After migrating, remove lint-staged from your dependencies and delete any lint-staged config files. See the [Commit hooks guide](/guide/commit-hooks) and [Staged config reference](/config/staged) for details.

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
