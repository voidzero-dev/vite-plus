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
Foreground `vp` starts
       │
       ├── parse the requested command and read the local cache
       │          │
       │          ├── ineligible command or fresh cache → spawn nothing
       │          └── stale/missing cache → launch detached
       │                         `vp upgrade --background-check`
       │                                  │
       │                                  ├── fresh cache or lock held → exit silently
       │                                  └── acquire OS file lock → atomically write
       │                                      `unknown` cooldown → query registry
       │                                      → atomically write final status
       │
       └── run the requested command without waiting for registry I/O
                  │
                  └── if no helper was launched, optionally print a cached notice
```

The foreground process performs only argument and cache checks before launching the helper, avoiding a new process while the cache is fresh. The helper is placed in a separate process group with standard streams disconnected, so it can finish after the foreground command and its shell prompt return. A command that launches a helper does not consume its result; a later eligible command can display the notice.

### Cache File

Locations:

- Cache: `~/.vite-plus/cache/upgrade-check.json`
- Cross-process lock: `~/.vite-plus/cache/upgrade-check.lock`

Format (single JSON line for simplicity):

```json
{
  "checked_for": "0.1.0",
  "latest": "0.2.0",
  "status": "available",
  "checked_at": 1711500000,
  "prompted_at": 1711500000
}
```

- `checked_for`: The installed `vp` version this result applies to
- `latest`: The version returned by the npm registry during the latest successful check
- `status`: `available`, `current`, or `unknown`
- `checked_at`: Unix timestamp (seconds) of when the latest check attempt began or completed
- `prompted_at`: Unix timestamp (seconds) of when the user was last shown the notice

Cache writes use a temporary file plus atomic replacement. An OS file lock serializes workers and prompt timestamp updates, releases automatically when a process exits, and stores a generation token so a worker cannot write into an install that was removed or replaced while its request was in flight. The worker writes an `unknown` result with a fresh `checked_at` before its first network await, so cancellation, offline registries, and abrupt shell exit cannot cause a request on every invocation.

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

### Check Triggers and Foreground Suppression

After parsing an eligible foreground command, `vp` checks opt-out and CI state plus the local cache. It launches a detached worker only when that cache is stale or missing. The worker repeats the cache check after startup, then uses the cross-process lock and another cache check to coordinate concurrent invocations. Its standard streams are discarded, and foreground commands never wait for its registry request.

The cached notice is not displayed after:

- `vp upgrade` (already handles version checking)
- `vp implode` (removing the tool)
- `vp lint` / `vp fmt` (too fast to benefit from a background check)
- `vp --version` / `vp -V` (version display, keep it fast)
- Any command with quiet/machine-readable flags (`--silent`, `-s`, `--json`, `--parseable`, `--format json/list`)
- Shim invocations (`node`, `npm`, `npx` via vp)

Shim invocations do not pass through the foreground notice path.

### File Structure

```
crates/vp_global_cli/src/
├── upgrade_check.rs        # New: cache read/write, background check, display
├── main.rs                # Modified: conditionally launch helper and display cached result
└── cli.rs                 # Modified: hidden background-check option
```

No new crate — this is a small, focused module in the existing `vp_global_cli` crate. It imports `resolve_version` from the existing `commands/upgrade/registry.rs`.

### Implementation Details

#### Background Check Command

```rust
if options.background_check {
    run_background_check().await;
    return Ok(ExitStatus::default());
}
```

`--background-check` is hidden because it is an implementation detail of the foreground launcher. The foreground `vp` process configures and spawns this command as a detached child before running the requested command. The hidden command repeats the cheap policy and cache checks to close races between concurrent foreground invocations.

Foreground commands that did not launch a helper call `display_cached_upgrade_notice` after completing. This path performs no network work and only acquires the lock when an available, unprompted cached result exists.

## Design Decisions

### 1. Cache-Based Rate Limiting (Not Probabilistic)

**Decision**: Check once per 24 hours, cached to disk.

**Alternatives considered**:

- Probabilistic (1-in-N chance per invocation) — simpler but inconsistent; unlucky users might never see the notice
- Timer-based without cache — would need a background daemon or cron job

**Rationale**: Deterministic behavior, no surprises. The cache file is tiny and cheap to read. 24 hours is long enough to not annoy, short enough to be useful.

### 2. Detached Background Process (Not an In-Process Task)

**Decision**: Let eligible foreground `vp` commands launch the hidden Rust check command as a detached process, but only after determining that the cache is stale.

**Alternatives considered**:

- Check after the command finishes — adds visible latency
- Let shell integrations launch a worker — excludes shells without an integration and starts unnecessary processes while the cache is fresh
- Spawn a Tokio task inside the foreground CLI — its runtime must wait or cancel the request when the CLI exits
- Separate background daemon — heavyweight, harder to manage

**Rationale**: The foreground process can avoid almost all helper launches with a cheap cache read, while the detached process has no registry-request latency tail. Keeping the launcher in `vp` also provides consistent behavior across shells without maintaining a daemon.

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

- Cache read/write: valid JSON, atomic replacement, corrupt/missing files
- OS file-lock exclusivity, automatic release, and install-generation invalidation
- `should_check`: respects env vars, cache freshness, TTY detection
- Version comparison: same version, different version, pre-release

### Integration Tests

- Mock registry server returning a version, verify notice is displayed
- Verify no notice when cache is fresh
- Verify no notice in CI mode
- Start concurrent checks against a slow mock registry; verify exactly one request and that the cooldown is persisted before the response
- Verify an eligible foreground command launches a detached check only when the cache is stale

### Manual Testing

```bash
# Clear cache to force a fresh check
rm ~/.vite-plus/cache/upgrade-check.json

# Run an eligible foreground command to launch the check
vp build

# Run again after the background request completes — should not re-query (cached)
vp build

# Disable and verify
VP_NO_UPDATE_CHECK=1 vp build
```

## References

- [RFC: Self-Update Command](./upgrade-command.md)
- [npm update-notifier pattern](https://github.com/yeoman/update-notifier)
- [Rust CLI update check (cargo-update)](https://github.com/nabijaczleweli/cargo-update)
