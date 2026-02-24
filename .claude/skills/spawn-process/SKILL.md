---
name: spawn-process
description: Guide for writing subprocess execution code using the vite_command crate
allowed-tools: Read, Grep, Glob, Edit, Write, Bash
---

# Add Subprocess Execution Code

When writing Rust code that needs to spawn subprocesses (resolve binaries, build commands, execute programs), always use the `vite_command` crate. Never use `which`, `tokio::process::Command::new`, or `std::process::Command::new` directly.

## Available APIs

### `vite_command::resolve_bin(name, path_env, cwd)` — Resolve a binary name to an absolute path

Handles PATHEXT (`.cmd`/`.bat`) on Windows. Pass `None` for `path_env` to search the current process PATH.

```rust
// Resolve using current PATH
let bin = vite_command::resolve_bin("node", None, &cwd)?;

// Resolve using a custom PATH
let custom_path = std::ffi::OsString::from(&path_env_str);
let bin = vite_command::resolve_bin("eslint", Some(&custom_path), &cwd)?;
```

### `vite_command::build_command(bin_path, cwd)` — Build a command for a pre-resolved binary

Returns `tokio::process::Command` with cwd, inherited stdio, and `fix_stdio_streams` on Unix already configured. Add args, envs, or override stdio as needed.

```rust
let bin = vite_command::resolve_bin("eslint", None, &cwd)?;
let mut cmd = vite_command::build_command(&bin, &cwd);
cmd.args(&[".", "--fix"]);
cmd.env("NODE_ENV", "production");
let mut child = cmd.spawn()?;
let status = child.wait().await?;
```

### `vite_command::build_shell_command(shell_cmd, cwd)` — Build a shell command

Uses `/bin/sh -c` on Unix, `cmd.exe /C` on Windows. Same stdio and `fix_stdio_streams` setup as `build_command`.

```rust
let mut cmd = vite_command::build_shell_command("echo hello && ls", &cwd);
let mut child = cmd.spawn()?;
let status = child.wait().await?;
```

### `vite_command::run_command(bin_name, args, envs, cwd)` — Resolve + build + run in one call

Combines resolve_bin, build_command, and status().await. The `envs` HashMap must include `"PATH"` if you want custom PATH resolution.

```rust
let envs = HashMap::from([("PATH".to_string(), path_value)]);
let status = vite_command::run_command("node", &["--version"], &envs, &cwd).await?;
```

## Dependency Setup

Add `vite_command` to the crate's `Cargo.toml`:

```toml
[dependencies]
vite_command = { workspace = true }
```

Do NOT add `which` as a direct dependency — binary resolution goes through `vite_command::resolve_bin`.

## Exception

`crates/vite_global_cli/src/shim/exec.rs` uses synchronous `std::process::Command` with Unix `exec()` for process replacement. This is the only place that bypasses `vite_command`.
