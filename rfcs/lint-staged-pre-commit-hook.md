# RFC: Built-in Pre-commit Hook via `vp prepare` + `vp lint-staged`

## Summary

Add `vp prepare` and `vp lint-staged` as built-in commands that bundle husky and lint-staged functionality. Projects get a zero-config pre-commit hook that runs `vp check --fix` on staged files — no extra devDependencies needed.

## Motivation

Currently, setting up pre-commit hooks in a Vite+ project requires:

1. Installing husky and lint-staged as devDependencies
2. Configuring husky hooks
3. Configuring lint-staged

Pain points:

- **Extra devDependencies** that every project needs
- **Manual setup steps** after `vp create` or `vp migrate`
- **No standardized pre-commit workflow** across Vite+ projects
- husky and lint-staged are universal enough to be bundled

By bundling these tools inside vite-plus, projects get pre-commit hooks with zero extra devDependencies. Both `vp create` and `vp migrate` set this up automatically.

## Command Syntax

```bash
# Set up git hooks (runs bundled husky install)
vp prepare

# Run lint-staged on staged files (runs bundled lint-staged)
vp lint-staged
```

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

### .husky/pre-commit

```
vp lint-staged
```

### Why `*` glob

`vp check --fix` already handles unsupported file types gracefully (it only processes files that match known extensions). Using `*` simplifies the configuration — no need to maintain a list of extensions.

## Behavior

### `vp prepare`

1. Delegates to bundled husky install logic
2. Sets `core.hooksPath` to `.husky/_`
3. Creates hook scripts in `.husky/_/` that source the user-defined hooks in `.husky/`
4. Safe to run multiple times (idempotent)
5. Skips if `HUSKY=0` environment variable is set
6. Skips if `.git` directory doesn't exist

### `vp lint-staged`

1. Delegates to bundled lint-staged
2. Reads lint-staged config from package.json `lint-staged` field (or standalone config files)
3. Runs configured commands on git-staged files only
4. Exits with non-zero code if any command fails

### Automatic Setup

#### `vp create`

- Monorepo template includes `"prepare": "vp prepare"` and `"lint-staged"` config
- After `git init`, creates `.husky/pre-commit` with `vp lint-staged`

#### `vp migrate`

- **No hooks configured** — adds full setup (prepare script + lint-staged config + .husky/pre-commit)
- **Has husky** — rewrites `"prepare": "husky"` to `"prepare": "vp prepare"`, removes husky from devDeps
- **Has lint-staged** — keeps existing config (already rewritten by migration rules), removes from devDeps
- **Has other tool (simple-git-hooks, lefthook, yorkie)** — warns and skips
- **No .git directory** — adds package.json config but doesn't create .husky/ directory

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

- `src/prepare/bin.ts` — imports husky and runs install
- `src/lint-staged/bin.ts` — imports lint-staged CLI entry

### Build

Both husky and lint-staged are devDependencies of the `vite-plus` package, bundled by rolldown at build time. They are NOT externalized — the bundled output includes all their code, so users don't need these packages installed.

## Relationship to Existing Commands

| Command              | Purpose                             | When                        |
| -------------------- | ----------------------------------- | --------------------------- |
| `vp check`           | Format + lint + type check          | Manual or CI                |
| `vp check --fix`     | Auto-fix format + lint issues       | Manual or pre-commit        |
| **`vp prepare`**     | **Set up git hooks**                | **npm `prepare` lifecycle** |
| **`vp lint-staged`** | **Run lint-staged on staged files** | **Pre-commit hook**         |

## Comparison with Other Tools

| Tool                            | Approach                                  |
| ------------------------------- | ----------------------------------------- |
| husky + lint-staged             | Separate devDependencies, manual setup    |
| simple-git-hooks                | Lightweight alternative to husky          |
| lefthook                        | Go binary, config-file based              |
| **vp prepare + vp lint-staged** | **Bundled, zero-config, automatic setup** |
