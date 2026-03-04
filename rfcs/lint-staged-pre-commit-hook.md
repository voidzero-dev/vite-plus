# RFC: Built-in Pre-commit Hook via `vp prepare` + `vp lint-staged`

## Summary

Add `vp prepare` and `vp lint-staged` as built-in commands. `vp prepare` is a husky-compatible reimplementation (husky itself is not a dependency), and `vp lint-staged` bundles lint-staged. Projects get a zero-config pre-commit hook that runs `vp check --fix` on staged files — no extra devDependencies needed.

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
# Set up git hooks (built-in husky-compatible implementation)
vp prepare
vp prepare -h               # Show help

# Run lint-staged on staged files (runs bundled lint-staged)
vp lint-staged

# Control hooks setup during create/migrate
vp create --hooks           # Force hooks setup
vp create --no-hooks        # Skip hooks setup
vp migrate --hooks          # Force hooks setup
vp migrate --no-hooks       # Skip hooks setup
```

Both commands are listed under "Core Commands" in `vp -h` (global and local CLI).

## User-Facing Configuration

### package.json (zero extra devDependencies)

```json
{
  "scripts": {
    "prepare": "vp prepare"
  },
  "lint-staged": {
    "*": "vp check --fix"
  }
}
```

If the project already has a prepare script, `vp prepare` is prepended:

```json
{
  "scripts": {
    "prepare": "vp prepare && npm run build"
  }
}
```

### .husky/pre-commit

```
vp lint-staged
```

### Why `*` glob

`vp check --fix` already handles unsupported file types gracefully (it only processes files that match known extensions). Using `*` simplifies the configuration — no need to maintain a list of extensions.

## Behavior

### `vp prepare`

1. Built-in husky-compatible install logic (reimplementation of husky v9, not a bundled dependency)
2. Sets `core.hooksPath` to `.husky/_`
3. Creates hook scripts in `.husky/_/` that source the user-defined hooks in `.husky/`
4. Safe to run multiple times (idempotent)
5. Exits 0 and skips if `HUSKY=0` environment variable is set
6. Exits 0 and skips if `.git` directory doesn't exist (safe during `npm install` in consumer projects)
7. Exits 1 on real errors (git command not found, `git config` failed)

### `vp lint-staged`

1. Delegates to bundled lint-staged
2. Reads lint-staged config from package.json `lint-staged` field (or standalone config files)
3. Runs configured commands on git-staged files only
4. Exits with non-zero code if any command fails

### Automatic Setup

Both `vp create` and `vp migrate` prompt the user before setting up pre-commit hooks:

- **Interactive mode**: Shows a `prompts.confirm()` prompt: "Set up pre-commit hooks to run format, lint, and type checks with auto-fix?" (default: yes)
- **Non-interactive mode**: Defaults to yes (hooks are set up automatically)
- **`--hooks` flag**: Force hooks setup (no prompt)
- **`--no-hooks` flag**: Skip hooks setup entirely (no prompt)

#### `vp create`

- After project creation and migration rewrite, prompts for hooks setup
- If accepted, adds `"prepare": "vp prepare"` and `"lint-staged"` config to package.json
- Creates `.husky/pre-commit` with `vp lint-staged` (if `.git` directory exists)

#### `vp migrate`

Migration rewrite (`rewritePackageJson`) uses `vite-tools.yml` rules to rewrite tool commands (vite, oxlint, vitest, etc.) in all scripts. Crucially, the husky rule is **not** in `vite-tools.yml` — it lives in a separate `vite-prepare.yml` and is only applied to `scripts.prepare` via `rewritePrepareScript()`. This ensures husky is never accidentally rewritten in non-prepare scripts.

- Prompts for hooks setup **before** migration rewrite
- If `--no-hooks`: `rewritePrepareScript()` is never called, so the prepare script stays as-is (e.g. `"husky"` remains `"husky"`). No undo logic needed.
- If hooks enabled: after migration rewrite, calls `rewritePrepareScript()` then `setupGitHooks()`

Hook setup behavior:

- **No hooks configured** — adds full setup (prepare script + lint-staged config + .husky/pre-commit)
- **Has husky** — `rewritePrepareScript()` rewrites `"prepare": "husky"` to `"prepare": "vp prepare"`, `setupGitHooks()` removes husky from devDeps
- **Has `husky install`** — `rewritePrepareScript()` collapses `"husky install"` → `"husky"` before applying the ast-grep rule, so `"husky install .hooks"` becomes `"vp prepare .hooks"`
- **Has existing prepare script** (e.g. `"npm run build"`) — composes as `"vp prepare && npm run build"` (prepend so hooks are active before other prepare tasks; idempotent if already contains `vp prepare`)
- **Has lint-staged** — keeps existing config (already rewritten by migration rules), removes from devDeps
- **Has husky <9.0.0** — warns "please upgrade to husky v9+ first" and skips hooks setup entirely. The prepare script, devDependencies, and hook files are left untouched.
- **Has other tool (simple-git-hooks, lefthook, yorkie)** — warns and skips
- **No .git directory** — adds package.json config but doesn't create .husky/ directory
- After creating `.husky/pre-commit`, runs `vp prepare` directly to install hook shims (does not rely on npm install lifecycle, which may not run in CI or snap test contexts)

## Implementation Architecture

### Rust Global CLI

Both commands follow Category B (JS Script Commands) pattern — same as `vp create` and `vp migrate`:

```rust
// crates/vite_global_cli/src/commands/prepare.rs
pub async fn execute(cwd: AbsolutePathBuf, args: &[String]) -> Result<ExitStatus, Error> {
    super::delegate::execute(cwd, "prepare", args).await
}

// crates/vite_global_cli/src/commands/lint_staged.rs
pub async fn execute(cwd: AbsolutePathBuf, args: &[String]) -> Result<ExitStatus, Error> {
    super::delegate::execute(cwd, "lint-staged", args).await
}
```

### JavaScript Side

Entry points bundled by rolldown into `dist/global/`:

- `src/prepare/bin.ts` — built-in husky-compatible install logic
- `src/lint-staged/bin.ts` — imports lint-staged CLI entry
- `src/migration/bin.ts` — migration flow, calls `rewritePrepareScript()` + `setupGitHooks()`

### AST-grep Rules

- `rules/vite-tools.yml` — rewrites tool commands (vite, oxlint, vitest, lint-staged, tsdown) in **all** scripts
- `rules/vite-prepare.yml` — rewrites `husky` → `vp prepare`, applied **only** to `scripts.prepare` via `rewritePrepareScript()`

The separation ensures the husky rule is never applied to non-prepare scripts (e.g. a hypothetical `"postinstall": "husky something"` won't be touched). The `husky install` → `husky` collapsing (needed because ast-grep can't match multi-word commands in bash) is done in TypeScript before applying the rule.

### Build

lint-staged is a devDependency of the `vite-plus` package, bundled by rolldown at build time into `dist/global/`. husky is not a dependency — `vp prepare` is a standalone reimplementation of husky v9's install logic.

### Why husky cannot be bundled

husky v9's `install()` function uses `new URL('husky', import.meta.url)` to resolve and `copyFileSync` its shell script (the hook dispatcher) relative to its own source location. When bundled by rolldown, `import.meta.url` points to the bundled output directory, not the original `node_modules/husky/` directory, so the shell script file cannot be found at runtime. Rather than working around this with asset copying hacks, `vp prepare` inlines the equivalent shell script as a string constant and writes it directly via `writeFileSync`.

Husky <9.0.0 is not supported by auto migration — `vp migrate` detects unsupported versions and skips hooks setup with a warning.

## Relationship to Existing Commands

| Command              | Purpose                             | When                        |
| -------------------- | ----------------------------------- | --------------------------- |
| `vp check`           | Format + lint + type check          | Manual or CI                |
| `vp check --fix`     | Auto-fix format + lint issues       | Manual or pre-commit        |
| **`vp prepare`**     | **Set up git hooks**                | **npm `prepare` lifecycle** |
| **`vp lint-staged`** | **Run lint-staged on staged files** | **Pre-commit hook**         |

## Comparison with Other Tools

| Tool                            | Approach                                   |
| ------------------------------- | ------------------------------------------ |
| husky + lint-staged             | Separate devDependencies, manual setup     |
| simple-git-hooks                | Lightweight alternative to husky           |
| lefthook                        | Go binary, config-file based               |
| **vp prepare + vp lint-staged** | **Built-in, zero-config, automatic setup** |
