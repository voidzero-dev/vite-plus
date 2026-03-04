# RFC: CLI Output Polish

## Status

Draft

## Executive Summary

Vite+ wraps several sub-tools (rolldown-vite, vitest, oxlint, oxfmt) and has native Rust commands (upgrade, env, vpx, package manager commands). Each sub-tool currently shows its own branding and uses inconsistent formatting for messages, prefixes, and status indicators. This RFC proposes unifying all CLI output under the "Vite+" brand identity with consistent message formatting, starting with rolldown-vite (whose source is cloned locally and directly modifiable) and extending to Rust commands and other sub-tools.

## Motivation

### Current Pain Points

**1. Fragmented branding confuses users**

When a user runs `vp dev`, the banner displays:

```
  VITE v8.0.0-beta.13  ready in 312 ms
```

When they run `vp build`, it shows:

```
  vite v8.0.0-beta.13 building client environment for production...
```

Neither identifies the experience as "Vite+". Users who installed `vite-plus` see "VITE" branding and may not understand the relationship.

**2. Message prefix styles are inconsistent across Rust commands**

| File             | Prefix                         | Example                                          |
| ---------------- | ------------------------------ | ------------------------------------------------ |
| `upgrade/mod.rs` | `info: ` (lowercase)           | `info: checking for updates...`                  |
| `upgrade/mod.rs` | `warn: ` (lowercase)           | `warn: Shim refresh failed (non-fatal): ...`     |
| `vpx.rs`         | `Error: ` (Title case)         | `Error: vpx requires a command to run`           |
| `which.rs`       | `error:` (lowercase, bold red) | `error: tool 'foo' not found`                    |
| `main.rs`        | `Error: ` (Title case)         | `Error: Failed to get current directory`         |
| `pin.rs`         | `Warning: ` (Title case)       | `Warning: Failed to download Node.js ...`        |
| `pin.rs`         | `Note: `                       | `Note: Version will be downloaded on first use.` |
| `dlx.rs`         | `Warning: ` (Title case)       | `Warning: yarn dlx does not support shell mode`  |
| `dlx.rs`         | `Note: `                       | `Note: yarn@1 does not have dlx command...`      |

**3. Status indicator symbols vary**

| Context          | Success                | Failure              | Warning                 |
| ---------------- | ---------------------- | -------------------- | ----------------------- |
| `doctor.rs`      | `✓` (`\u{2713}`) green | `✗` (`\u{2717}`) red | `⚠` (`\u{26A0}`) yellow |
| `upgrade/mod.rs` | `✔` (`\u{2714}`) green | —                    | —                       |
| Task runner      | `✓`                    | `✗`                  | —                       |

**4. Color libraries differ (but this is acceptable)**

| Layer              | Library                 |
| ------------------ | ----------------------- |
| Rust (global CLI)  | `owo_colors`            |
| JS (vite-plus CLI) | `node:util styleText()` |
| rolldown-vite      | `picocolors`            |

**5. The `[vite]` logger prefix in rolldown-vite**

The logger in `rolldown-vite/packages/vite/src/node/logger.ts` defaults to `prefix = '[vite]'` for timestamped messages. This shows up during dev server operation as colored `[vite]` tags.

### What Users See Today

```bash
# Dev server — shows "VITE" branding
$ vp dev
  VITE v8.0.0-beta.13  ready in 312 ms
  ➜  Local:   http://localhost:5173/

# Build — shows lowercase "vite" branding
$ vp build
  vite v8.0.0-beta.13 building client environment for production...

# Upgrade — uses "info:" prefix (lowercase)
$ vp upgrade --check
  info: checking for updates...
  info: found vite-plus@0.4.0 (current: 0.3.0)

# vpx — uses "Error:" prefix (Title case)
$ vpx
  Error: vpx requires a command to run
```

## Goals

1. Establish a unified branding format where "VITE+" is the primary identity shown to users
2. Standardize message prefix formatting across all commands to a single convention
3. Standardize status indicator symbols to a single set
4. Apply branding changes to rolldown-vite output (dev banner, build banner, logger prefix)
5. Define a repeatable approach: modify sub-tool source directly to achieve consistent output

## Non-Goals

1. Changing the `VITE_` environment variable prefix (user-facing API, not CLI output)
2. Changing internal build markers (`__VITE_ASSET__`, `__VITE_PRELOAD__`, etc.)
3. Changing `vite.config.ts` file names or config API naming
4. Changing the color library used by each component (each keeps its own)
5. Rebranding vitest or oxlint in Phase 1 (deferred to later phases)

## Proposed Solution

### Overview: Direct Source Modification

Since vite-plus clones sub-tool source repositories (rolldown-vite at `rolldown-vite/`, rolldown at `rolldown/`), we modify the source directly. This is simple, transparent, and easy to audit via `git diff`. When syncing upstream, branding patches are rebased or re-applied — a small, well-defined set of changes.

Other sub-tools (vitest, oxlint, oxfmt) can follow the same pattern once their source is cloned or forked.

### Phase 1: Rebrand rolldown-vite Output

#### 1.1 Dev server banner

**File:** `rolldown-vite/packages/vite/src/node/cli.ts` (line 256)

**Current:**

```javascript
info(
  `\n  ${colors.green(
    `${colors.bold('VITE')} v${VERSION}`,
  )}${modeString}  ${startupDurationString}\n`,
  { clear: !hasExistingLogs },
);
```

**Output:** `VITE v8.0.0-beta.13  ready in 312 ms`

**Proposed change:**

```javascript
info(
  `\n  ${colors.green(
    `${colors.bold('VITE+')} v${VITE_PLUS_VERSION}`,
  )}${modeString}  ${startupDurationString}\n`,
  { clear: !hasExistingLogs },
);
```

**Output:** `VITE+ v0.3.0  ready in 312 ms`

Where `VITE_PLUS_VERSION` is the vite-plus package version, injected via:

- A new constant in `rolldown-vite/packages/vite/src/node/constants.ts`, or
- Read from an environment variable set by the Rust CLI before spawning vite (e.g., `VITE_PLUS_VERSION`)

**Recommended approach:** Environment variable injection. The Rust NAPI binding in `packages/cli/binding/src/cli.rs` already merges environment variables when spawning sub-tools via `merge_resolved_envs()`. We add `VITE_PLUS_VERSION` to the env map, and read it in rolldown-vite:

```javascript
const VITE_PLUS_VERSION = process.env.VITE_PLUS_VERSION || VERSION;
```

This is clean: the rolldown-vite source change is minimal (reads an env var with fallback), and the version injection happens in the Rust layer that already owns this responsibility.

#### 1.2 Build banner

**File:** `rolldown-vite/packages/vite/src/node/build.ts` (line 789)

**Current:**

```javascript
logger.info(
  colors.blue(
    `vite v${VERSION} ${colors.green(
      `building ${environment.name} environment for ${environment.config.mode}...`,
    )}`,
  ),
);
```

**Output:** `vite v8.0.0-beta.13 building client environment for production...`

**Proposed change:**

```javascript
logger.info(
  colors.blue(
    `vite+ v${VITE_PLUS_VERSION} ${colors.green(
      `building ${environment.name} environment for ${environment.config.mode}...`,
    )}`,
  ),
);
```

**Output:** `vite+ v0.3.0 building client environment for production...`

#### 1.3 Logger prefix

**File:** `rolldown-vite/packages/vite/src/node/logger.ts` (line 78)

**Current:**

```javascript
prefix = '[vite]',
```

**Proposed:**

```javascript
prefix = '[vite+]',
```

#### 1.4 Other user-visible strings to audit

A full audit of rolldown-vite source for user-visible "vite" strings:

| Location                      | String                                                                       | Action                                                |
| ----------------------------- | ---------------------------------------------------------------------------- | ----------------------------------------------------- |
| `cli.ts:256`                  | `'VITE'` in banner                                                           | Change to `'VITE+'`                                   |
| `build.ts:789`                | `` `vite v${VERSION}` ``                                                     | Change to `` `vite+ v${VITE_PLUS_VERSION}` ``         |
| `logger.ts:78`                | `'[vite]'`                                                                   | Change to `'[vite+]'`                                 |
| `build.ts:674`                | `"This is deprecated and will override all Vite.js default output options."` | Leave — refers to the Vite project name, not branding |
| `build.ts:680`                | `"Vite does not support..."`                                                 | Leave — project name reference                        |
| `build.ts:1079`               | `"[vite]: Rolldown failed to resolve..."`                                    | Change to `"[vite+]: ..."`                            |
| Config error messages         | `"Vite requires Node.js..."`                                                 | Leave — project name reference                        |
| `vite:*` plugin name prefixes | `'vite:esbuild-banner-footer-compat'` etc.                                   | Leave — internal plugin IDs, not user-facing          |
| `VITE_*` env var detection    | `import.meta.env.VITE_*`                                                     | Leave — user API, not branding                        |

**Principle:** Change branding text that appears in terminal output. Leave references to "Vite" as a project/software name in error descriptions, and leave all internal identifiers.

### Phase 2: Standardize Rust CLI Output

#### 2.1 Create a shared output module

Add formatting functions to a shared location. This could be a new `vite_output` crate or a module within an existing shared crate.

```rust
use owo_colors::OwoColorize;

// Standard status symbols
pub const CHECK: &str = "\u{2713}";   // ✓ — success
pub const CROSS: &str = "\u{2717}";   // ✗ — failure
pub const WARN_SIGN: &str = "\u{26A0}"; // ⚠ — warning
pub const ARROW: &str = "\u{2192}";   // → — transitions

/// Print an info message to stderr.
pub fn info(msg: &str) {
    eprintln!("{} {}", "info:".bright_blue().bold(), msg);
}

/// Print a warning message to stderr.
pub fn warn(msg: &str) {
    eprintln!("{} {}", "warn:".yellow().bold(), msg);
}

/// Print an error message to stderr.
pub fn error(msg: &str) {
    eprintln!("{} {}", "error:".red().bold(), msg);
}

/// Print a note message to stderr (supplementary info).
pub fn note(msg: &str) {
    eprintln!("{} {}", "note:".dimmed().bold(), msg);
}

/// Print a success line with checkmark to stdout.
pub fn success(msg: &str) {
    println!("{} {}", CHECK.green(), msg);
}
```

**Design choice — lowercase prefixes:** Matches the Rust compiler convention (`error[E0308]:`, `warning:`, `note:`). Since vite-plus has a Rust core, aligning with the Rust ecosystem feels natural and is more compact than Title case.

#### 2.2 Standardize symbols

Adopt a single set everywhere:

| Symbol           | Unicode      | Usage           | Color  |
| ---------------- | ------------ | --------------- | ------ |
| `✓` (`\u{2713}`) | Check mark   | Success         | green  |
| `✗` (`\u{2717}`) | Ballot X     | Failure         | red    |
| `⚠` (`\u{26A0}`) | Warning sign | Warning/caution | yellow |
| `→` (`\u{2192}`) | Right arrow  | Transitions     | none   |

**Change:** Replace `\u{2714}` (heavy check mark ✔) in `upgrade/mod.rs` with `\u{2713}` (check mark ✓) for consistency with `doctor.rs` and the task runner.

#### 2.3 Migration targets

Commands to update (representative, not exhaustive):

| File                 | Current                               | New                                |
| -------------------- | ------------------------------------- | ---------------------------------- |
| `upgrade/mod.rs:58`  | `eprintln!("info: checking...")`      | `output::info("checking...")`      |
| `upgrade/mod.rs:69`  | `eprintln!("info: found...")`         | `output::info("found...")`         |
| `upgrade/mod.rs:173` | `eprintln!("warn: Shim refresh...")`  | `output::warn("Shim refresh...")`  |
| `upgrade/mod.rs:75`  | `"\u{2714}".green()`                  | `output::CHECK.green()`            |
| `main.rs:75`         | `eprintln!("Error: Failed...")`       | `output::error("Failed...")`       |
| `main.rs:121`        | `eprintln!("Error: {e}")`             | `output::error(...)`               |
| `vpx.rs:72`          | `eprintln!("Error: vpx requires...")` | `output::error("vpx requires...")` |
| `which.rs:40`        | `"error:".red().bold()`               | `output::error(...)`               |
| `pin.rs:142`         | `println!("  Note: Version...")`      | `output::note("Version...")`       |
| `pin.rs:155`         | `eprintln!("Warning: Failed...")`     | `output::warn("Failed...")`        |
| `dlx.rs:167`         | `eprintln!("Warning: yarn dlx...")`   | `output::warn("yarn dlx...")`      |
| `dlx.rs:184`         | `eprintln!("Note: yarn@1...")`        | `output::note("yarn@1...")`        |

The `vite_install` crate also has `Warning:` and `Note:` messages across multiple command files (`list.rs`, `why.rs`, `outdated.rs`, `pack.rs`, `publish.rs`, `cache.rs`, `config.rs`, `audit.rs`, `dlx.rs`, `unlink.rs`, `update.rs`, `rebuild.rs`, `whoami.rs`). All should be migrated.

### Phase 3: Rebrand vitest Output

Vitest is bundled (not cloned source) via `@voidzero-dev/vite-plus-test`. Its build script (`packages/test/build.ts`) copies and rewrites vitest's dist files. We patch the bundled cac chunk during the build to rebrand CLI output.

#### 3.1 Approach: Build-time patching of bundled cac chunk

After `bundleVitest()` copies vitest files to `dist/`, a `brandVitest()` step patches the cac chunk (`dist/chunks/cac.*.js`) with string replacements:

1. `cac("vitest")` → `cac("vp test")` — CLI name shown in banner and help output
2. `var version = "<semver>"` → `var version = process.env.VITE_PLUS_VERSION || "<semver>"` — runtime version injection via env var
3. `/^vitest\/\d+\.\d+\.\d+$/` regex → `/^vp test\/[\d.]+$/` — so the help callback can still find the banner line
4. `$ vitest --help --expand-help` → `$ vp test --help --expand-help` — hardcoded help text

The Rust NAPI binding injects `VITE_PLUS_VERSION` env var (same mechanism used for rolldown-vite build/dev/preview commands), so `vp test -h` shows `vp test/<vite-plus-version>`.

#### 3.3 Remaining `vite` → `vp` branding in CLI output

Several user-visible strings still show `vite` instead of `vp`:

1. **Local CLI help usage line** (`packages/cli/binding/src/cli.rs`): `Usage: vite <COMMAND>` → `Usage: vp <COMMAND>`
2. **Pack CLI cac name** (`packages/cli/src/pack-bin.ts`): `cac('vp pack')` → `cac('vp pack')`
3. **Migration message** (`packages/cli/src/migration/bin.ts`): `vp install` → `vp install`

These are straightforward string replacements in the source, verified by snap test updates.

#### 3.4 Future: oxlint, oxfmt

For oxlint and oxfmt, pre-spawn banners or build-time patching can follow the same pattern once their source/dist is bundled.

### Phase 3.5: Rebrand tsdown Output

tsdown is bundled via `@voidzero-dev/vite-plus-core`. Its build script (`packages/core/build.ts`) bundles tsdown's dist files via rolldown.

#### 3.5.1 Approach: Build-time patching of bundled build chunk

After `bundleTsdown()` rebuilds tsdown, a `brandTsdown()` step patches the build chunk (`dist/tsdown/build-*.js`) with string replacements:

1. `"tsdown <your-file>"` → `"vp pack <your-file>"` — error message when no input files found

Internal identifiers are left unchanged: debug namespaces (`tsdown:*`), plugin names (`tsdown:external`), config prefix (`tsdown.config`), temp dirs (`tsdown-pack-`).

### Phase 4: JS-Side Output Consistency

The JS code in `packages/cli/src/utils/terminal.ts` already has `accent()`, `headline()`, `muted()`, `success()`, `error()` functions. Extend it with prefix functions matching the Rust convention:

```typescript
export function info(msg: string) {
  console.error(styleText(['blue', 'bold'], 'info:'), msg);
}

export function warn(msg: string) {
  console.error(styleText(['yellow', 'bold'], 'warn:'), msg);
}

export function errorMsg(msg: string) {
  console.error(styleText(['red', 'bold'], 'error:'), msg);
}

export function note(msg: string) {
  console.error(styleText(['gray', 'bold'], 'note:'), msg);
}
```

Migrate JS-side code (`migration/bin.ts`, `create/bin.ts`) to use these shared functions where they currently use ad-hoc formatting.

## Design Decisions

### D1: Direct source modification over build-time transforms

**Decision:** Modify rolldown-vite source files directly.

**Rationale:** The user has the source cloned locally. Direct modification is transparent — anyone can `git diff rolldown-vite/` to see exactly what changed. The set of branding changes is small and well-defined (3-5 files), making rebasing during upstream sync manageable. Build-time transforms (Rolldown plugins in `packages/core/build.ts`) are an alternative that avoids merge conflicts, but they are less visible and can break silently when upstream changes the strings being matched.

### D2: Only show vite-plus version, not underlying vite version

**Decision:** Banner shows `VITE+ v0.3.0`, not `VITE+ v0.3.0 (vite 8.0.0-beta.13)`.

**Rationale:** Cleaner output. The underlying vite version is still available via `vp --version` which shows a detailed version table. The banner should communicate identity, not debug information.

### D3: Inject version via environment variable

**Decision:** The Rust CLI sets `VITE_PLUS_VERSION` env var before spawning rolldown-vite. The modified rolldown-vite source reads it with a fallback.

**Rationale:** This avoids hardcoding the version in rolldown-vite source (which would require updating on every release). The Rust CLI already manages environment variables for sub-tool spawning via `merge_resolved_envs()`. The env var approach is the minimal-touch change to rolldown-vite.

### D4: Lowercase prefixes (`info:` not `Info:`)

**Decision:** All prefixes are lowercase with bold coloring: `info:`, `warn:`, `error:`, `note:`.

**Rationale:** Matches the Rust compiler convention. Compact and consistent. The current codebase is split between lowercase (`info:` in upgrade.rs) and Title case (`Warning:` in vpx.rs) — picking one convention eliminates the inconsistency.

### D5: Pre-spawn banners for sub-tools we don't control

**Decision:** Print a single `vite+ v0.3.0 — <command>` line before spawning vitest/oxlint/oxfmt.

**Rationale:** Parsing or wrapping sub-tool stdout/stderr is fragile and can break ANSI colors, progress indicators, and interactive output. A single leading line is non-intrusive. Long-term, these sub-tools should be directly modified once their source is cloned.

### D6: Keep each layer's color library

**Decision:** Rust keeps `owo_colors`, JS keeps `node:util styleText()`, rolldown-vite keeps `picocolors`.

**Rationale:** Changing color libraries is high-risk, low-reward. The shared formatting module abstracts the library choice so the output convention is consistent regardless of the underlying library.

## Scope of rolldown-vite Changes

### Strings to Change

These are user-visible branding strings that appear in terminal output:

1. **`cli.ts:256`** — Dev server banner: `'VITE'` → `'VITE+'`, `VERSION` → `VITE_PLUS_VERSION`
2. **`build.ts:789`** — Build banner: `` `vite v${VERSION}` `` → `` `vite+ v${VITE_PLUS_VERSION}` ``
3. **`logger.ts:78`** — Logger prefix: `'[vite]'` → `'[vite+]'`
4. **`build.ts:1079`** — Error message prefix: `'[vite]:'` → `'[vite+]:'`

### Strings to Leave Unchanged

These are internal identifiers, API references, or project name references:

- `VITE_` environment variable prefix and detection
- `VITE_PACKAGE_DIR`, `CLIENT_ENTRY`, `ENV_ENTRY` constant names
- `__VITE_ASSET__`, `__VITE_PRELOAD__` internal build markers
- `vite:*` plugin name prefixes (`vite:esbuild-banner-footer-compat`, etc.)
- `vite.config.ts`, `vite.config.js` file detection
- Error messages that reference "Vite" as a project name (e.g., `"Vite does not support..."`)
- `import.meta.env.VITE_*` documentation and detection
- `.vite/` cache directory name

## Implementation Plan

### Phase 1: rolldown-vite Rebranding

1. Add `VITE_PLUS_VERSION` env var injection in `packages/cli/binding/src/cli.rs` for vite commands (build, dev, preview)
2. Modify `rolldown-vite/packages/vite/src/node/cli.ts` — read env var, change banner text
3. Modify `rolldown-vite/packages/vite/src/node/build.ts` — change build banner text
4. Modify `rolldown-vite/packages/vite/src/node/logger.ts` — change default prefix
5. Modify `rolldown-vite/packages/vite/src/node/build.ts:1079` — change error prefix
6. Rebuild with `pnpm bootstrap-cli` and verify output
7. Update affected snap tests

### Phase 2: Rust CLI Output Standardization

1. Create shared output module with `info()`, `warn()`, `error()`, `note()`, `success()` and symbol constants
2. Add as dependency to `vite_global_cli` and `vite_install`
3. Migrate `upgrade/mod.rs` (6 message sites)
4. Migrate `main.rs` error handling (3 sites)
5. Migrate `vpx.rs` (4 sites)
6. Migrate `env/which.rs` (3 sites)
7. Migrate `env/pin.rs` (3 sites)
8. Migrate `vite_install/src/commands/*.rs` Warning/Note messages
9. Update snap tests

### Phase 2.5: tsdown Branding

1. Add `brandTsdown()` in `packages/core/build.ts` after `bundleTsdown()`
2. Patch `dist/tsdown/build-*.js` with string replacement: `"tsdown <your-file>"` → `"vp pack <your-file>"`
3. Update snap tests

### Phase 3: Sub-tool Banners

1. Add `print_banner()` for vitest, oxlint, oxfmt in `packages/cli/binding/src/cli.rs`
2. Gate on TTY check (skip in piped output)
3. Update snap tests

### Phase 4: JS Output Consistency

1. Add prefix functions to `packages/cli/src/utils/terminal.ts`
2. Migrate `migration/bin.ts` and `create/bin.ts` to use shared functions
3. Update snap tests

## Testing Strategy

### Snap Tests

Many existing snap tests will need updates due to prefix and branding changes:

- `snap-tests-global/command-upgrade-check/snap.txt` — `info:` prefix format
- `snap-tests-global/command-upgrade-rollback/snap.txt` — success format
- `snap-tests-global/command-env-which/snap.txt` — error format
- `snap-tests/command-dev-*/snap.txt` — vite banner change
- `snap-tests/command-build-*/snap.txt` — build banner change
- All `Warning:`/`Note:` snap outputs across global snap tests
- `snap-tests/command-pack-no-input/snap.txt` — tsdown error message branding

**Process:** Run `pnpm -F vite-plus snap-test` after each phase, review `git diff` on `snap.txt` files, and verify the new formatting matches expectations.

### Manual Verification

- `vp dev` shows `VITE+ v<version>  ready in X ms`
- `vp build` shows `vite+ v<version> building ...`
- `vp upgrade --check` shows `info: checking for updates...`
- `vp env doctor` shows consistent ✓/✗/⚠ symbols
- `vpx` (no args) shows `error: vpx requires a command to run`
- Piped output (`vp dev | cat`) does not show sub-tool banners

### CI

- All existing `cargo test` and snap tests pass with updated expectations
- No regressions in rolldown-vite's own test suite

## Future Enhancements

- Clone oxlint/oxfmt source for `vp lint` / `vp fmt` branding (or apply build-time patching)
- Unified progress indicator style (spinner, progress bar) across long-running operations
- Structured JSON output mode (`--json`) for machine-readable output across all commands
