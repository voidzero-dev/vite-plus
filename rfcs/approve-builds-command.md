# RFC: Vite+ Approve Builds Command (`vp pm approve-builds`)

## Summary

Add `vp pm approve-builds` subcommand under the `vp pm` command group to provide a unified, cross-package-manager interface for approving (or denying) dependency lifecycle scripts (`preinstall` / `install` / `postinstall`). The design mirrors [`pnpm approve-builds`](https://pnpm.io/cli/approve-builds) one-to-one and is adapted to [`bun pm trust`](https://bun.com/docs/pm/cli/pm#trust), with a warning-and-no-op fallback for npm and yarn (which do not have an equivalent first-class command).

This is a sub-RFC of [pm-command-group.md](./pm-command-group.md) and extends the list of `vp pm` subcommands.

## Motivation

Modern package managers ship with **opt-in lifecycle script execution** to mitigate supply-chain attacks:

- **pnpm ≥ v10** ignores all install scripts by default and requires explicit approval via the `allowBuilds` map in `pnpm-workspace.yaml`.
- **bun** blocks lifecycle scripts for any dependency not listed in `trustedDependencies` (in `package.json`) or shipped in bun's default-trusted list.
- **npm** still executes lifecycle scripts by default but can be flipped off with `ignore-scripts=true` in `.npmrc`.
- **yarn (Berry)** blocks third-party build scripts by default (`enableScripts` is `false`); per-package opt-in is via `dependenciesMeta.<pkg>.built: true` in `package.json`.

This produces two parallel workflows for the **same conceptual task** — "I trust `esbuild` to run its post-install build":

```bash
# pnpm
pnpm approve-builds                       # interactive
pnpm approve-builds esbuild fsevents      # by name
pnpm approve-builds esbuild !core-js      # approve esbuild, deny core-js
pnpm approve-builds --all                 # approve all pending

# bun
bun pm trust esbuild                      # trust one package
bun pm trust --all                        # trust everything pending

# npm / yarn
# (no equivalent command; users must edit package.json or config manually)
```

### Pain Points

1. **Conceptual divergence**: pnpm stores approvals in `pnpm-workspace.yaml` (`allowBuilds:`), bun stores them in `package.json` (`trustedDependencies:`). The two are semantically similar but live in different files with different shapes.
2. **Asymmetric models**: pnpm supports both _allow_ and _deny_ (via `!pkg`), bun only supports _trust_ (deny is the default).
3. **CI portability**: A monorepo migrating between pnpm and bun today must rewrite all build-approval automation.

### Proposed Solution

A single Vite+ subcommand routes to the underlying package manager's idiomatic command:

```bash
# Works for all package managers
vp pm approve-builds                          # interactive prompt (pnpm/aube) / warn (bun, npm, yarn)
vp pm approve-builds esbuild fsevents         # approve listed packages
vp pm approve-builds esbuild !core-js         # approve esbuild, deny core-js (pnpm/aube only)
vp pm approve-builds --all                    # approve every pending package
```

## Proposed Solution

### Command Syntax

```bash
vp pm approve-builds [PACKAGES...] [OPTIONS]
```

**Positional arguments:**

- `PACKAGES...`: One or more package names to approve.
    - Prefix with `!` to deny (`!core-js`) — pnpm/aube only; for bun this prints a warning explaining the model doesn't support denylisting and skips the denied entries.
    - Omitting all positionals (and `--all`) launches **interactive mode** on pnpm/aube; bun has no interactive picker so we print a `note` asking the user to pass package names explicitly.

**Options:**

- `--all`: Approve every package that is currently pending approval. Maps to `pnpm approve-builds --all` (added in pnpm v10.32.0) and `bun pm trust --all`.

(Intentionally matches pnpm's documented surface. `pnpm approve-builds --global` was removed in pnpm v11.0.0, so we do not expose `-g/--global`. Other ergonomics — listing pending packages, showing default-trusted lists, CI confirmation gates — are deferred to follow-up RFCs; see [Future Enhancements](#future-enhancements).)

### Subcommand Behavior

#### 1. Interactive mode

```bash
$ vp pm approve-builds
Detected package manager: pnpm@10.32.0
Running: pnpm approve-builds

? Choose which packages to build (Press <space> to select, <a> to toggle all, <i> to invert selection)
 ◯ @biomejs/biome
 ◯ esbuild
 ◯ fsevents
 ◯ sharp
```

- **pnpm**: forwards interactively to `pnpm approve-builds` (it owns the TUI).
- **bun**: bun has no interactive picker for `bun pm trust`. Vite+ prints:

  ```
  note  bun pm trust requires package names. Run `bun pm untrusted` to see
        which packages are pending, then pass them explicitly:
          vp pm approve-builds <pkg> [<pkg>...]
          vp pm approve-builds --all
  ```

  Exit 0.

- **npm**: prints a warning and exits 0 (no-op):

  ```
  warn  npm runs lifecycle scripts by default. To restrict them, set
        `ignore-scripts=true` in .npmrc and rebuild approved packages with
        `vp pm rebuild <package>`.
  ```

- **yarn**: prints a warning and exits 0 (no-op):

  ```
  warn  yarn does not run third-party build scripts by default. To allow a
        package, set `dependenciesMeta["<package>"].built: true` in package.json.
  ```

#### 2. Direct approval

```bash
$ vp pm approve-builds esbuild fsevents
Detected package manager: pnpm@10.32.0
Running: pnpm approve-builds esbuild fsevents
✔ esbuild approved
✔ fsevents approved
```

- **pnpm**: pass-through; pnpm updates `allowBuilds` in `pnpm-workspace.yaml`.
- **bun**: invokes `bun pm trust esbuild fsevents`; bun appends `trustedDependencies` in `package.json`.
- **npm / yarn**: prints the warning shown above and exits 0.

#### 3. Deny syntax (`!pkg`)

```bash
$ vp pm approve-builds esbuild !core-js
Detected package manager: pnpm@10.32.0
Running: pnpm approve-builds esbuild !core-js
✔ esbuild approved
✗ core-js denied
```

- **pnpm**: pass-through (native syntax).
- **bun**: Vite+ prints:

  ```
  warn  bun does not support denylisting build scripts. Packages outside
        `trustedDependencies` in package.json are already denied by default.
        Skipping: core-js
  ```

  Then forwards the non-denied positional packages (`esbuild`) to `bun pm trust`.

- **npm / yarn**: warning as above; no-op.

#### 4. `--all`

```bash
$ vp pm approve-builds --all
Detected package manager: bun@1.3.0
Running: bun pm trust --all
✔ Trusted 4 packages
```

- **pnpm** ≥ v10.32.0: forwards to `pnpm approve-builds --all`.
- **pnpm** < v10.32.0: errors with a usage hint asking the user to upgrade pnpm or enumerate the packages explicitly.
- **bun**: forwards to `bun pm trust --all`.
- **npm / yarn**: warning as above; no-op.

### Command Mapping

**pnpm references:**

- https://pnpm.io/cli/approve-builds
- https://pnpm.io/settings#allowbuilds

**bun references:**

- https://bun.com/docs/pm/cli/pm#trust

**npm references:**

- `npm approve-scripts` / `npm deny-scripts` (npm ≥ 11.16.0, npm/cli #9360) manage an advisory `allowScripts` field in `package.json`. In npm 11.x this is advisory only: install scripts still run; npm just warns about unreviewed packages.
- For npm < 11.16.0: no equivalent command. Closest configuration: [`ignore-scripts`](https://docs.npmjs.com/cli/v11/using-npm/config#ignore-scripts) and [`npm rebuild`](https://docs.npmjs.com/cli/v11/commands/npm-rebuild).

**yarn references:**

- No equivalent command. yarn@2+ already blocks third-party build scripts by default ([`enableScripts`](https://yarnpkg.com/configuration/yarnrc#enableScripts) defaults to `false`); per-package opt-in is via [`dependenciesMeta.<pkg>.built`](https://yarnpkg.com/configuration/manifest#dependenciesMeta) in `package.json`.

| Vite+ Flag                    | pnpm                                     | npm (≥ 11.16.0)                               | yarn@1     | yarn@2+    | bun                         | Description                                 |
| ----------------------------- | ---------------------------------------- | --------------------------------------------- | ---------- | ---------- | --------------------------- | ------------------------------------------- |
| `vp pm approve-builds`        | `pnpm approve-builds`                    | `npm approve-scripts --allow-scripts-pending` | N/A (warn) | N/A (warn) | N/A (note)                  | pnpm: interactive prompt; npm: list pending |
| `vp pm approve-builds <pkg>`  | `pnpm approve-builds <pkg>`              | `npm approve-scripts <pkg>`                   | N/A (warn) | N/A (warn) | `bun pm trust <pkg>`        | Approve named packages                      |
| `vp pm approve-builds !<pkg>` | `pnpm approve-builds !<pkg>`             | `npm deny-scripts <pkg>`                      | N/A (warn) | N/A (warn) | N/A (warn — model mismatch) | Deny named packages (pnpm, npm)             |
| `--all`                       | `pnpm approve-builds --all` (≥ v10.32.0) | `npm approve-scripts --all`                   | N/A (warn) | N/A (warn) | `bun pm trust --all`        | Approve every pending package               |

**Notes:**

- **`!pkg` deny syntax is supported on pnpm and npm.** pnpm forwards `!core-js` verbatim; npm strips the `!` and routes it to `npm deny-scripts core-js`. For bun the deny syntax is rejected with a warning that names the affected positionals (so users notice rather than silently get a partial approval).
- **npm splits approve vs. deny into two separate commands** (`approve-scripts` / `deny-scripts`). Because `vp pm approve-builds` accepts both in one invocation, a mixed call (`vp pm approve-builds esbuild !core-js`) is **rejected** on npm with an actionable message asking the user to run the two operations separately. pnpm handles the mixed case in a single command.
- **npm < 11.16.0 and yarn never have an `approve-builds` command.** Vite+ prints a one-line `warn` and exits 0. For npm the warn points at upgrading to npm ≥ 11.16.0 (or `ignore-scripts`). For yarn (which blocks third-party scripts by default) the warn points at `dependenciesMeta.<pkg>.built`. We intentionally exit 0 (not non-zero) so monorepo scripts that run `vp pm approve-builds` opportunistically don't break on heterogeneous environments.
- **npm's `allowScripts` is advisory in npm 11.x.** Even after approving, install scripts still run; npm only warns about unreviewed packages at install time. Vite+ surfaces a one-line `note` after an npm approve/deny write to make this clear. Enforcement is planned for a future npm release.
- **No-args mode on bun** also exits 0 with a `note` (bun's `bun pm trust` requires package names; there's no interactive picker to forward to).
- **Configuration storage differs:** pnpm writes to `pnpm-workspace.yaml` under `allowBuilds:`. bun writes to `package.json` under `trustedDependencies: []`. Vite+ does not normalize the storage location — each PM owns its own state. (See [Design Decision §2](#2-do-not-normalize-storage).)

### Implementation Architecture

#### 1. Command Structure

**File**: `crates/vite_task/src/lib.rs`

Add a new variant under `PmCommands`:

```rust
#[derive(Subcommand, Debug)]
pub enum PmCommands {
    // ... existing subcommands

    /// Approve dependency lifecycle scripts (install/postinstall) to run
    ApproveBuilds {
        /// Packages to approve. Prefix with `!` to deny (pnpm/aube only).
        /// Omit to launch interactive mode (pnpm/aube only).
        packages: Vec<String>,

        /// Approve every package that is currently pending
        #[arg(long)]
        all: bool,
    },
}
```

#### 2. Package Manager Adapter

**File**: `crates/vite_package_manager/src/commands/approve_builds.rs` (new file)

```rust
use std::process::ExitStatus;

use vite_error::Error;
use vite_path::AbsolutePath;
use vite_shared::output::{note, warn};

use crate::package_manager::{PackageManager, PackageManagerType};

pub struct ApproveBuildsOptions<'a> {
    pub packages: &'a [String],
    pub all: bool,
}

impl PackageManager {
    /// Approve dependency lifecycle scripts.
    pub async fn run_approve_builds(
        &self,
        opts: ApproveBuildsOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        match self.client {
            PackageManagerType::Pnpm => self.pnpm_approve_builds(opts, cwd).await,
            PackageManagerType::Bun => self.bun_approve_builds(opts, cwd).await,
            PackageManagerType::Npm => {
                warn(
                    "npm runs lifecycle scripts by default. To restrict them, set \
                     `ignore-scripts=true` in .npmrc and rebuild approved packages with \
                     `vp pm rebuild <package>`.",
                );
                Ok(ExitStatus::default()) // exit 0 — no-op
            }
            PackageManagerType::Yarn => {
                note(
                    "yarn does not run third-party build scripts by default. To allow a \
                     package, set `dependenciesMeta[\"<package>\"].built: true` in package.json.",
                );
                Ok(ExitStatus::default()) // exit 0 — no-op
            }
        }
    }

    async fn pnpm_approve_builds(
        &self,
        opts: ApproveBuildsOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        if opts.all && self.version_lt("10.32.0") {
            return Err(Error::Usage(
                "`--all` requires pnpm ≥ 10.32.0. Upgrade pnpm or pass package names explicitly."
                    .into(),
            ));
        }

        let mut args = vec!["approve-builds".to_string()];
        if opts.all {
            args.push("--all".into());
        }
        args.extend(opts.packages.iter().cloned());
        self.run_raw("pnpm", &args, cwd).await
    }

    async fn bun_approve_builds(
        &self,
        opts: ApproveBuildsOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        // Reject `!pkg` deny syntax with a clear warning.
        let (denies, approves): (Vec<&String>, Vec<&String>) =
            opts.packages.iter().partition(|p| p.starts_with('!'));
        if !denies.is_empty() {
            let names: Vec<String> =
                denies.iter().map(|p| p.trim_start_matches('!').to_string()).collect();
            warn(&format!(
                "bun does not support denylisting build scripts. Packages outside\n  \
                 `trustedDependencies` in package.json are already denied by default.\n  \
                 Skipping: {}",
                names.join(", ")
            ));
        }

        // No-args mode: bun has no interactive picker.
        if approves.is_empty() && !opts.all {
            note(
                "bun pm trust requires package names. Run `bun pm untrusted` to see\n  \
                 which packages are pending, then pass them explicitly:\n    \
                 vp pm approve-builds <pkg> [<pkg>...]\n    \
                 vp pm approve-builds --all",
            );
            return Ok(ExitStatus::default());
        }

        let mut args = vec!["pm".to_string(), "trust".into()];
        if opts.all {
            args.push("--all".into());
        }
        args.extend(approves.iter().map(|s| s.to_string()));
        self.run_raw("bun", &args, cwd).await
    }
}
```

**File**: `crates/vite_package_manager/src/commands/mod.rs`

```rust
pub mod add;
mod install;
pub mod remove;
pub mod update;
pub mod link;
pub mod unlink;
pub mod dedupe;
pub mod why;
pub mod outdated;
pub mod approve_builds;  // <- add this
// pub mod pm;             // (future; from pm-command-group RFC)
```

#### 3. CLI Implementation

**File**: `crates/vite_task/src/approve_builds.rs` (new file)

```rust
use vite_error::Error;
use vite_package_manager::{
    PackageManager,
    commands::approve_builds::ApproveBuildsOptions,
};
use vite_path::AbsolutePathBuf;
use vite_workspace::Workspace;

pub struct ApproveBuildsCommand {
    workspace_root: AbsolutePathBuf,
}

impl ApproveBuildsCommand {
    pub fn new(workspace_root: AbsolutePathBuf) -> Self {
        Self { workspace_root }
    }

    pub async fn execute(self, packages: Vec<String>, all: bool) -> Result<(), Error> {
        let package_manager = PackageManager::builder(&self.workspace_root).build().await?;
        let workspace = Workspace::partial_load(self.workspace_root.clone())?;

        let status = package_manager
            .run_approve_builds(
                ApproveBuildsOptions { packages: &packages, all },
                &workspace.root,
            )
            .await?;

        if !status.success() {
            return Err(Error::CommandFailed {
                command: "pm approve-builds".into(),
                exit_code: status.code(),
            });
        }
        workspace.unload().await?;
        Ok(())
    }
}
```

## Design Decisions

### 1. Mirror pnpm's documented surface exactly

**Decision**: The Vite+ command exposes only what `pnpm approve-builds` documents — positional packages (with `!pkg` deny prefix) and `--all`. Bun's additional commands (`bun pm untrusted`, `bun pm default-trusted`) are **not** folded in as flags.

**Rationale**: pnpm and bun share only the "approve these packages" operation. Adding `--list`, `--default-trusted`, `-y`, or `-g` would either invent flags that don't exist in pnpm's documented surface or paper over bun's separate-command model. If `vp pm untrusted` and `vp pm default-trusted` are wanted later, they should be their own sibling subcommands under `pm` (mirroring bun) — that's a follow-up RFC, not creeping scope here.

### 2. Do not normalize storage

**Decision**: Vite+ does **not** rewrite `pnpm-workspace.yaml ↔ package.json#trustedDependencies` into a single shared file.

**Rationale**:

- The two formats encode different semantics (`allowBuilds: { core-js: false }` has no bun analog).
- Round-tripping between them on every command would mutate files the user doesn't expect.
- Migrations between package managers are rare; on-demand conversion (e.g., a future `vp migrate` step) is the right place for that translation, not the day-to-day `approve-builds` command.
- The pnpm/aube-only `!pkg` deny syntax stays meaningful and isn't silently lost.

### 3. `!pkg` deny syntax: pnpm/aube-only, surface a warning

**Decision**: Accept `!pkg` in positional args; for bun, print a `warn` naming the affected packages and continue with the approved ones.

**Rationale**:

- Silently dropping `!core-js` would leave users believing they had denied a package when they hadn't.
- Erroring out would break `vp pm approve-builds esbuild !core-js` for a developer who copied the command from a pnpm tutorial but happens to be on bun for one repo.
- The warning names the dropped packages so the divergence is auditable.

### 4. npm < 11.16.0 / yarn: warn + exit 0

**Decision**: Print a `warn` and return exit code 0 when the package manager has no native approval command to forward to: yarn (all versions) and npm < 11.16.0.

> **Update (npm ≥ 11.16.0):** npm shipped `approve-scripts` / `deny-scripts` ([npm/cli#9360](https://github.com/npm/cli/pull/9360)). Vite+ now forwards to them (see [Command Mapping](#command-mapping)). As with pnpm and bun, that means a real command runs and can exit non-zero, and a mixed approve+deny invocation is rejected (`Error::InvalidArgument`). The warn + exit-0 fallback below applies only to npm < 11.16.0.

**Rationale**:

- **npm < 11.16.0** runs lifecycle scripts by default and has no approval command — the warn points at upgrading to npm ≥ 11.16.0 or at how to _restrict_ scripts (`ignore-scripts=true`).
- **yarn (Berry)** blocks third-party build scripts by default; the per-package opt-in lives in `package.json` (`dependenciesMeta.<pkg>.built: true`). We `warn` pointing at that field rather than performing the edit ourselves — staying within the RFC's intentionally-tight scope.
- Both fallback surfaces use `warn` (not `note`) for consistency: the user invoked `approve-builds` and the requested action could not be completed on this PM, so they need a visible signal and a manual workaround.
- Exit 0 on the fallback lets CI scripts that conditionally run `vp pm approve-builds --all` work across heterogeneous repos where the PM has no approval command.
- Exit non-zero on the fallback (the alternative) would break monorepo orchestration scripts and demand per-PM conditionals. (Once a PM _does_ have a command — pnpm, bun, npm ≥ 11.16.0 — Vite+ runs it and surfaces its real exit code, same as any forwarded command.)

### 5. No-args on bun: note + exit 0

**Decision**: When `vp pm approve-builds` is invoked with no args (and no `--all`) on bun, print a `note` and exit 0 rather than building a Vite+-owned interactive picker.

**Rationale**:

- Implementing a picker requires parsing `bun pm untrusted` output and reusing the prompts module — meaningful work that should land in its own RFC if/when wanted.
- The current behavior keeps this RFC's surface area minimal and faithful to pnpm's documented flag set.

### 6. No caching

**Decision**: Do not cache approve-builds results.

**Rationale**: This command mutates configuration files; caching would be incorrect.

## Error Handling

### No package manager detected

```
$ vp pm approve-builds
error  No package manager detected.
       Please run one of:
         - vp install (to set up package manager)
         - Add a `packageManager` field to package.json
```

### `--all` on pnpm < v10.32.0

```
$ vp pm approve-builds --all
Detected package manager: pnpm@10.20.0
error  `--all` requires pnpm ≥ 10.32.0. Upgrade pnpm or pass package names explicitly.
```

### Deny syntax against bun

```
$ vp pm approve-builds esbuild !core-js
Detected package manager: bun@1.3.0
warn  bun does not support denylisting build scripts. Packages outside
      `trustedDependencies` in package.json are already denied by default.
      Skipping: core-js
Running: bun pm trust esbuild
✔ Trusted 1 package
```

### Underlying command failed

```
$ vp pm approve-builds esbuild
Detected package manager: pnpm@10.32.0
Running: pnpm approve-builds esbuild
ERR_PNPM_CONFIG_WRITE_FAILED: cannot write pnpm-workspace.yaml
exit code: 1
```

Exit code is propagated.

## User Experience

### Interactive approval (pnpm)

```
$ vp pm approve-builds
Detected package manager: pnpm@10.32.0
Running: pnpm approve-builds

? Choose which packages to build (Press <space> to select, <a> to toggle all, <i> to invert selection)
❯◯ @biomejs/biome
 ◯ esbuild
 ◯ fsevents
 ◯ sharp

✔ Updated pnpm-workspace.yaml (allowBuilds)
```

### Direct approval (bun)

```
$ vp pm approve-builds esbuild fsevents
Detected package manager: bun@1.3.0
Running: bun pm trust esbuild fsevents
✔ Updated package.json (trustedDependencies)
```

### Bulk approval

```
$ vp pm approve-builds --all
Detected package manager: bun@1.3.0
Running: bun pm trust --all
✔ Trusted 4 packages
```

### No-args on bun

```
$ vp pm approve-builds
Detected package manager: bun@1.3.0
note  bun pm trust requires package names. Run `bun pm untrusted` to see
      which packages are pending, then pass them explicitly:
        vp pm approve-builds <pkg> [<pkg>...]
        vp pm approve-builds --all
```

### No-op on npm

```
$ vp pm approve-builds
Detected package manager: npm@11.0.0
warn  npm runs lifecycle scripts by default. To restrict them, set
      `ignore-scripts=true` in .npmrc and rebuild approved packages with
      `vp pm rebuild <package>`.
```

### No-op on yarn

```
$ vp pm approve-builds esbuild
Detected package manager: yarn@4.0.0
warn  yarn does not run third-party build scripts by default. To allow a
      package, set `dependenciesMeta["<package>"].built: true` in package.json.
```

## Alternative Designs Considered

### Alternative 1: Separate `vp pm trust` / `vp pm untrusted` / `vp pm allow-build`

```bash
vp pm trust esbuild
vp pm untrusted
vp pm allow-build esbuild
```

**Rejected because:**

- Mirrors PM-specific vocabulary instead of unifying it.
- Triples the surface area users have to learn.
- The single-command shape matches existing Vite+ conventions.

### Alternative 2: Normalize all approvals into a Vite+-owned file (e.g., `vite-plus.json`)

```json
{ "approvedBuilds": ["esbuild", "fsevents"] }
```

**Rejected because:**

- Forces Vite+ to re-implement script execution gating (today owned by pnpm/bun).
- Creates two sources of truth (`vite-plus.json` and the PM's own file) — drift inevitable.
- Loses pnpm's allow/deny distinction and version-specific entries (`nx@21.6.4 || 21.6.5: true`).

### Alternative 3: Always shell out raw

```bash
vp pm approve-builds -- pnpm approve-builds --all
```

**Rejected because:**

- Defeats the purpose of a unified command.
- Forces the user to know which PM is active.
- No bun parity (bun has no `approve-builds` to delegate to).

### Alternative 4: Auto-approve everything on install

**Rejected because:**

- Defeats the supply-chain protection that pnpm/bun designed the gating to provide.
- Would be a security regression compared to running pnpm/bun directly.

### Alternative 5: Bundle bun's untrusted/default-trusted as flags

```bash
vp pm approve-builds --list
vp pm approve-builds --default-trusted
```

**Rejected because:**

- These flags don't exist in pnpm's documented surface; adding them invents a unified vocabulary that no PM actually uses.
- bun models them as separate commands (`bun pm untrusted`, `bun pm default-trusted`); the cleaner Vite+ mirror is also separate subcommands.
- Out of scope for this RFC; see [Future Enhancements](#future-enhancements).

## Implementation Plan

### Phase 1: Core plumbing

1. Add `ApproveBuilds` variant to `PmCommands` in `crates/vite_task/src/lib.rs`.
2. Create `crates/vite_package_manager/src/commands/approve_builds.rs` with the pnpm + bun adapters.
3. Wire pass-through for pnpm (`approve-builds`, `approve-builds <pkg>`, `approve-builds <pkg> !<pkg>`, `--all`).
4. Wire `bun pm trust` (positionals + `--all`), with `!pkg` filter + warning.
5. Wire the npm/yarn warning path (exit 0).
6. Wire the bun no-args `note` path.

### Phase 2: Version-gating for pnpm `--all`

1. Detect pnpm version and reject `--all` against < v10.32.0 with the usage hint.

### Phase 3: Tests + snap tests

1. Unit tests for command resolution (per PM × flag matrix).
2. Snap tests covering each PM in `packages/cli/snap-tests/`.

### Phase 4: Docs

1. Update `vp pm --help` to list the new subcommand.
2. Add a row to the [pm-command-group RFC](./pm-command-group.md) compatibility matrix.
3. Add `vp pm approve-builds` to user-facing CLI docs.

## Testing Strategy

### Unit tests

```rust
#[test]
fn pnpm_basic_approve() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm).with_version("10.32.0");
    let opts = ApproveBuildsOptions { packages: &vec!["esbuild".into()], all: false };
    let cmd = pm.resolve_approve_builds(&opts);
    assert_eq!(cmd.bin, "pnpm");
    assert_eq!(cmd.args, vec!["approve-builds", "esbuild"]);
}

#[test]
fn pnpm_all_flag() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm).with_version("10.32.0");
    let opts = ApproveBuildsOptions { packages: &vec![], all: true };
    let cmd = pm.resolve_approve_builds(&opts);
    assert_eq!(cmd.args, vec!["approve-builds", "--all"]);
}

#[test]
fn pnpm_all_rejected_below_v10_32() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm).with_version("10.20.0");
    let opts = ApproveBuildsOptions { packages: &vec![], all: true };
    assert!(pm.resolve_approve_builds(&opts).is_err());
}

#[test]
fn pnpm_passes_deny_syntax_through() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm).with_version("10.32.0");
    let opts = ApproveBuildsOptions {
        packages: &vec!["esbuild".into(), "!core-js".into()],
        all: false,
    };
    let cmd = pm.resolve_approve_builds(&opts);
    assert_eq!(cmd.args, vec!["approve-builds", "esbuild", "!core-js"]);
}

#[test]
fn bun_deny_syntax_filtered_with_warning() {
    let pm = PackageManager::mock(PackageManagerType::Bun);
    let opts = ApproveBuildsOptions {
        packages: &vec!["esbuild".into(), "!core-js".into()],
        all: false,
    };
    let cmd = pm.resolve_approve_builds(&opts);
    assert_eq!(cmd.args, vec!["pm", "trust", "esbuild"]);
    assert!(cmd.warnings.iter().any(|w| w.contains("core-js")));
}

#[test]
fn bun_all_flag_passes_through() {
    let pm = PackageManager::mock(PackageManagerType::Bun);
    let opts = ApproveBuildsOptions { packages: &vec![], all: true };
    let cmd = pm.resolve_approve_builds(&opts);
    assert_eq!(cmd.args, vec!["pm", "trust", "--all"]);
}

#[test]
fn bun_no_args_emits_note() {
    let pm = PackageManager::mock(PackageManagerType::Bun);
    let opts = ApproveBuildsOptions { packages: &vec![], all: false };
    let result = pm.resolve_approve_builds(&opts);
    assert!(result.no_op);
    assert!(result.notes.iter().any(|n| n.contains("bun pm untrusted")));
}

#[test]
fn npm_warns_and_exits_zero() {
    let pm = PackageManager::mock(PackageManagerType::Npm);
    let result = pm.resolve_approve_builds(&Default::default());
    assert!(result.no_op);
    assert!(result.warnings.iter().any(|w| w.contains("ignore-scripts=true")));
}
```

### Snap tests

Add fixtures under `packages/cli/snap-tests/pm-approve-builds-{pnpm,bun,npm,yarn}` covering:

- No-op invocation (warning for npm/yarn, note for bun).
- `--all` against bun.
- `--all` against pnpm.
- `esbuild !core-js` against bun (asserts the deny warning text).
- `esbuild !core-js` against pnpm (asserts pass-through).

## CLI Help Output

```
$ vp pm approve-builds --help
Approve dependency lifecycle scripts (install/postinstall) to run

Usage: vp pm approve-builds [OPTIONS] [PACKAGES]...

Arguments:
    [PACKAGES]...  Packages to approve. Prefix with `!` to deny (pnpm/aube only).
                                 Omit all positionals to launch interactive mode (pnpm/aube only).

Options:
      --all   Approve every package currently pending approval
  -h, --help  Print help

Examples:
    vp pm approve-builds                       # interactive prompt (pnpm/aube)
  vp pm approve-builds esbuild fsevents      # approve specific packages
    vp pm approve-builds esbuild !core-js      # approve esbuild, deny core-js (pnpm/aube only)
  vp pm approve-builds --all                 # approve every pending package
```

## Package Manager Compatibility

| Capability            | pnpm                                   | npm     | yarn@1                                          | yarn@2+                                         | bun                                     |
| --------------------- | -------------------------------------- | ------- | ----------------------------------------------- | ----------------------------------------------- | --------------------------------------- |
| Interactive (no args) | ✅ native                              | ❌ warn | ❌ warn                                         | ❌ warn                                         | ❌ note (no picker)                     |
| Approve by name       | ✅ `pnpm approve-builds <pkg>`         | ❌ warn | ❌ warn                                         | ❌ warn                                         | ✅ `bun pm trust <pkg>`                 |
| Deny by name (`!pkg`) | ✅ `pnpm approve-builds !<pkg>`        | ❌ warn | ❌ warn                                         | ❌ warn                                         | ⚠️ warn (model mismatch)                |
| `--all`               | ✅ ≥ v10.32.0 (error on older)         | ❌ warn | ❌ warn                                         | ❌ warn                                         | ✅ `bun pm trust --all`                 |
| Storage location      | `pnpm-workspace.yaml` → `allowBuilds:` | n/a     | `package.json` → `dependenciesMeta.<pkg>.built` | `package.json` → `dependenciesMeta.<pkg>.built` | `package.json` → `trustedDependencies:` |

## Future Enhancements

### 1. `vp pm untrusted`

Mirror `bun pm untrusted` as a sibling subcommand. For pnpm, derive the pending list from `pnpm install --lockfile-only --reporter=ndjson` (filtering `ignored-scripts` events). For npm/yarn, warn-and-exit-0.

### 2. `vp pm default-trusted`

Mirror `bun pm default-trusted` as a sibling subcommand. For pnpm/npm/yarn, print a `note` explaining no such list exists.

### 3. Cross-PM migration helper

`vp migrate approve-builds` could read `allowBuilds:` from `pnpm-workspace.yaml` and emit a `trustedDependencies:` list for `package.json` (or vice versa).

### 4. CI confirmation gate / `--yes`

If user feedback shows `--all` is being run unintentionally in scripts, revisit a confirmation prompt + `-y` opt-out. Not in scope today.

### 5. Audit integration

`vp pm audit` (already in the [pm command group RFC](./pm-command-group.md)) could surface "this package is currently approved to run install scripts" alongside its CVE list.

## Security Considerations

1. **No silent denylist drops**: bun users who type `!core-js` see a warning instead of having it silently ignored.
2. **No storage normalization**: Vite+ does not introduce a new file format that could become a parallel source of truth for which scripts may run.
3. **Pass-through preserves PM-native auditing**: pnpm and bun continue to own the actual gating; Vite+ is a thin orchestration layer.

## Backward Compatibility

This is a new subcommand under the (also new) `vp pm` command group. No breaking changes.

- Independent of [pm-command-group.md](./pm-command-group.md) — can ship before, after, or alongside it.
- No changes to existing commands.
- No changes to cache or task graph behavior.

## Real-World Usage Examples

### Approving after `vp install`

```bash
vp install
# pnpm reports 4 packages with ignored build scripts
vp pm approve-builds esbuild fsevents sharp
vp install   # re-run to actually execute the approved scripts
```

### Bulk approval in CI

```yaml
- run: vp install
- run: vp pm approve-builds --all
```

### Mixed approve/deny on pnpm

```bash
vp pm approve-builds esbuild fsevents !core-js !some-tracker
```

## Conclusion

This RFC adds `vp pm approve-builds` as a focused mirror of `pnpm approve-builds`, adapted to `bun pm trust`. The surface area is intentionally tight:

- ✅ Positional `[PACKAGES...]` with pnpm's `!pkg` deny prefix
- ✅ `--all` flag (matches pnpm v10.32.0+ and bun)
- ✅ pnpm interactive mode passes through; bun no-args mode emits a `note`
- ✅ `!pkg` deny syntax is preserved for pnpm; warned (not silently dropped) for bun
- ✅ npm and yarn both warn (pointing at `ignore-scripts` and `dependenciesMeta.<pkg>.built` respectively) and exit 0, keeping CI scripts portable
- ✅ No new storage format — each PM keeps owning its own configuration
- ✅ Auxiliary bun commands (`untrusted`, `default-trusted`) deferred to follow-up sibling subcommands rather than folded in as flags
