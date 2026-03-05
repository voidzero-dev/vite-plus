# RFC: Built-in Pre-commit Hook via `vp config` + `vp staged`

## Summary

Add `vp config` and `vp staged` as built-in commands. `vp config` is a unified configuration command that sets up git hooks (husky-compatible reimplementation, not a bundled dependency) and agent integration. `vp staged` bundles lint-staged and reads config from the `staged` key in `vite.config.ts`. Projects get a zero-config pre-commit hook that runs `vp check --fix` on staged files — no extra devDependencies needed.

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

## Command Syntax

```bash
# Configure project (hooks + agent integration)
vp config
vp config -h                        # Show help
vp config --hooks-dir .husky        # Custom hooks directory (default: .vite-hooks)

# Run staged linters on staged files (runs bundled lint-staged with staged config)
vp staged

# Control hooks setup during create/migrate
vp create --hooks           # Force hooks setup
vp create --no-hooks        # Skip hooks setup
vp migrate --hooks          # Force hooks setup
vp migrate --no-hooks       # Skip hooks setup
```

Both commands are listed under "Core Commands" in `vp -h` (global and local CLI).

## User-Facing Configuration

### vite.config.ts + package.json (zero extra devDependencies)

```typescript
// vite.config.ts
export default defineConfig({
  staged: {
    '*': 'vp check --fix',
  },
});
```

```json
// package.json (new project)
{
  "scripts": {
    "prepare": "vp config"
  }
}
```

```json
// package.json (migrated from husky — default .husky dir migrates to .vite-hooks)
{
  "scripts": {
    "prepare": "vp config"
  }
}
```

```json
// package.json (migrated from husky with custom dir — dir is preserved)
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

### .vite-hooks/pre-commit (or custom dir for projects with non-default husky dir)

```
vp staged
```

### Why `*` glob

`vp check --fix` already handles unsupported file types gracefully (it only processes files that match known extensions). Using `*` simplifies the configuration — no need to maintain a list of extensions.

### Config Discovery

`vp staged` reads config from the `staged` key in `vite.config.ts` via Vite's `resolveConfig()`. If no `staged` key is found, it exits with a warning and instructions to add the config. Standalone config files (`.lintstagedrc.*`, `lint-staged.config.*`) are not supported by the migration — projects using those formats are warned to migrate manually.

## Behavior

### `vp config`

1. Built-in husky-compatible install logic (reimplementation of husky v9, not a bundled dependency)
2. Sets `core.hooksPath` to `<hooks-dir>/_` (default: `.vite-hooks/_`)
3. Creates hook scripts in `<hooks-dir>/_/` that source the user-defined hooks in `<hooks-dir>/`
4. Agent integration: injects agent instructions and MCP config (skipped during `prepare` lifecycle — see point 11)
5. Safe to run multiple times (idempotent)
6. Exits 0 and skips hooks if `VITE_GIT_HOOKS=0` or `HUSKY=0` environment variable is set (backwards compatible)
7. Exits 0 and skips hooks if `.git` directory doesn't exist (safe during `npm install` in consumer projects)
8. Exits 1 on real errors (git command not found, `git config` failed)
9. Interactive mode: prompts on first run for hooks and agent setup; updates silently on subsequent runs
10. Non-interactive mode: runs everything by default
11. `prepare` lifecycle detection: when `npm_lifecycle_event=prepare` (set by npm/pnpm/yarn during `npm install`), agent setup is skipped automatically. This ensures `"prepare": "vp config"` only installs hooks during install — agent setup is a one-time operation handled by `vp create`/`vp migrate`, not repeated on every `npm install`

### `vp staged`

1. Reads config from `staged` key in `vite.config.ts` via `resolveConfig()`
2. If `staged` key not found, exits with a warning and setup instructions
3. Passes config to bundled lint-staged via its programmatic API
4. Runs configured commands on git-staged files only
5. Exits with non-zero code if any command fails
6. Does not support custom config file paths — config must be in vite.config.ts

### Automatic Setup

Both `vp create` and `vp migrate` prompt the user before setting up pre-commit hooks:

- **Interactive mode**: Shows a `prompts.confirm()` prompt: "Set up pre-commit hooks to run formatting, linting, and type checking with auto-fixes?" (default: yes)
- **Non-interactive mode**: Defaults to yes (hooks are set up automatically)
- **`--hooks` flag**: Force hooks setup (no prompt)
- **`--no-hooks` flag**: Skip hooks setup entirely (no prompt)

#### `vp create`

- After project creation and migration rewrite, prompts for hooks setup
- If accepted, calls `rewritePrepareScript()` then `setupGitHooks()` — same as `vp migrate`
- `rewritePrepareScript()` rewrites any template-provided `"prepare": "husky"` to `"prepare": "vp config"` before `setupGitHooks()` runs
- Creates `.vite-hooks/pre-commit` with `vp staged`

#### `vp migrate`

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
- **Has husky <9.0.0** — detected **before** migration rewrite. Warns "please upgrade to husky v9+ first", skips hooks setup, and also skips lint-staged migration (`skipStagedMigration` flag). This preserves the `lint-staged` config in package.json and standalone config files, since `.husky/pre-commit` still references `npx lint-staged`.
- **Has other tool (simple-git-hooks, lefthook, yorkie)** — warns and skips
- **Subdirectory project** (e.g. `vp migrate foo`) — if the project path differs from the git root, warns "Subdirectory project detected" and skips hooks setup entirely. This prevents `vp config` from setting `core.hooksPath` to a subdirectory path, which would hijack the repo-wide hooks.
- **No .git directory** — adds package.json config and creates hook pre-commit file, but skips `vp config` hook install (no `core.hooksPath` to set)
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
| **`vp config`**  | **Configure project (hooks + agent)**  | **npm `prepare` lifecycle** |
| **`vp staged`**  | **Run staged linters on staged files** | **Pre-commit hook**         |

## Comparison with Other Tools

| Tool                      | Approach                                   |
| ------------------------- | ------------------------------------------ |
| husky + lint-staged       | Separate devDependencies, manual setup     |
| simple-git-hooks          | Lightweight alternative to husky           |
| lefthook                  | Go binary, config-file based               |
| **vp config + vp staged** | **Built-in, zero-config, automatic setup** |
