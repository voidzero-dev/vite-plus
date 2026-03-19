# RFC: Built-in Pre-commit Hook via `vp config` + `vp staged`

## Summary

Add `vp config` and `vp staged` as built-in commands. `vp config` is a `prepare`-lifecycle command that installs git hook shims (husky-compatible reimplementation, not a bundled dependency). `vp staged` bundles lint-staged and reads config from the `staged` key in `vite.config.ts`. Projects get a zero-config pre-commit hook that runs `vp check --fix` on staged files — no extra devDependencies needed.

## Motivation

Currently, setting up pre-commit hooks in a Vite+ project requires:

1. Installing husky and lint-staged as devDependencies
2. Configuring husky hooks
3. Configuring lint-staged

Pain points:

- **Extra devDependencies** that every project needs
- **Manual setup steps** after `vp create` or `vp migrate`
- **No standardized pre-commit workflow** across Vite+ projects
- husky and lint-staged are universal enough to be built in

By building these capabilities into vite-plus, projects get pre-commit hooks with zero extra devDependencies. Both `vp create` and `vp migrate` set this up automatically.

## User Workflows

There are three distinct entry points, each with a different responsibility:

### New projects: `vp create`

`vp create` scaffolds a new project and optionally sets up the full hooks pipeline:

1. Prompts "Set up pre-commit hooks?" (default: yes)
2. If accepted, calls `installGitHooks()` which:
   - Adds `"prepare": "vp config"` to `package.json`
   - Adds `staged` config to `vite.config.ts`
   - Creates `.vite-hooks/pre-commit` containing `vp staged`
   - Runs `vp config --hooks-only` to install hook shims and set `core.hooksPath`

Flags: `--hooks` (force), `--no-hooks` (skip)

### Existing projects: `vp migrate`

`vp migrate` migrates from husky/lint-staged and sets up the full hooks pipeline:

1. Runs pre-flight checks (husky version, other tools, subdirectory detection)
2. Prompts "Set up pre-commit hooks?" (default: yes)
3. If accepted, after migration rewrite, calls `installGitHooks()` which:
   - Detects old husky directory from `scripts.prepare`
   - Rewrites `"prepare": "husky"` → `"prepare": "vp config"` via `rewritePrepareScript()`
   - Migrates `lint-staged` config from `package.json` to `staged` in `vite.config.ts`
   - Copies `.husky/` hooks to `.vite-hooks/` (or preserves custom dir)
   - Creates `.vite-hooks/pre-commit` containing `vp staged`
   - Runs `vp config --hooks-only` to install hook shims and set `core.hooksPath`
   - Removes husky and lint-staged from `devDependencies`

Flags: `--hooks` (force), `--no-hooks` (skip)

### Ongoing use: `vp config` (prepare lifecycle)

`vp config` is the command that runs on every `npm install` via the `prepare` script. It reinstalls hook shims — it does **not** create the `staged` config or the pre-commit hook file. Those are created by `vp create`/`vp migrate`.

```json
{ "scripts": { "prepare": "vp config" } }
```

When `npm_lifecycle_event=prepare` (set by npm/pnpm/yarn during `npm install`), agent setup is skipped automatically — only hooks are reinstalled.

### Manual setup (without `vp create`/`vp migrate`)

For users who want to set up hooks manually, four steps are required:

1. **Add prepare script** to `package.json`:
   ```json
   { "scripts": { "prepare": "vp config" } }
   ```
2. **Add staged config** to `vite.config.ts`:
   ```typescript
   export default defineConfig({
     staged: { '*': 'vp check --fix' },
   });
   ```
3. **Create pre-commit hook** at `.vite-hooks/pre-commit`:
   ```sh
   vp staged
   ```
4. **Run `vp config`** to install hook shims and set `core.hooksPath`

## Commands

### `vp config`

```bash
vp config                           # Configure project (hooks + agent integration)
vp config -h                        # Show help
vp config --hooks-dir .husky        # Custom hooks directory (default: .vite-hooks)
```

Behavior:

1. Built-in husky-compatible install logic (reimplementation of husky v9, not a bundled dependency)
2. Sets `core.hooksPath` to `<hooks-dir>/_` (default: `.vite-hooks/_`)
3. Creates hook scripts in `<hooks-dir>/_/` that source the user-defined hooks in `<hooks-dir>/`
4. Agent instructions: silently updates existing files that contain Vite+ markers (`<!--VITE PLUS START-->`) when content is outdated. Never creates new agent files. Skipped with `--hooks-only`.
5. Safe to run multiple times (idempotent)
6. Exits 0 and skips hooks if `VITE_GIT_HOOKS=0` or `HUSKY=0` environment variable is set (backwards compatible)
7. Exits 0 and skips hooks if `.git` directory doesn't exist (safe during `npm install` in consumer projects)
8. Exits 1 on real errors (git command not found, `git config` failed)
9. Agent update runs uniformly in all modes (`prepare`, interactive, non-interactive). New agent file creation is handled by `vp create`/`vp migrate`.
10. Interactive mode: prompts on first run for hooks setup
11. Non-interactive mode: sets up hooks by default

### `vp staged`

```bash
vp staged                           # Run staged linters on staged files
```

Behavior:

1. Reads config from `staged` key in `vite.config.ts` via `resolveConfig()`
2. If `staged` key not found, exits with a warning and setup instructions
3. Passes config to bundled lint-staged via its programmatic API
4. Runs configured commands on git-staged files only
5. Exits with non-zero code if any command fails
6. Does not support custom config file paths — config must be in `vite.config.ts`

Both commands are listed under "Core Commands" in `vp -h` (global and local CLI).

## Configuration

### `vite.config.ts`

```typescript
export default defineConfig({
  staged: {
    '*': 'vp check --fix',
  },
});
```

`vp staged` reads config from the `staged` key via Vite's `resolveConfig()`. If no `staged` key is found, it exits with a warning and instructions to add the config. Standalone config files (`.lintstagedrc.*`, `lint-staged.config.*`) are not supported by the migration — projects using those formats are warned to migrate manually.

### `package.json`

```json
// New project
{
  "scripts": {
    "prepare": "vp config"
  }
}
```

```json
// Migrated from husky with custom dir — dir is preserved
{
  "scripts": {
    "prepare": "vp config --hooks-dir .config/husky"
  }
}
```

If the project already has a prepare script, `vp config` is prepended:

```json
{
  "scripts": {
    "prepare": "vp config && npm run build"
  }
}
```

### `.vite-hooks/pre-commit`

```
vp staged
```

### Why `*` glob

`vp check --fix` already handles unsupported file types gracefully (it only processes files that match known extensions). Using `*` simplifies the configuration — no need to maintain a list of extensions.

## Automatic Setup

Both `vp create` and `vp migrate` prompt the user before setting up pre-commit hooks:

- **Interactive mode**: Shows a `prompts.confirm()` prompt: "Set up pre-commit hooks to run formatting, linting, and type checking with auto-fixes?" (default: yes)
- **Non-interactive mode**: Defaults to yes (hooks are set up automatically)
- **`--hooks` flag**: Force hooks setup (no prompt)
- **`--no-hooks` flag**: Skip hooks setup entirely (no prompt)

```bash
vp create --hooks           # Force hooks setup
vp create --no-hooks        # Skip hooks setup
vp migrate --hooks          # Force hooks setup
vp migrate --no-hooks       # Skip hooks setup
```

### `vp create`

- After project creation and migration rewrite, prompts for hooks setup
- If accepted, calls `rewritePrepareScript()` then `setupGitHooks()` — same as `vp migrate`
- `rewritePrepareScript()` rewrites any template-provided `"prepare": "husky"` to `"prepare": "vp config"` before `setupGitHooks()` runs
- Creates `.vite-hooks/pre-commit` with `vp staged`

### `vp migrate`

Migration rewrite (`rewritePackageJson`) uses `vite-tools.yml` rules to rewrite tool commands (vite, oxlint, vitest, etc.) in all scripts. Crucially, the husky rule is **not** in `vite-tools.yml` — it lives in a separate `vite-prepare.yml` and is only applied to `scripts.prepare` via `rewritePrepareScript()`. This ensures husky is never accidentally rewritten in non-prepare scripts.

- Prompts for hooks setup **before** migration rewrite
- If `--no-hooks`: `rewritePrepareScript()` is never called, so the prepare script stays as-is (e.g. `"husky"` remains `"husky"`). No undo logic needed.
- If hooks enabled but Husky v8 detected: warns, sets `shouldSetupHooks = false` and `skipStagedMigration = true` **before** migration rewrite, so lint-staged config is preserved
- If hooks enabled: after migration rewrite, calls `rewritePrepareScript()` then `setupGitHooks()`

Hook setup behavior:

- **No hooks configured** — adds full setup (prepare script + staged config in vite.config.ts + .vite-hooks/pre-commit)
- **Has husky (default dir)** — `rewritePrepareScript()` rewrites `"prepare": "husky"` to `"prepare": "vp config"`, `setupGitHooks()` copies `.husky/` hooks to `.vite-hooks/` and removes husky from devDeps
- **Has husky (custom dir)** — `rewritePrepareScript()` preserves the custom dir as `"vp config --hooks-dir .config/husky"`, `setupGitHooks()` keeps hooks in the custom dir (no copy)
- **Has `husky install`** — `rewritePrepareScript()` collapses `"husky install"` → `"husky"` before applying the ast-grep rule, so `"husky install .hooks"` becomes `"vp config --hooks-dir .hooks"` (custom dir preserved)
- **Has existing prepare script** (e.g. `"npm run build"`) — composes as `"vp config && npm run build"` (prepend so hooks are active before other prepare tasks; idempotent if already contains `vp config`)
- **Has lint-staged** — migrates `"lint-staged"` key to `staged` in vite.config.ts, keeps existing config (already rewritten by migration rules), removes lint-staged from devDeps

## Migration Edge Cases

- **Has husky <9.0.0** — detected **before** migration rewrite. Warns "please upgrade to husky v9+ first", skips hooks setup, and also skips lint-staged migration (`skipStagedMigration` flag). This preserves the `lint-staged` config in package.json and standalone config files, since `.husky/pre-commit` still references `npx lint-staged`.
- **Has other tool (simple-git-hooks, lefthook, yorkie)** — warns and skips
- **Subdirectory project** (e.g. `vp migrate foo`) — if the project path differs from the git root, warns "Subdirectory project detected" and skips hooks setup entirely. This prevents `vp config` from setting `core.hooksPath` to a subdirectory path, which would hijack the repo-wide hooks.
- **No .git directory** — adds package.json config and creates hook pre-commit file, but skips `vp config` hook install (no `core.hooksPath` to set)
- **Standalone lint-staged config** (`.lintstagedrc.*`, `lint-staged.config.*`) — not supported by auto-migration. Projects using those formats are warned to migrate manually.
- After creating the pre-commit hook, runs `vp config` directly to install hook shims (does not rely on npm install lifecycle, which may not run in CI or snap test contexts)

## Implementation Architecture

### Rust Global CLI

Both commands follow Category B (JS Script Commands) pattern — same as `vp create` and `vp migrate`:

```rust
// crates/vite_global_cli/src/commands/config.rs
pub async fn execute(cwd: AbsolutePathBuf, args: &[String]) -> Result<ExitStatus, Error> {
    super::delegate::execute(cwd, "config", args).await
}

// crates/vite_global_cli/src/commands/staged.rs
pub async fn execute(cwd: AbsolutePathBuf, args: &[String]) -> Result<ExitStatus, Error> {
    super::delegate::execute(cwd, "staged", args).await
}
```

### JavaScript Side

Entry points bundled by rolldown into `dist/global/`:

- `src/config/bin.ts` — unified configuration: hooks setup (husky-compatible) + agent integration
- `src/staged/bin.ts` — imports lint-staged programmatic API, reads `staged` config from vite.config.ts
- `src/migration/bin.ts` — migration flow, calls `rewritePrepareScript()` + `setupGitHooks()`

### AST-grep Rules

- `rules/vite-tools.yml` — rewrites tool commands (vite, oxlint, vitest, lint-staged, tsdown) in **all** scripts
- `rules/vite-prepare.yml` — rewrites `husky` → `vp config`, applied **only** to `scripts.prepare` via `rewritePrepareScript()`

The separation ensures the husky rule is never applied to non-prepare scripts (e.g. a hypothetical `"postinstall": "husky something"` won't be touched). The `husky install` → `husky` collapsing (needed because ast-grep can't match multi-word commands in bash) is done in TypeScript before applying the rule. After the AST-grep rewrite, post-processing handles the dir arg: custom dirs get `--hooks-dir` flags, while the default `.husky` dir is dropped (hooks are copied to `.vite-hooks/` instead).

### Build

lint-staged is a devDependency of the `vite-plus` package, bundled by rolldown at build time into `dist/global/`. husky is not a dependency — `vp config` is a standalone reimplementation of husky v9's install logic.

### Why husky cannot be bundled

husky v9's `install()` function uses `new URL('husky', import.meta.url)` to resolve and `copyFileSync` its shell script (the hook dispatcher) relative to its own source location. When bundled by rolldown, `import.meta.url` points to the bundled output directory, not the original `node_modules/husky/` directory, so the shell script file cannot be found at runtime. Rather than working around this with asset copying hacks, `vp config` inlines the equivalent shell script as a string constant and writes it directly via `writeFileSync`.

Husky <9.0.0 is not supported by auto migration — `vp migrate` detects unsupported versions and skips hooks setup with a warning.

## Relationship to Existing Commands

| Command          | Purpose                                | When                        |
| ---------------- | -------------------------------------- | --------------------------- |
| `vp check`       | Format + lint + type check             | Manual or CI                |
| `vp check --fix` | Auto-fix format + lint issues          | Manual or pre-commit        |
| **`vp config`**  | **Reinstall hook shims + agent setup** | **npm `prepare` lifecycle** |
| **`vp staged`**  | **Run staged linters on staged files** | **Pre-commit hook**         |

## Comparison with Other Tools

| Tool                      | Approach                                   |
| ------------------------- | ------------------------------------------ |
| husky + lint-staged       | Separate devDependencies, manual setup     |
| simple-git-hooks          | Lightweight alternative to husky           |
| lefthook                  | Go binary, config-file based               |
| **vp config + vp staged** | **Built-in, zero-config, automatic setup** |
