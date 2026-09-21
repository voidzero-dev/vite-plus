<script setup lang="ts">
const compatibilityReviewPrompt = `After establishing a passing baseline, try to remove the generated "Vitest v4 compatibility" settings with no code changes or small, localized fixes:

1. Use the migration diff and generated comments to identify additions in root, workspace, and inline project configs. Read each linked explanation and check the effective setting after removal, including inherited values. Preserve pre-existing user settings and settings whose origin is unclear.
2. Remove one added setting at a time and first run the affected projects and suites without code changes. If needed, make small, localized application, test, or setup fixes that preserve test intent, such as correcting a locator or adjusting mock setup in a few tests. Do not weaken assertions, accept snapshot changes without review, or reduce the test set. For fakeTimers.toNotFake, remove only the added Temporal entry and preserve other exclusions. Do not weaken coverage enforcement: retain glob-threshold perFile: true unless I approve aggregate checking, even if coverage passes.
3. Keep a removal only when the affected tests pass and still execute the same tests without new skips. Remove that setting's generated comment too. If removal requires widespread test edits or shared setup refactoring, keep compatibility for now and report the follow-up work. Restore the setting and its comment if validation still fails, cannot run, or leaves uncertainty about behavior. Undo only cleanup-specific trial edits; preserve completed migration fixes and unrelated work.
4. Run the full validation commands again with the accepted removals together. Report each candidate's config path, removed or retained status, code changes, commands and results, and the reason for retaining it. Distinguish a deferred rewrite from a setting you could not validate.

For projects that use vp pack, also review the generated "tsdown <0.23 compatibility" settings after a passing library build. Use the migration diff and linked comments to distinguish inserted settings from pre-existing user choices. Removing deps.resolveDepSubpath: true preserves external subpath imports as written; check that consumers can still resolve the emitted imports. Removing attw.profile: 'strict' adopts the esm-only profile, which skips node10 and CommonJS resolution checks. Keep strict if those checks are part of the package's intended support. Do not drop intended declaration checks merely to make a build pass. Remove one setting and its generated comment at a time, then run vp pack and the package's consumer checks. Restore the setting and comment if validation fails or the required consumer behavior is unclear. Report which settings remain and why.`;

const migrationPrompt = `Migrate this project to Vite+ while preserving its application, test, and library build behavior.

Read these guides before making changes:

- ${__DOCS_ORIGIN__}/guide/migrate
- ${__DOCS_ORIGIN__}/guide/migrate-rules
- ${__DOCS_ORIGIN__}/guide/vitest-v5
- https://vitest.dev/guide/migration/
- https://github.com/rolldown/tsdown/releases/tag/v0.23.0

Inspect the worktree and preserve unrelated changes. Identify the workspace root, package manager, scripts, configuration files, and tools in use. If the project already uses Vite+, follow the upgrade flow in the migration guide and keep the existing setup; do not use --full unless I request it.

For a project that does not use Vite+ yet, check the prerequisites for the tools it uses: Vite 8+ and Vitest 4.1+. Complete any required upstream upgrades and validate them before starting the Vite+ migration. Then keep those manifests, lockfile, and installed packages available so the migrator can identify the original Vitest version. Do not install vite-plus or upgrade Vitest to the target's bundled version before running migration.

Choose the intended Vite+ release or preview and use its CLI. Use a supported Node.js runtime from the compatibility guide. A global installation is optional:

- With a global vp installation, follow ${__DOCS_ORIGIN__}/guide/upgrade to select the target release and check \`vp toolchain --global\`. Run \`vp help\` and \`vp help migrate\`, then \`vp migrate --no-interactive\` from the workspace root.
- Without a global installation, replace <target-version> with the intended version and run \`pnpm dlx --package=vite-plus@<target-version> vp migrate --no-interactive\` or \`npx --package=vite-plus@<target-version> vp migrate --no-interactive\` from the workspace root. First run the same command with \`help migrate\` instead of \`migrate --no-interactive\` to read its help. For a preview, use the version from its PR and pass \`--registry=https://registry-bridge.viteplus.dev\` to pnpm or npx before the vp command.

Do not run migration with an old project's node_modules/.bin/vp. Migrate monorepos from the workspace root so shared manifests, catalogs, overrides, and lockfiles remain consistent.

Resolve BLOCK findings and rerun migration. Review every REVIEW finding and manual-migration warning using the linked guidance, even if migration exits with success. Keep the generated Vitest v4 and tsdown <0.23 compatibility settings and comments for the first validation run.

Review the resulting changes against the migration rules:

- Confirm Vite and test imports use the supported vite-plus and vite-plus/test* entries. Keep type augmentations on their upstream module identities and the community WebDriverIO provider on @vitest/browser-webdriverio.
- Preserve dependencies, aliases, catalogs, and overrides configured by the migrator. On pnpm, keep the configured vite and vitest entries. Retain upstream packages when the migration rules require them.
- Move remaining tool-specific configuration into the appropriate blocks in vite.config.ts. Review manual follow-up for lint, formatting, packaging, and hooks without discarding project-specific behavior.
- Distinguish built-ins from tasks: \`vp dev\` and \`vp test\` run built-in tools; \`vp run dev\` and \`vp run test\` run the corresponding project scripts or tasks. The packageManager field selects the package manager used by \`vp install\`, \`vp add\`, and \`vp remove\`.

Run \`vp install\`, \`vp check\`, and \`vp test\`, plus configured browser, coverage, and benchmark suites. Run \`vp build\` for applications and \`vp pack\` for libraries, including both where the workspace contains both. Check library consumers against emitted imports and declarations. Without a global CLI, install with the project's package manager and invoke the updated local CLI through it, such as \`pnpm exec vp check\` or \`npm exec -- vp check\`. Fix failures without weakening assertions or dropping test coverage.

${compatibilityReviewPrompt}

Report the migration changes, validation results, retained compatibility settings, and unresolved findings. Do not commit or push unless I ask.`;

const upgradePrompt = `Upgrade this project from Vite+ 0.3.x to Vite+ 1.0 while preserving its test and library build behavior.

Read these guides before making changes:

- ${__DOCS_ORIGIN__}/guide/migrate
- ${__DOCS_ORIGIN__}/guide/vitest-v5
- https://vitest.dev/guide/migration/
- ${__DOCS_ORIGIN__}/guide/migrate-rules#pack-configuration
- https://github.com/rolldown/tsdown/releases/tag/v0.23.0

Inspect the worktree and preserve unrelated changes. Keep the original manifests, lockfile, and installed packages available so migration can identify the original Vitest version. Do not update the project's vite-plus or Vitest dependencies before running migration.

Use the CLI from the target Vite+ 1.0 release or its preview build. A global installation is optional. Use a supported Node.js runtime from the Vite+ compatibility guide.

- With a global vp installation, follow ${__DOCS_ORIGIN__}/guide/upgrade to upgrade it and check \`vp toolchain --global\`. Run \`vp help migrate\`, then \`vp migrate --no-interactive\` from the workspace root.
- Without a global installation, run the target CLI through the package manager from the workspace root. For the 1.0.0 release, use \`pnpm dlx --package=vite-plus@1.0.0 vp migrate --no-interactive\` or \`npx --package=vite-plus@1.0.0 vp migrate --no-interactive\`. First run the same command with \`help migrate\` instead of \`migrate --no-interactive\` to read its help. These commands fetch the target CLI without replacing the old project dependencies first.

Replace 1.0.0 with the intended release version. For a preview, use the version from its PR and pass \`--registry=https://registry-bridge.viteplus.dev\` to pnpm or npx before the vp command. Do not run migration with the old project's node_modules/.bin/vp. Keep the existing project setup; do not use --full unless I request it.

Resolve BLOCK findings and rerun migration. Review each REVIEW finding using its documentation link, even if migration exits with success. Preserve test intent and keep the generated Vitest v4 and tsdown <0.23 compatibility settings and comments for the first validation run.

Check workspace manifests, catalogs, overrides, and import changes against the Vite+ guide. Keep test APIs on supported vite-plus/test entries; use @vitest/browser-webdriverio for the community WebDriverIO provider.

Run \`vp install\`, \`vp check\`, and \`vp test\`, plus the project's browser, coverage, and benchmark suites where configured. Run \`vp build\` or \`vp pack\` as appropriate. Without a global CLI, finish installation with the project's package manager, then invoke the updated local CLI through it, such as \`pnpm exec vp check\` or \`npm exec -- vp check\`. Fix migration failures without weakening assertions or dropping test coverage.

${compatibilityReviewPrompt}

Report the migration changes and unresolved findings as well. Do not commit or push unless I ask.`;
</script>

# Migrate to Vite+

`vp migrate` helps move existing projects onto Vite+.

## Overview

This command is the starting point for consolidating separate Vite, Vitest, Oxlint, Oxfmt, ESLint, Prettier, tsdown, and tsup setups into Vite+.

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

View and copy this prompt into your coding agent to migrate an existing project to Vite+:

<CopyPrompt :prompt="migrationPrompt" label="View Migration Prompt" />

## Upgrade from Vite+ 0.3 to 1.0

Vite+ 1.0 includes the breaking changes in Vitest 5. Read the [Vite+ compatibility guide](./vitest-v5.md) alongside the [upstream migration guide](https://vitest.dev/guide/migration/).

Library builds also use tsdown 0.23. Review the [pack configuration migration rules](./migrate-rules.md#pack-configuration) and [upstream release notes](https://github.com/rolldown/tsdown/releases/tag/v0.23.0) for option changes and new defaults.

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

After the migrated project passes validation, [check whether you can remove the generated v4 compatibility settings](./vitest-v5.md#remove-unneeded-compatibility-settings) with no code changes or small, localized fixes. Keep compatibility for now if removal requires extensive test changes or you cannot validate the result.

For libraries, review the generated [tsdown compatibility settings](#tsdown) after a successful `vp pack` run. Follow their documentation links before adopting the new defaults, and check the emitted imports and declarations with your package's consumers.

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

`vp migrate` updates supported static options in `pack` blocks and `tsdown.config.*` for tsdown 0.23. This also runs for existing Vite+ projects and workspace packages without `--full`. See [Pack Configuration](./migrate-rules.md#pack-configuration) for the option mappings and cases that require manual review.

The transform preserves earlier defaults by inserting `deps.resolveDepSubpath: true` and, when ATTW checks are enabled, `attw.profile: 'strict'` if these settings are absent. Each inserted setting includes a `tsdown <0.23 compatibility` comment with a documentation link and removal guidance. Explicit settings remain unchanged.

Keep these settings for the first `vp pack` run. After validation, review whether your package can adopt the new defaults:

- Removing `deps.resolveDepSubpath: true` [preserves external subpath imports as written](https://tsdown.dev/options/dependencies#deps-resolvedepsubpath). Check that consumers can resolve the emitted imports.
- Removing the inserted `attw.profile: 'strict'` selects the `esm-only` [resolution profile](https://tsdown.dev/options/lint#profiles). This skips `node10` and CommonJS resolution checks. Keep `strict` if your package requires those checks.

Remove each accepted setting's generated comment too. Run `vp pack` and your package's consumer checks after each change. Keep the setting when you cannot validate the result.

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
