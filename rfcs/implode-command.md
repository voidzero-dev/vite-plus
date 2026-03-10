# RFC: Implode (Self-Uninstall) Command

## Status

Implemented

## Background

Vite+ currently has no built-in way to uninstall itself. Users must manually delete `~/.vite-plus/` and hunt through shell profiles (`.zshrc`, `.bashrc`, `.profile`, `config.fish`, etc.) to remove the sourcing lines added by `install.sh`. This is error-prone and leaves artifacts behind.

A native `vp implode` command cleanly removes all Vite+ artifacts from the system in a single step.

### What the Install Script Writes

The `install.sh` script adds the following block to shell profiles:

```
<blank line>
# Vite+ bin (https://viteplus.dev)
. "$HOME/.vite-plus/env"
```

For fish shell:

```
<blank line>
# Vite+ bin (https://viteplus.dev)
source "$HOME/.vite-plus/env.fish"
```

On Windows, `install.ps1` adds `~/.vite-plus/bin` to the User PATH environment variable.

## Goals

1. Provide a single command to completely remove Vite+ from the system
2. Clean up shell profiles (remove sourcing lines and associated comments)
3. Remove the `~/.vite-plus/` directory and all its contents
4. Handle Windows-specific cleanup (User PATH, locked binary)
5. Require explicit confirmation to prevent accidental uninstalls

## Non-Goals

1. Selective removal (e.g., keeping downloaded Node.js versions)
2. Backup before removal
3. Removing project-level `vite-plus` npm dependencies

## User Stories

### Story 1: Interactive Uninstall

```bash
$ vp implode
warn: This will completely remove vite-plus from your system!

  Directory: /home/user/.vite-plus
  Shell profiles to clean:
    - ~/.zshenv
    - ~/.bashrc

Type uninstall to confirm:
uninstall
✓ Cleaned ~/.zshenv
✓ Cleaned ~/.bashrc
✓ Removed /home/user/.vite-plus

✓ vite-plus has been removed from your system.
note: Restart your terminal to apply shell changes.
```

### Story 2: Non-Interactive Uninstall (CI)

```bash
$ vp implode --yes
✓ Cleaned ~/.zshenv
✓ Cleaned ~/.bashrc
✓ Removed /home/user/.vite-plus

✓ vite-plus has been removed from your system.
note: Restart your terminal to apply shell changes.
```

### Story 3: Not Installed

```bash
$ vp implode --yes
info: vite-plus is not installed (directory does not exist)
```

### Story 4: Non-TTY Without --yes

```bash
$ echo "" | vp implode
Cannot prompt for confirmation: stdin is not a TTY. Use --yes to skip confirmation.
```

## Technical Design

### Command Interface

```
vp implode [OPTIONS]

Options:
  -y, --yes   Skip confirmation prompt
  -h, --help  Print help
```

### Command Name: `implode`

**Decision**: Use `implode` following mise's convention for a self-destruct command.

**Alternatives considered**:

- `self uninstall` / `self remove` — used by rustup (`rustup self uninstall`); requires subcommand group
- `uninstall` — ambiguous with package uninstall operations

**Rationale**:

- Single word, memorable, unambiguous
- Follows mise precedent (`mise implode`)
- Cannot be confused with package management operations

### Implementation Flow

```
┌───────────────────────────────────────────────┐
│                vp implode                     │
├───────────────────────────────────────────────┤
│  1. Resolve ~/.vite-plus via get_vite_plus_home│
│  2. Scan shell profiles for Vite+ lines       │
│  3. Confirmation prompt (unless --yes)        │
│  4. Clean shell profiles                      │
│  5. Remove Windows PATH entry (Windows only)  │
│  6. Remove ~/.vite-plus/ directory            │
│  7. Print success message                     │
└───────────────────────────────────────────────┘
```

#### Step 1: Resolve Home Directory

Use `vite_shared::get_vite_plus_home()` to determine the install directory. If it doesn't exist, print "not installed" and exit 0.

#### Step 2: Scan Shell Profiles

Check the following files for Vite+ sourcing lines:

| Shell | Files                                        |
| ----- | -------------------------------------------- |
| zsh   | `~/.zshenv`, `~/.zshrc`                      |
| bash  | `~/.bash_profile`, `~/.bashrc`, `~/.profile` |
| fish  | `~/.config/fish/config.fish`                 |

**POSIX detection pattern**: Lines containing `.vite-plus/env"` (trailing quote avoids matching `env.fish`).

**Fish detection pattern**: Lines containing `.vite-plus/env.fish`.

#### Step 3: Confirmation

Unless `--yes` is passed:

- If stdin is not a TTY, return an error asking for `--yes`
- Display what will be removed (directory path + affected shell profiles)
- Require the user to type `uninstall` to confirm (similar to `rustup self uninstall`)

#### Step 4: Shell Profile Cleanup

For each affected file, remove:

1. The sourcing line (`. "$HOME/.vite-plus/env"` or `source ... env.fish`)
2. The comment line above it (`# Vite+ bin (https://viteplus.dev)`)
3. The blank line before the comment (added by the install script)

Shell profile cleanup is non-fatal — if a file can't be written, a warning is printed and the process continues.

#### Step 5: Windows PATH Cleanup

On Windows, run PowerShell to remove `.vite-plus\bin` from the User PATH environment variable:

```powershell
[Environment]::SetEnvironmentVariable('Path',
  ([Environment]::GetEnvironmentVariable('Path', 'User') -split ';' |
  Where-Object { $_ -ne '<bin_path>' }) -join ';', 'User')
```

#### Step 6: Remove Directory

**Unix**: `std::fs::remove_dir_all` works even while the binary is running (Unix doesn't delete open files until all file descriptors are closed).

**Windows**: The running `vp.exe` is always locked by the OS. Strategy:

1. Rename `~/.vite-plus` to `~/.vite-plus.removing-<pid>` so the original path is immediately free for reinstall
2. Spawn a detached `cmd.exe` process that retries `rmdir /S /Q` up to 10 times with 1-second pauses (via `timeout /T 1 /NOBREAK`), exiting as soon as the directory is gone

### File Structure

```
crates/vite_global_cli/
├── src/
│   ├── commands/
│   │   ├── implode.rs        # Full implementation
│   │   ├── mod.rs            # Add implode module
│   │   └── ...
│   └── cli.rs                # Add Implode command variant
```

### Error Handling

| Error                        | Behavior                      |
| ---------------------------- | ----------------------------- |
| Home dir not found           | Print "not installed", exit 0 |
| Home dir doesn't exist       | Print "not installed", exit 0 |
| Can't determine user home    | Return error                  |
| Shell profile write failure  | Warn and continue             |
| Windows PATH cleanup failure | Warn and continue             |
| Directory removal failure    | Return error                  |
| Non-TTY without --yes        | Return error with suggestion  |

## Testing Strategy

### Unit Tests

- `test_remove_vite_plus_lines_posix` — strips comment + sourcing from mock `.zshrc`
- `test_remove_vite_plus_lines_fish` — strips fish `source` syntax
- `test_remove_vite_plus_lines_no_match` — no modification when no Vite+ lines present
- `test_remove_vite_plus_lines_absolute_path` — handles `/home/user/.vite-plus/env` variant
- `test_remove_vite_plus_lines_preserves_surrounding` — other content untouched
- `test_clean_shell_profile_integration` — tempdir-based integration test
- `test_execute_not_installed` — points `VITE_PLUS_HOME` at non-existent path, verifies success

### CI Tests

Implode tests run in `.github/workflows/ci.yml` alongside the upgrade tests, across all platforms (bash on all, powershell and cmd on Windows):

1. Run `vp implode --yes`
2. Verify `~/.vite-plus/` is removed
3. Reinstall via `pnpm bootstrap-cli:ci`
4. Verify reinstallation works (`vp --version`)

### Manual Testing

```bash
# Build and install
pnpm bootstrap-cli

# Test interactive confirmation (cancel)
vp implode

# Test full uninstall
vp implode --yes

# Verify cleanup
ls ~/.vite-plus      # should not exist
grep vite-plus ~/.zshenv ~/.zshrc ~/.bashrc  # should find nothing

# Verify vp is gone
which vp             # should not be found (after terminal restart)
```

## References

- [RFC: Upgrade Command](./upgrade-command.md)
- [RFC: Global CLI (Rust Binary)](./global-cli-rust-binary.md)
- [Install Script](../packages/cli/install.sh)
- [Install Script (Windows)](../packages/cli/install.ps1)
