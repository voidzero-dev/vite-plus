These are the official instructions for setting up Vite+ globally and using it for the current project. Complete the steps yourself by running commands directly.

Let `<docs-origin>` mean the origin that served this file, such as `https://viteplus.dev`. Use that origin for the Vite+ documentation links below so preview instructions stay on the same release.

First, read `<docs-origin>/llms-full.txt` and `<docs-origin>/guide` to learn Vite+'s commands and configuration. Inspect the worktree and preserve unrelated changes.

## Install or verify the global `vp` CLI

Run `vp toolchain --global`. Only a successful global toolchain report proves that the global CLI is installed; `vp --version` may resolve a project-local binary and is not sufficient.

If the global CLI is unavailable, install it for the current operating system:

### macOS or Linux

```bash
curl -fsSL https://vite.plus | bash
```

### Windows PowerShell

```powershell
irm https://vite.plus/ps1 | iex
```

When this file comes from a non-production docs deployment, use `<docs-origin>/install.sh` or `<docs-origin>/install.ps1` instead so the CLI matches that preview.

Open a new shell after installation if necessary. Follow `<docs-origin>/guide/upgrade` to select the intended release or preview, then run `vp toolchain --global` and `vp help` to verify the global installation.

## Install the Vite+ Agent Skill globally

Find the current agent's identifier in the definitive [Supported Agents](https://github.com/vercel-labs/skills#supported-agents) table. Install the skill only for that agent; replace `<agent-id>` with its identifier and do not use `*`.

```bash
vp dlx skills add https://github.com/voidzero-dev/vite-plus/tree/main/skills/vite-plus --global --yes --agent <agent-id>
```

If the table says the agent is project-only or does not list it, leave the verified global `vp` installation in place and report that the global skill was not installed. Do not edit an unknown global configuration file manually.

Verify the skill and report its installed path:

```bash
vp dlx skills list --global --agent <agent-id>
```

After completing the matching project flow below, tell the user to restart their agent so it loads the skill.

To update or remove the skill later, use:

```bash
vp dlx skills update vite-plus --global --yes
vp dlx skills remove vite-plus --global --yes
```

## Follow the matching project flow

Determine whether the user wants a new project or wants to migrate or upgrade an existing project. Follow only the matching flow.

### New project

Read `<docs-origin>/guide/create` and choose the template, target directory, package manager, and intended Vite+ release or preview for the project. Use the target CLI's `help create` output to select supported options. Scaffold with `vp create`; do not overwrite existing project files.

Without a global installation, use a supported Node.js runtime from the compatibility guide. For the 1.0.0 release, run `pnpm dlx --package=vite-plus@1.0.0 vp create` or `npx --package=vite-plus@1.0.0 vp create`. Replace `1.0.0` with the intended release version. For a preview, use the version from its PR and pass `--registry=https://registry-bridge.viteplus.dev` to pnpm or npx before the `vp` command.

Run `vp install`, `vp check`, and `vp test`, then `vp build` for applications or `vp pack` for libraries. Without a global CLI, install with the project's package manager and run the local CLI through it, such as `pnpm exec vp check` or `npm exec -- vp check`. Explain how to use `vp dev` for the dev server and `vp run <task>` for project scripts or tasks. Report the setup changes, validation results, and any remaining work. Do not commit or push unless asked.

### Existing project

Migrate this project to Vite+ while preserving its application, test, and library build behavior.

Read these guides before making changes:

- `<docs-origin>/guide/migrate`
- `<docs-origin>/guide/migrate-rules`
- `<docs-origin>/guide/vitest-v5`
- https://vitest.dev/guide/migration/
- https://github.com/rolldown/tsdown/releases/tag/v0.23.0

Inspect the worktree and preserve unrelated changes. Identify the workspace root, package manager, scripts, configuration files, and tools in use. If the project already uses Vite+, follow the upgrade flow in the migration guide and keep the existing setup; do not use `--full` unless requested.

For a project that does not use Vite+ yet, check the prerequisites for the tools it uses: Vite 8+ and Vitest 4.1+. Complete any required upstream upgrades and validate them before starting the Vite+ migration. Then keep those manifests, lockfile, and installed packages available so the migrator can identify the original Vitest version. Do not install vite-plus or upgrade Vitest to the target's bundled version before running migration.

Use the CLI from the target Vite+ 1.0 release or its preview build. Use a supported Node.js runtime from the compatibility guide:

- With a global `vp` installation, follow `<docs-origin>/guide/upgrade` to select the target release and check `vp toolchain --global`. Run `vp help` and `vp help migrate`, then `vp migrate --no-interactive` from the workspace root.
- Without a global installation, run the target CLI through the package manager from the workspace root. For the 1.0.0 release, use `pnpm dlx --package=vite-plus@1.0.0 vp migrate --no-interactive` or `npx --package=vite-plus@1.0.0 vp migrate --no-interactive`. First run the same command with `help migrate` instead of `migrate --no-interactive` to read its help. These commands fetch the target CLI without replacing the old project dependencies first.

Replace `1.0.0` with the intended release version. For a preview, use the version from its PR and pass `--registry=https://registry-bridge.viteplus.dev` to pnpm or npx before the `vp` command.

Do not run migration with an old project's `node_modules/.bin/vp`. Migrate monorepos from the workspace root so shared manifests, catalogs, overrides, and lockfiles remain consistent.

Resolve BLOCK findings and rerun migration. Review every REVIEW finding and manual-migration warning using the linked guidance, even if migration exits with success. Keep the generated Vitest v4 and tsdown <0.23 compatibility settings and comments for the first validation run.

Review the resulting changes against the migration rules:

- Confirm Vite and test imports use the supported `vite-plus` and `vite-plus/test*` entries. Keep type augmentations on their upstream module identities and the community WebDriverIO provider on `@vitest/browser-webdriverio`.
- Preserve dependencies, aliases, catalogs, and overrides configured by the migrator. On pnpm, keep the configured vite and vitest entries. Retain upstream packages when the migration rules require them.
- Move remaining tool-specific configuration into the appropriate blocks in `vite.config.ts`. Review manual follow-up for lint, formatting, packaging, and hooks without discarding project-specific behavior.
- Distinguish built-ins from tasks: `vp dev` and `vp test` run built-in tools; `vp run dev` and `vp run test` run the corresponding project scripts or tasks. The `packageManager` field selects the package manager used by `vp install`, `vp add`, and `vp remove`.

Run `vp install`, `vp check`, and `vp test`, plus configured browser, coverage, and benchmark suites. Run `vp build` for applications and `vp pack` for libraries, including both where the workspace contains both. Check library consumers against emitted imports and declarations. Without a global CLI, install with the project's package manager and invoke the updated local CLI through it, such as `pnpm exec vp check` or `npm exec -- vp check`. Fix failures without weakening assertions or dropping test coverage.

After establishing a passing baseline, try to remove the generated "Vitest v4 compatibility" settings with no code changes or small, localized fixes:

1. Use the migration diff and generated comments to identify additions in root, workspace, and inline project configs. Read each linked explanation and check the effective setting after removal, including inherited values. Preserve pre-existing user settings and settings whose origin is unclear.
2. Remove one added setting at a time and first run the affected projects and suites without code changes. If needed, make small, localized application, test, or setup fixes that preserve test intent, such as correcting a locator or adjusting mock setup in a few tests. Do not weaken assertions, accept snapshot changes without review, or reduce the test set. For `fakeTimers.toNotFake`, remove only the added Temporal entry and preserve other exclusions. Do not weaken coverage enforcement: retain glob-threshold `perFile: true` unless aggregate checking is approved, even if coverage passes.
3. Keep a removal only when the affected tests pass and still execute the same tests without new skips. Remove that setting's generated comment too. If removal requires widespread test edits or shared setup refactoring, keep compatibility for now and report the follow-up work. Restore the setting and its comment if validation still fails, cannot run, or leaves uncertainty about behavior. Undo only cleanup-specific trial edits; preserve completed migration fixes and unrelated work.
4. Run the full validation commands again with the accepted removals together. Report each candidate's config path, removed or retained status, code changes, commands and results, and the reason for retaining it. Distinguish a deferred rewrite from a setting that could not be validated.

For projects that use `vp pack`, also review the generated "tsdown <0.23 compatibility" settings after a passing library build. Use the migration diff and linked comments to distinguish inserted settings from pre-existing user choices. Removing `deps.resolveDepSubpath: true` preserves external subpath imports as written; check that consumers can still resolve the emitted imports. Removing `attw.profile: 'strict'` adopts the esm-only profile, which skips node10 and CommonJS resolution checks. Keep strict if those checks are part of the package's intended support. Do not drop intended declaration checks merely to make a build pass. Remove one setting and its generated comment at a time, then run `vp pack` and the package's consumer checks. Restore the setting and comment if validation fails or the required consumer behavior is unclear. Report which settings remain and why.

Report the migration changes, validation results, retained compatibility settings, and unresolved findings. Do not commit or push unless asked.

These instructions are published at https://viteplus.dev/agent-setup/prompt.md so their contents can be verified at any time.
