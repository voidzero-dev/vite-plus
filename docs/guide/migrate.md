<script setup lang="ts">
const upgradePrompt = `Upgrade this project from Vite+ 0.3.x to Vite+ 1.0 while preserving its test behavior.

Read these guides before making changes:

- ${__DOCS_ORIGIN__}/guide/migrate
- ${__DOCS_ORIGIN__}/guide/vitest-v5
- https://vitest.dev/guide/migration/

Inspect the worktree and preserve unrelated changes. Keep the original manifests, lockfile, and installed packages available so migration can identify the original Vitest version. Do not update the project's vite-plus or Vitest dependencies before running migration.

Use the CLI from the target Vite+ 1.0 release or its preview build. A global installation is optional. Use a supported Node.js runtime from the Vite+ compatibility guide.

- With a global vp installation, follow ${__DOCS_ORIGIN__}/guide/upgrade to upgrade it and check \`vp toolchain --global\`. Run \`vp help migrate\`, then \`vp migrate --no-interactive\` from the workspace root.
- Without a global installation, run the target CLI through the package manager from the workspace root. For the 1.0.0 release, use \`pnpm dlx --package=vite-plus@1.0.0 vp migrate --no-interactive\` or \`npx --package=vite-plus@1.0.0 vp migrate --no-interactive\`. First run the same command with \`help migrate\` instead of \`migrate --no-interactive\` to read its help. These commands fetch the target CLI without replacing the old project dependencies first.

Replace 1.0.0 with the intended release version. For a preview, use the version from its PR and pass \`--registry=https://registry-bridge.viteplus.dev\` to pnpm or npx before the vp command. Do not run migration with the old project's node_modules/.bin/vp. Keep the existing project setup; do not use --full unless I request it.

Resolve BLOCK findings and rerun migration. Review each REVIEW finding using its documentation link, even if migration exits with success. Preserve test intent and keep the generated v4 compatibility settings and comments until the affected tests support the v5 behavior.

Check workspace manifests, catalogs, overrides, and import changes against the Vite+ guide. Keep test APIs on supported vite-plus/test entries; use @vitest/browser-webdriverio for the community WebDriverIO provider.

Run \`vp install\`, \`vp check\`, and \`vp test\`, plus the project's browser, coverage, and benchmark suites where configured. Run \`vp build\` or \`vp pack\` as appropriate. Without a global CLI, finish installation with the project's package manager, then invoke the updated local CLI through it, such as \`pnpm exec vp check\` or \`npm exec -- vp check\`. Fix migration failures without weakening assertions or dropping test coverage. Report the changes, validation results, and unresolved findings. Do not commit or push unless I ask.`;
</script>

# Migrate to Vite+

`vp migrate` helps move existing projects onto Vite+.

## Overview

This command is the starting point for consolidating separate Vite, Vitest, Oxlint, Oxfmt, ESLint, Prettier, and tsup setups into Vite+.

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
- For a monorepo, the target must be the workspace root. Vite+ cannot migrate one workspace member, because migration updates the package-manager configuration, the catalogs, and the lockfiles that all members share.

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

See [Migration Rules](./migrate-rules.md) for the exact dependency, source rewrite, and package-manager behavior.

Most projects will require further manual adjustments after running `vp migrate`.

For the Vitest v5 upgrade, read the [compatibility settings and review checklist](./vitest-v5.md) and the [upstream Vitest migration guide](https://vitest.dev/guide/migration/). The preflight checks your original runner version and Node runtime before dependency updates. Keep the original lockfile available and resolve blocking findings before retrying.

## Recommended Workflow

Before running the migration:

- For projects that do not use Vite+ yet, upgrade to Vite 8+ and Vitest 4.1+ first
- Make sure you understand any existing lint, format, or test setup that should be preserved

After running the migration:

- Run `vp install`
- Run `vp check`
- Run `vp test`
- Run `vp build` (or `vp pack` if you are building a library)

## Migration Prompt

If you want to hand this work to a coding agent (or the reader is a coding agent!), use this migration prompt:

```md
Migrate this project to Vite+. Vite+ replaces the current split tooling around runtime management, package management, dev/build/test commands, linting, formatting, and packaging. Run `vp help` to understand Vite+ capabilities and `vp help migrate` before making changes. Use `vp migrate --no-interactive` in the workspace root. Make sure the project is using Vite 8+ and Vitest 4.1+ before migrating.

After the migration:

- Confirm `vite` imports were rewritten to `vite-plus` where needed
- Confirm Vitest and browser imports use supported `vite-plus/test*` entries; keep community WebDriverIO provider imports on `@vitest/browser-webdriverio`
- On pnpm, keep the `vite`, `vitest` dependency entries configured by `vp migrate` so the workspace aliases and overrides stay effective; with other package managers, you can remove them once those rewrites are confirmed
- Move remaining tool-specific config into the appropriate blocks in `vite.config.ts`

Command mapping to keep in mind:

- `vp run <script>` is the equivalent of `pnpm run <script>`
- `vp dev` and `vp test` always run the built-ins; `vp run dev` and `vp run test` run the `dev` and `test` scripts from `package.json`
- `vp install`, `vp add`, and `vp remove` delegate through the package manager declared by `packageManager`
- `vp dev`, `vp build`, `vp preview`, `vp lint`, `vp fmt`, `vp check`, and `vp pack` replace the corresponding standalone tools
- Prefer `vp check` for validation loops

Finally, verify the migration by running: `vp install`, `vp check`, `vp test`, and `vp build`

Summarize the migration at the end and report any manual follow-up still required.
```

## Upgrade from Vite+ 0.3 to 1.0

Vite+ 1.0 includes the breaking changes in Vitest 5. Read the [Vite+ compatibility guide](./vitest-v5.md) alongside the [upstream migration guide](https://vitest.dev/guide/migration/).

Keep your project's original dependencies and lockfile until migration identifies the old runner. Updating the project dependencies first can prevent the migration from preserving v4 behavior. Use one of the following paths from the workspace root.

### With the Global CLI

Upgrade the [global CLI](./upgrade.md#global-vp) to your target 1.0 release, then run `vp migrate --no-interactive`. For a preview, follow the [preview installation instructions](./upgrade.md#global-vp-preview).

### Without the Global CLI

Use an existing Node.js runtime that satisfies `^22.18.0 || ^24.11.0 || >=26.0.0`. Run the target migrator through your package manager without adding it to the project first. For the `1.0.0` release:

::: code-group

```bash [pnpm]
pnpm dlx --package=vite-plus@1.0.0 vp migrate --no-interactive
```

```bash [npm]
npx --package=vite-plus@1.0.0 vp migrate --no-interactive
```

:::

Replace `1.0.0` with your target release. For a preview, use the version from the PR and pass `--registry=https://registry-bridge.viteplus.dev` to `pnpm` or `npx` before the `vp` command. Keep the version in `--package` explicit so you run the target migrator rather than the old local CLI.

After migration, finish dependency installation and validate with the updated local CLI:

::: code-group

```bash [pnpm]
pnpm install
pnpm exec vp check
pnpm exec vp test
pnpm exec vp build
```

```bash [npm]
npm install
npm exec -- vp check
npm exec -- vp test
npm exec -- vp build
```

:::

Use `vp pack` in place of `vp build` for a library that uses the pack command. Run configured browser, coverage, and benchmark suites as well.

### Review the Upgrade

On an existing Vite+ project, use the default upgrade flow. Add `--full` if you also want to repeat project setup. Resolve blockers and review the file-specific report before committing. See [Upgrade vs. Full Setup](./migrate-rules.md#upgrade-vs-full-setup) for the scope of each mode.

### Copy Prompt

View and copy this prompt into your coding agent to upgrade an existing Vite+ 0.3 project:

<CopyPrompt :prompt="upgradePrompt" label="View Upgrade Prompt" />

## Tool-Specific Migrations

### Vitest

Vitest is automatically migrated through `vp migrate`. `vite-plus` re-exports upstream `vitest@5.0.1` under `vite-plus/test*`, so for node-mode tests a single `vite-plus` install is enough — you no longer need to install `vitest` directly.

For browser mode, you can use the base browser runtime (`@vitest/browser`) and Preview provider (`@vitest/browser-preview`) included in `vite-plus`. To use Playwright or WebDriverIO, you also need the opt-in provider (`@vitest/browser-playwright` or `@vitest/browser-webdriverio`) and its framework peer (`playwright` or `webdriverio`).

`vp migrate` adds the Playwright provider at the bundled Vitest version and ensures its framework peer. You can import it from `vite-plus/test/browser-playwright`.

For WebDriverIO, import from the community-maintained `@vitest/browser-webdriverio`. Migration restores legacy Vite+ provider imports, ensures a provider version of at least `5.0.0`, and adds an `@vitest/browser` override that matches the bundled runner. Manage later provider upgrades and framework peers yourself. See [Community WebDriverIO provider](./vitest-v5.md#community-webdriverio-provider).

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

When no existing hook policy owns the workflow, `vp migrate` can move supported lint-staged rules and remove the old configuration and dependency. If an existing hook tool is preserved, keep lint-staged in place until you convert that hook policy manually. See the [Commit hooks guide](/guide/commit-hooks) and [Staged config reference](/config/staged) for details.

### Git hook tools

The `vp migrate` command does not automatically convert Husky setups. When Husky is detected, Vite+ leaves its hooks, lifecycle scripts, configuration, and dependencies unchanged and shows a warning. You can migrate the project manually using the [Commit hooks guide](/guide/commit-hooks).

Existing project-owned Vite+ hooks are also preserved. The default staged workflow is introduced only when no existing hook policy is found.

If your project currently uses `lefthook`, `simple-git-hooks`, or `yorkie`, `vp migrate` will leave your existing configuration alone and show a warning. This happens even if you choose to set up hooks during the prompt or include the `--hooks` flag.

If you want to move one of those tools over to Vite+ manually, you can follow these steps. First, move your staged-file commands into the `staged` block within `vite.config.ts`. Then, update your lifecycle script so it runs `vp config`. You will also need to create a Vite+ hook at `.vite-hooks/pre-commit` that runs `vp staged`. Run `vp hooks enable` (or `vp config`) to install the dispatcher and set `core.hooksPath`. Finally, once you have confirmed that the Vite+ hook is working as expected, you can remove the old tool's configuration and dependency.

Use `vp hooks status` to verify the dispatcher is active, and `vp hooks disable` if you need to turn it off again in this clone. You can find more details about the full Vite+ hook setup in the [Commit hooks guide](/guide/commit-hooks).

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
