const localMigrationPrompt = `Without a global installation, run the target CLI through the package manager from the workspace root. For the 1.0.0 release, use \`pnpm dlx --package=vite-plus@1.0.0 vp migrate --no-interactive\` or \`npx --package=vite-plus@1.0.0 vp migrate --no-interactive\`. First run the same command with \`help migrate\` instead of \`migrate --no-interactive\` to read its help. These commands fetch the target CLI without replacing the old project dependencies first.`;

const compatibilityReviewPrompt = `After establishing a passing baseline, try to remove the generated "Vitest v4 compatibility" settings with no code changes or small, localized fixes:

1. Use the migration diff and generated comments to identify additions in root, workspace, and inline project configs. Read each linked explanation and check the effective setting after removal, including inherited values. Preserve pre-existing user settings and settings whose origin is unclear.
2. Remove one added setting at a time and first run the affected projects and suites without code changes. If needed, make small, localized application, test, or setup fixes that preserve test intent, such as correcting a locator or adjusting mock setup in a few tests. Do not weaken assertions, accept snapshot changes without review, or reduce the test set. For fakeTimers.toNotFake, remove only the added Temporal entry and preserve other exclusions. Do not weaken coverage enforcement: retain glob-threshold perFile: true unless I approve aggregate checking, even if coverage passes.
3. Keep a removal only when the affected tests pass and still execute the same tests without new skips. Remove that setting's generated comment too. If removal requires widespread test edits or shared setup refactoring, keep compatibility for now and report the follow-up work. Restore the setting and its comment if validation still fails, cannot run, or leaves uncertainty about behavior. Undo only cleanup-specific trial edits; preserve completed migration fixes and unrelated work.
4. Run the full validation commands again with the accepted removals together. Report each candidate's config path, removed or retained status, code changes, commands and results, and the reason for retaining it. Distinguish a deferred rewrite from a setting you could not validate.

For projects that use vp pack, also review the generated "tsdown <0.23 compatibility" settings after a passing library build. Use the migration diff and linked comments to distinguish inserted settings from pre-existing user choices. Removing deps.resolveDepSubpath: true preserves external subpath imports as written; check that consumers can still resolve the emitted imports. Removing attw.profile: 'strict' adopts the esm-only profile, which skips node10 and CommonJS resolution checks. Keep strict if those checks are part of the package's intended support. Do not drop intended declaration checks merely to make a build pass. Remove one setting and its generated comment at a time, then run vp pack and the package's consumer checks. Restore the setting and comment if validation fails or the required consumer behavior is unclear. Report which settings remain and why.`;

export const migrationPrompt = `Migrate this project to Vite+ while preserving its application, test, and library build behavior.

Read these guides before making changes:

- ${__DOCS_ORIGIN__}/guide/migrate
- ${__DOCS_ORIGIN__}/guide/migrate-rules
- ${__DOCS_ORIGIN__}/guide/vitest-v5
- https://vitest.dev/guide/migration/
- https://github.com/rolldown/tsdown/releases/tag/v0.23.0

Inspect the worktree and preserve unrelated changes. Identify the workspace root, package manager, scripts, configuration files, and tools in use. If the project already uses Vite+, follow the upgrade flow in the migration guide and keep the existing setup; do not use --full unless I request it.

For a project that does not use Vite+ yet, check the prerequisites for the tools it uses: Vite 8+ and Vitest 4.1+. Complete any required upstream upgrades and validate them before starting the Vite+ migration. Then keep those manifests, lockfile, and installed packages available so the migrator can identify the original Vitest version. Do not install vite-plus or upgrade Vitest to the target's bundled version before running migration.

Use the CLI from the target Vite+ 1.0 release or its preview build. Use a supported Node.js runtime from the compatibility guide. A global installation is optional:

- With a global vp installation, follow ${__DOCS_ORIGIN__}/guide/upgrade to select the target release and check \`vp toolchain --global\`. Run \`vp help\` and \`vp help migrate\`, then \`vp migrate --no-interactive\` from the workspace root.
- ${localMigrationPrompt}

Replace 1.0.0 with the intended release version. For a preview, use the version from its PR and pass \`--registry=https://registry-bridge.viteplus.dev\` to pnpm or npx before the vp command.

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

export const upgradePrompt = `Upgrade this project from Vite+ 0.3.x to Vite+ 1.0 while preserving its test and library build behavior.

Read these guides before making changes:

- ${__DOCS_ORIGIN__}/guide/migrate
- ${__DOCS_ORIGIN__}/guide/vitest-v5
- https://vitest.dev/guide/migration/
- ${__DOCS_ORIGIN__}/guide/migrate-rules#pack-configuration
- https://github.com/rolldown/tsdown/releases/tag/v0.23.0

Inspect the worktree and preserve unrelated changes. Keep the original manifests, lockfile, and installed packages available so migration can identify the original Vitest version. Do not update the project's vite-plus or Vitest dependencies before running migration.

Use the CLI from the target Vite+ 1.0 release or its preview build. A global installation is optional. Use a supported Node.js runtime from the Vite+ compatibility guide.

- With a global vp installation, follow ${__DOCS_ORIGIN__}/guide/upgrade to upgrade it and check \`vp toolchain --global\`. Run \`vp help migrate\`, then \`vp migrate --no-interactive\` from the workspace root.
- ${localMigrationPrompt}

Replace 1.0.0 with the intended release version. For a preview, use the version from its PR and pass \`--registry=https://registry-bridge.viteplus.dev\` to pnpm or npx before the vp command. Do not run migration with the old project's node_modules/.bin/vp. Keep the existing project setup; do not use --full unless I request it.

Resolve BLOCK findings and rerun migration. Review each REVIEW finding using its documentation link, even if migration exits with success. Preserve test intent and keep the generated Vitest v4 and tsdown <0.23 compatibility settings and comments for the first validation run.

Check workspace manifests, catalogs, overrides, and import changes against the Vite+ guide. Keep test APIs on supported vite-plus/test entries; use @vitest/browser-webdriverio for the community WebDriverIO provider.

Run \`vp install\`, \`vp check\`, and \`vp test\`, plus the project's browser, coverage, and benchmark suites where configured. Run \`vp build\` or \`vp pack\` as appropriate. Without a global CLI, finish installation with the project's package manager, then invoke the updated local CLI through it, such as \`pnpm exec vp check\` or \`npm exec -- vp check\`. Fix migration failures without weakening assertions or dropping test coverage.

${compatibilityReviewPrompt}

Report the migration changes and unresolved findings as well. Do not commit or push unless I ask.`;

// Shared by the homepage and Getting Started guide.
export const setupPrompt = `Fetch and execute the appropriate instructions to set me up for Vite+ from ${__DOCS_ORIGIN__}/agent-setup/prompt.md`;
