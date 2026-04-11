# RFC: Upgrade Check

## Status

Draft

## Background

Vite+ has a `vp upgrade` command for self-updating, but users only discover new versions if they manually run `vp upgrade --check` or hear about it externally. Most modern CLI tools (npm, rustup, Homebrew) display a brief, non-intrusive notice when a newer version is available. This helps users stay current without requiring them to actively poll for updates.

The upgrade-command RFC explicitly listed "auto-update on every command invocation" as a non-goal and noted "periodic background check with opt-in notification" as a future enhancement. This RFC defines that enhancement.

### Design Principles

1. **Never block the user.** The check must not add latency to any command.
2. **Never be annoying.** The notice should be rare, single-line, and easy to suppress.
3. **Never phone home unexpectedly.** The network request is rate-limited and skipped in CI.

## Goals

1. Show a one-line upgrade notice when a newer version of `vp` is available
2. Zero impact on command latency (fully async, cached)
3. Reasonable default frequency (once per 24 hours)
4. Easy to disable via environment variable
5. Reuse the existing npm registry resolution from the upgrade command

## Non-Goals

1. Auto-installing updates (user must explicitly run `vp upgrade`)
2. Checking local `vite-plus` package versions (only the global CLI)
3. Showing notices for pre-release/test channel versions

## User Stories

### Story 1: New Version Available

```
$ vp build
...build output...

vp update available: 0.1.0 → 0.2.0, run `vp upgrade`
```

### Story 2: Already Up to Date (no notice)

```
$ vp build
...build output...
```

No upgrade notice is shown — the user sees only their command output.

### Story 3: CI Environment (no notice)

```
$ CI=true vp build
...build output...
```

Upgrade checks are completely disabled in CI.

### Story 4: User Opts Out

```
$ VP_NO_UPDATE_CHECK=1 vp build
...build output...
```

No network request is made and no notice is shown.

### Story 5: Offline / Registry Unreachable

```
$ vp build
...build output...
```

The check fails silently. No notice, no error, no retry spam.

## Technical Design

### Overview

```
Command starts
       │
       ├──────────────────────────────┐
       │                              │
       ▼                              ▼
  Run the actual command        Spawn background task:
       │                         1. Check if cache is fresh (<24h)
       │                            → Yes: read cached version
       │                            → No:  query npm registry,
       │                                   write result to cache file
       │                              │
       ▼                              ▼
  Command finishes              Background task finishes
       │                              │
       ▼                              ▼
  If newer version found, print one-line notice
  Show tip (existing behavior)
  Exit
```

The background task runs concurrently with the command. When the command finishes, we check if the background task has a result (with a very short timeout — if it hasn't finished, skip the notice this time).

### Cache File

Location: `~/.vite-plus/.upgrade-check.json`

Format (single JSON line for simplicity):

```json
{ "latest": "0.2.0", "checked_at": 1711500000, "prompted_at": 1711500000 }
```

- `latest`: The version string returned by the npm registry for the `latest` dist-tag
- `checked_at`: Unix timestamp (seconds) of when the registry was last queried
- `prompted_at`: Unix timestamp (seconds) of when the user was last shown the notice

The file is small and cheap to read. A direct overwrite is sufficient — if corruption occurs (e.g., process killed mid-write), the worst case is one extra registry query.

### Check Logic (Pseudocode)

Two independent rate limits control the behavior:

1. **`checked_at`** — controls how often the registry is queried (once per 24h)
2. **`prompted_at`** — controls how often the notice is shown (once per 24h)

This means: the registry is queried at most once per day, and even if an update exists, the user sees the notice at most once per day. After displaying, `prompted_at` is updated so subsequent runs within 24h are silent.

### Display

The upgrade notice is printed to **stderr** (like tips), after the command output and before the tip line:

```
vp update available: 0.1.0 → 0.2.0, run `vp upgrade`
```

Styling:

- Single line, no indentation
- Dimmed text with version numbers highlighted (current in dim, new in green bold) and `vp upgrade` highlighted

The notice is printed **after** the command output and **before** any tip, so it feels like a natural postscript rather than an interruption.

### Suppression Rules

The notice is **not shown** when:

| Condition                       | Reason                                                          |
| ------------------------------- | --------------------------------------------------------------- |
| `VP_NO_UPDATE_CHECK=1`          | Explicit opt-out                                                |
| `CI` is set                     | CI environments should not see upgrade prompts                  |
| `VP_CLI_TEST` is set            | Test environments                                               |
| Quiet/machine-readable flags    | `--silent`, `-s`, `--json`, `--parseable`, `--format json/list` |
| `vp upgrade` is running         | Already upgrading, don't nag                                    |
| `vp upgrade --check` is running | Already checking, don't duplicate                               |
| Stderr is not a TTY             | Non-interactive / piped / redirected output                     |
| Already prompted within 24h     | Show at most once per day, not on every run                     |

### Commands That Trigger the Check

The background check runs on **all** commands except:

- `vp upgrade` (already handles version checking)
- `vp implode` (removing the tool)
- `vp lint` / `vp fmt` (too fast to benefit from a background check)
- `vp --version` / `vp -V` (version display, keep it fast)
- Any command with quiet/machine-readable flags (`--silent`, `-s`, `--json`, `--parseable`, `--format json/list`)
- Shim invocations (`node`, `npm`, `npx` via vp)

This keeps the check broadly useful without interfering with special commands.

### Integration with Tips System

The upgrade notice is **not** a tip — it is higher priority and displayed independently. When both an upgrade notice and a tip would be shown, both are displayed (notice first, then tip). The tip system's rate limiting and the upgrade check's rate limiting are independent.

```
...command output...

vp update available: 0.1.0 → 0.2.0, run `vp upgrade`

tip: short aliases available: i (install), rm (remove), un (uninstall), up (update), ls (list), ln (link)
```

### File Structure

```
crates/vite_global_cli/src/
├── upgrade_check.rs        # New: cache read/write, background check, display
├── main.rs                # Modified: spawn check, display result after command
```

No new crate — this is a small, focused module in the existing `vite_global_cli` crate. It imports `resolve_version` from the existing `commands/upgrade/registry.rs`.

### Implementation Details

#### Async Background Check

```rust
// In main.rs, before running the command:
let update_handle = if should_run_for_command(&args, &raw_args) {
    Some(tokio::spawn(check_for_update()))
} else {
    None
};

// After command completes:
if let Some(handle) = update_handle {
    // Wait up to 500ms for the result — if the network is slow, skip it
    match tokio::time::timeout(Duration::from_millis(500), handle).await {
        Ok(Ok(Some(result))) => {
            display_upgrade_notice(&result); // also records prompted_at
        }
        _ => {} // Timeout, error, or no update — silent
    }
}
```

The 500ms timeout ensures that even if the registry is slow, the user's command exits promptly. In practice, most checks will read from cache (instant) or complete the network request during the time the actual command runs.

`display_upgrade_notice` updates `prompted_at` in the cache file after showing the notice, so subsequent runs within 24h are silent.

## Design Decisions

### 1. Cache-Based Rate Limiting (Not Probabilistic)

**Decision**: Check once per 24 hours, cached to disk.

**Alternatives considered**:

- Probabilistic (1-in-N chance per invocation) — simpler but inconsistent; unlucky users might never see the notice
- Timer-based without cache — would need a background daemon or cron job

**Rationale**: Deterministic behavior, no surprises. The cache file is tiny and cheap to read. 24 hours is long enough to not annoy, short enough to be useful.

### 2. Background Async (Not Post-Command Blocking)

**Decision**: Spawn the registry query concurrently with the command.

**Alternatives considered**:

- Check after the command finishes — adds visible latency
- Separate background daemon — heavyweight, harder to manage

**Rationale**: The registry query runs in parallel with the actual command. By the time the command finishes, the check is usually done. The 500ms timeout is a safety net for slow networks.

### 3. Stderr for the Notice

**Decision**: Print to stderr, not stdout.

**Rationale**: Matches the tip system. Does not pollute stdout which may be piped or parsed. Tools that capture stdout (e.g., `result=$(vp ...)`) are unaffected.

### 4. No Opt-In Required

**Decision**: Enabled by default, with easy opt-out via `VP_NO_UPDATE_CHECK=1`.

**Alternatives considered**:

- Opt-in only — most users would never discover it
- Ask on first run — adds friction to installation

**Rationale**: Most CLI tools (npm, pip, gh) enable update checks by default. The check is non-blocking and the notice is rare (at most once per 24 hours, only when an update exists). Users who don't want it can set a single env var.

### 5. Semver Comparison (Not String Equality)

**Decision**: Only show the notice when `latest` is strictly greater than `current` per semver.

**Rationale**: String inequality would prompt prerelease/alpha users to "downgrade" to an older stable release. Semver comparison ensures the notice only appears for genuine upgrades. Dev builds (`0.0.0`) are skipped entirely.

## Testing Strategy

### Unit Tests

- Cache read/write: valid JSON, corrupt file, missing file
- `should_check`: respects env vars, cache freshness, TTY detection
- Version comparison: same version, different version, pre-release

### Integration Tests

- Mock registry server returning a version, verify notice is displayed
- Verify no notice when cache is fresh
- Verify no notice in CI mode
- Verify timeout behavior (slow mock server)

### Manual Testing

```bash
# Clear cache to force a fresh check
rm ~/.vite-plus/.upgrade-check.json

# Run any command — should show notice if behind latest
vp --version

# Run again immediately — should not re-query (cached)
vp build

# Disable and verify
VP_NO_UPDATE_CHECK=1 vp build
```

## References

- [RFC: Self-Update Command](./upgrade-command.md)
- [RFC: CLI Tips](./cli-tips.md)
- [npm update-notifier pattern](https://github.com/yeoman/update-notifier)
- [Rust CLI update check (cargo-update)](https://github.com/nabijaczleweli/cargo-update)
