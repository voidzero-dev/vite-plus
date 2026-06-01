# RFC: Vite+ `vp pm stage` Command

- Issue: [#1674](https://github.com/voidzero-dev/vite-plus/issues/1674)
- Status: Implemented in [#1715](https://github.com/voidzero-dev/vite-plus/pull/1715)

## Summary

Add `vp pm stage` to the `vp pm` command group. Staged publishing is npm's
new security workflow that inserts an approval step between uploading a package
and making it public: a package is uploaded to a staging area (no 2FA), and a
maintainer later approves or rejects it from a trusted device (2FA). `vp pm
stage` exposes that workflow through the unified `vp` surface and adapts to the
detected package manager (pnpm / npm / yarn / bun).

The feature ships as a structured subcommand group mirroring the existing
`vp pm dist-tag` / `vp pm owner` / `vp pm token` subcommands:

```bash
vp pm stage publish [TARBALL|FOLDER]   # Upload a package to the staging area (no 2FA)
vp pm stage list [PACKAGE_SPEC]        # List staged versions
vp pm stage view <STAGE_ID>            # Show details about a staged version
vp pm stage download <STAGE_ID>        # Download the staged tarball for inspection
vp pm stage approve <STAGE_ID>         # Promote a staged version to live (2FA)
vp pm stage reject <STAGE_ID>          # Discard a staged version (2FA)
```

## Motivation

npm shipped **staged publishing** (npm CLI ≥ 11.15.0, Node ≥ 22.14.0) as a
defense against supply-chain attacks: CI can upload a build to a staging area
without holding 2FA credentials, and a human approves it later. pnpm is adding
an equivalent `pnpm stage` command, and yarn berry already exposes it through
`yarn npm publish --staged` + `yarn npm stage …`.

Because Vite+ already normalizes the rest of the publishing surface
(`vp pm publish`, `vp pm dist-tag`, `vp pm owner`, …), the staging workflow
should be reachable the same way instead of forcing users to drop down to a
raw `pnpm`/`npm`/`yarn` invocation. Issue #1674 asks specifically for a
`vp pm stage` passthrough that "delegates correctly to the configured package
manager" and stays "aligned with the rest of the `vp pm <subcommand>` surface."

### Background: how staged publishing works

| Step       | Command                                | 2FA?   | Notes                                                                                   |
| ---------- | -------------------------------------- | ------ | --------------------------------------------------------------------------------------- |
| 1. Stage   | `npm stage publish`                    | ❌ No  | Uploads the tarball to a pending staging area. Safe for CI / trusted publishers (OIDC). |
| 2. Review  | `npm stage list` / `view` / `download` | ❌ No  | Inspect what was staged (also visible on npmjs.com "Staged Packages" tab).              |
| 3. Approve | `npm stage approve <id>`               | ✅ Yes | Promotes to the live registry.                                                          |
| 3'. Reject | `npm stage reject <id>`                | ✅ Yes | Discards the staged version.                                                            |

#### Minimum versions

The version floors differ per package manager (and npm additionally gates on
the Node.js version), which is a key input to the version-gating decision below.

| PM           | Minimum for staged publishing                                                                      |
| ------------ | -------------------------------------------------------------------------------------------------- |
| npm          | CLI ≥ 11.15.0 **and** Node ≥ 22.14.0                                                               |
| pnpm         | pnpm ≥ 11.3.0 (`pnpm stage` was "Added in: v11.3.0"; no separate Node floor documented)            |
| yarn ≥ 2     | via the npm plugin (`yarn npm publish --staged`); registry-side, no separate yarn floor documented |
| yarn 1 / bun | unsupported                                                                                        |

References:

- npm: <https://docs.npmjs.com/staged-publishing>
- pnpm: <https://pnpm.io/cli/stage> (added in pnpm 11.3, see <https://pnpm.io/blog/releases/11.3>)
- yarn (berry): <https://yarnpkg.com/cli/npm/publish> (`--staged` flag) and `yarn npm stage …`
- bun: no staged-publishing support today (`bun publish` only)

## Proposed Solution

### Command surface

`vp pm stage` is a subcommand group; a subcommand is required (bare `vp pm
stage` prints help, matching `vp pm dist-tag`).

```bash
vp pm stage <SUBCOMMAND>

Subcommands:
  publish    Stage a package for publishing (no 2FA)
  list       List staged versions (alias: ls)
  view       Show details about a staged version
  download   Download the staged tarball for inspection
  approve    Promote a staged version to the live registry (2FA)
  reject     Discard a staged version (2FA)
```

**Examples:**

```bash
# Stage the current package (CI-friendly, no 2FA)
vp pm stage publish
vp pm stage publish --tag next --access public

# Stage a prebuilt tarball
vp pm stage publish ./my-pkg-1.2.3.tgz

# Stage every publishable workspace package (pnpm)
vp pm stage publish -r
vp pm stage publish --filter "@scope/*"

# Review what is staged
vp pm stage list
vp pm stage list my-pkg --json
vp pm stage view 1a2b3c4d
vp pm stage download 1a2b3c4d

# Approve / reject (requires 2FA on a trusted device)
vp pm stage approve 1a2b3c4d
vp pm stage approve 1a2b3c4d --otp 123456
vp pm stage reject 1a2b3c4d
```

### Flags

Following the existing `vp pm` convention, only the common, stable flags are
modeled; anything else flows through the trailing `-- <args>` escape hatch
(`#[arg(last = true, allow_hyphen_values = true)]`).

| Subcommand | Positional          | Modeled flags                                                                                                                                  |
| ---------- | ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| `publish`  | `[TARBALL\|FOLDER]` | `--tag`, `--access <public\|restricted>`, `--otp`, `--dry-run`, `--json`, `-r/--recursive`, `--filter <pattern>`, `--provenance`, `--registry` |
| `list`     | `[PACKAGE_SPEC]`    | `--json`, `--registry`                                                                                                                         |
| `view`     | `<STAGE_ID>`        | `--json`, `--registry`                                                                                                                         |
| `download` | `<STAGE_ID>`        | `--registry`                                                                                                                                   |
| `approve`  | `<STAGE_ID>`        | `--otp`, `--registry`                                                                                                                          |
| `reject`   | `<STAGE_ID>`        | `--otp`, `--registry`                                                                                                                          |

`stage publish` intentionally reuses the option vocabulary of the existing
`vp pm publish` command so the two read consistently.

### Command mapping

This is the core of the RFC. The mapping is driven by what each package
manager actually supports.

| `vp pm stage …` | pnpm                       | npm                       | yarn ≥ 2 (berry)               | yarn 1 (classic)               | bun                            |
| --------------- | -------------------------- | ------------------------- | ------------------------------ | ------------------------------ | ------------------------------ |
| `publish [t]`   | `pnpm stage publish [t]`   | `npm stage publish [t]`   | `yarn npm publish --staged`    | ⚠️ → `npm stage publish`       | ⚠️ → `npm stage publish`       |
| `list [spec]`   | `pnpm stage list [spec]`   | `npm stage list [spec]`   | `yarn npm stage list [spec]`   | ⚠️ → `npm stage list`          | ⚠️ → `npm stage list`          |
| `view <id>`     | `pnpm stage view <id>`     | `npm stage view <id>`     | ⚠️ → `npm stage view <id>`     | ⚠️ → `npm stage view <id>`     | ⚠️ → `npm stage view <id>`     |
| `download <id>` | `pnpm stage download <id>` | `npm stage download <id>` | ⚠️ → `npm stage download <id>` | ⚠️ → `npm stage download <id>` | ⚠️ → `npm stage download <id>` |
| `approve <id>`  | `pnpm stage approve <id>`  | `npm stage approve <id>`  | `yarn npm stage approve <id>`  | ⚠️ → `npm stage approve`       | ⚠️ → `npm stage approve`       |
| `reject <id>`   | `pnpm stage reject <id>`   | `npm stage reject <id>`   | `yarn npm stage reject <id>`   | ⚠️ → `npm stage reject`        | ⚠️ → `npm stage reject`        |

⚠️ = print a `output::warn` line, then fall back to `npm stage …` (consistent
with how `vp pm dist-tag`, `vp pm fund`, `vp pm token` already fall back to npm
for registry-only features).

### ⚠️ Critical: `yarn stage` is NOT staged publishing

Yarn berry has a built-in command literally called **`yarn stage`** (from
`plugin-stage`), but it is **completely unrelated** to npm staged publishing:
it stages Yarn-related files (`package.json`, `.yarnrc.yml`, linker output) into
your **git/mercurial** staging area and can auto-create a release commit
(<https://yarnpkg.com/cli/stage>).

> `vp pm stage` must **never** resolve to `yarn stage`. Doing so would touch the
> user's VCS index / create commits instead of publishing.

For yarn, npm staged publishing is reached through the **npm plugin**:

- Stage: `yarn npm publish --staged`
- Manage: `yarn npm stage list` / `yarn npm stage approve` / `yarn npm stage reject`

yarn berry exposes only `list` / `approve` / `reject` under `yarn npm stage`
(no `view` / `download`), so those two fall back to `npm stage …`.

### Per-package-manager behavior

#### pnpm

Direct passthrough: `pnpm stage <sub> [args]`. pnpm mirrors npm's subcommand
set (`publish`, `list`, `view`, `download`, `approve`, `reject`) and adds
`-r/--filter` for monorepos. `--otp` is accepted for `approve`/`reject`.

#### npm

Direct passthrough: `npm stage <sub> [args]`. This is the canonical
implementation; every other PM's gaps fall back here.

#### yarn ≥ 2 (berry)

- `publish` → `yarn npm publish --staged` (with the target dir/tarball and
  `--tag`/`--access`/`--otp`/`--provenance` forwarded).
- `list` / `approve` / `reject` → `yarn npm stage <sub>`.
- `view` / `download` → not supported by yarn; warn and fall back to
  `npm stage <sub>` (registry-side operation, same data).

#### yarn 1 (classic)

No staged publishing. yarn classic already delegates publishing to npm in this
repo (`publish.rs`), so all `stage` subcommands warn and fall back to
`npm stage <sub>`.

#### bun

No staged publishing and no `bun stage`. Warn and fall back to `npm stage <sub>`,
consistent with `vp pm dist-tag`/`fund`/`token` on bun.

## Implementation Architecture

The current code lives in `crates/vite_pm_cli/` (clap surface + dispatch) and
`crates/vite_install/src/commands/` (per-command resolvers). The
`PackageManagerCommand`/`PmCommands` enums are shared by both the global CLI and
the local NAPI binding via `#[command(flatten)]`, so adding a variant surfaces
in both CLIs automatically.

### 1. Clap surface: `crates/vite_pm_cli/src/cli.rs`

Add a `Stage` variant to `PmCommands` and a `StageCommands` subcommand enum
(modeled on the existing `DistTagCommands`):

```rust
// in enum PmCommands
/// Stage a package for publishing (npm staged publishing workflow)
#[command(subcommand)]
Stage(StageCommands),
```

```rust
/// Staged-publishing subcommands.
#[derive(Subcommand, Debug, Clone)]
pub enum StageCommands {
    /// Stage a package for publishing (no 2FA)
    Publish {
        /// Tarball or folder to stage
        #[arg(value_name = "TARBALL|FOLDER")]
        target: Option<String>,
        #[arg(long)] tag: Option<String>,
        #[arg(long)] access: Option<String>,
        #[arg(long, value_name = "OTP")] otp: Option<String>,
        #[arg(long)] dry_run: bool,
        #[arg(long)] json: bool,
        #[arg(short = 'r', long)] recursive: bool,
        #[arg(long, value_name = "PATTERN")] filter: Option<Vec<String>>,
        #[arg(long)] provenance: bool,
        #[arg(long, value_name = "URL")] registry: Option<String>,
        #[arg(last = true, allow_hyphen_values = true)] pass_through_args: Option<Vec<String>>,
    },
    /// List staged versions
    #[command(visible_alias = "ls")]
    List {
        package: Option<String>,
        #[arg(long)] json: bool,
        #[arg(long, value_name = "URL")] registry: Option<String>,
        #[arg(last = true, allow_hyphen_values = true)] pass_through_args: Option<Vec<String>>,
    },
    /// Show details about a staged version
    View {
        stage_id: String,
        #[arg(long)] json: bool,
        #[arg(long, value_name = "URL")] registry: Option<String>,
        #[arg(last = true, allow_hyphen_values = true)] pass_through_args: Option<Vec<String>>,
    },
    /// Download the staged tarball for inspection
    Download {
        stage_id: String,
        #[arg(long, value_name = "URL")] registry: Option<String>,
        #[arg(last = true, allow_hyphen_values = true)] pass_through_args: Option<Vec<String>>,
    },
    /// Promote a staged version to the live registry (2FA)
    Approve {
        stage_id: String,
        #[arg(long, value_name = "OTP")] otp: Option<String>,
        #[arg(long, value_name = "URL")] registry: Option<String>,
        #[arg(last = true, allow_hyphen_values = true)] pass_through_args: Option<Vec<String>>,
    },
    /// Discard a staged version (2FA)
    Reject {
        stage_id: String,
        #[arg(long, value_name = "OTP")] otp: Option<String>,
        #[arg(long, value_name = "URL")] registry: Option<String>,
        #[arg(last = true, allow_hyphen_values = true)] pass_through_args: Option<Vec<String>>,
    },
}
```

Extend `PmCommands::is_quiet_or_machine_readable` so `--json` on
`stage publish`/`list`/`view` suppresses decorative output:

```rust
Self::Stage(sub) => sub.is_quiet_or_machine_readable(),
```

with a matching `impl StageCommands` returning `*json` for `Publish`/`List`/`View`.

### 2. Resolver: `crates/vite_install/src/commands/stage.rs` (new)

Mirror `dist_tag.rs`: an owned `StageSubcommand` enum, a `StageCommandOptions`
struct, and `resolve_stage_command` / `run_stage_command`:

```rust
pub enum StageSubcommand {
    Publish { target: Option<String>, tag: Option<String>, access: Option<String>,
              otp: Option<String>, dry_run: bool, json: bool, recursive: bool,
              filters: Option<Vec<String>>, provenance: bool },
    List { package: Option<String>, json: bool },
    View { stage_id: String, json: bool },
    Download { stage_id: String },
    Approve { stage_id: String, otp: Option<String> },
    Reject { stage_id: String, otp: Option<String> },
}

pub struct StageCommandOptions<'a> {
    pub subcommand: StageSubcommand,
    pub registry: Option<&'a str>,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    pub async fn run_stage_command(&self, options: &StageCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>) -> Result<ExitStatus, Error> { /* run_command */ }

    pub fn resolve_stage_command(&self, options: &StageCommandOptions) -> ResolveCommandResult {
        // match self.client {
        //   Pnpm                       => bin "pnpm", ["stage", <sub>, ...]
        //   Npm                        => bin "npm",  ["stage", <sub>, ...]
        //   Yarn (berry) Publish       => bin "yarn", ["npm", "publish", "--staged", ...]
        //   Yarn (berry) List/Approve/Reject => bin "yarn", ["npm", "stage", <sub>, ...]
        //   Yarn (berry) View/Download => warn + bin "npm", ["stage", <sub>, ...]
        //   Yarn (classic) / Bun       => warn + bin "npm", ["stage", <sub>, ...]
        // }
    }
}
```

Register the module in `crates/vite_install/src/commands/mod.rs`:

```rust
pub mod stage;
```

### 3. Handler: `crates/vite_pm_cli/src/handlers.rs`

Import `stage::{StageCommandOptions, StageSubcommand}` and add a `PmCommands::Stage`
arm to `run_pm_subcommand`, converting the clap `StageCommands` into the owned
`StageSubcommand` (same shape as the existing `DistTag`/`Owner`/`Token` arms).

Dispatch target selection (the `needs_project` block at the top of
`run_pm_subcommand`): only `stage publish` reads the local package and needs a
real project; `list`/`view`/`download`/`approve`/`reject` are registry-only and
can run against `build_package_manager_or_npm_default` (npm fallback when there
is no `package.json`), exactly like `vp pm view` / `vp pm dist-tag` today:

```rust
let needs_project = matches!(command,
    // …existing…
    | PmCommands::Stage(StageCommands::Publish { .. })
);
```

No changes are needed in `dispatch.rs`; `PackageManagerCommand::Pm` already
forwards to `handlers::run_pm_subcommand`.

### 4. Wiring summary

```
vp pm stage <sub>
  └─ cli.rs            PmCommands::Stage(StageCommands)            (shared, both CLIs)
       └─ handlers.rs  run_pm_subcommand → StageCommandOptions
            └─ stage.rs resolve_stage_command → run_command(<pm>, args)
```

## Design Decisions

1. **Structured subcommands, not a free-string passthrough.** `vp pm cache`
   takes a free `subcommand: String`, but the publishing-adjacent commands
   (`dist-tag`, `owner`, `token`, `config`) are modeled as typed subcommand
   enums. `stage` has a small, well-defined, stable subcommand set, so typed
   modeling gives proper `--help`, tab-completion, and per-subcommand flags
   while staying aligned with its neighbors. (Thin passthrough is Alternative 1.)

2. **yarn uses its native npm plugin, never `yarn stage`.** As covered above,
   `yarn stage` is git staging. yarn berry's real staged-publishing path is
   `yarn npm publish --staged` + `yarn npm stage …`. This respects the project's
   yarn auth/registry config (`.yarnrc.yml`) rather than assuming npm is
   authenticated. (See Open Question 1 for the alternative of always delegating
   to npm, which would match `publish.rs`.)

3. **bun and yarn-classic fall back to `npm stage` with a warning.** Staged
   publishing is a registry-side feature; npm is the reference client and the
   repo already routes registry-only features (`dist-tag`, `fund`, `token`,
   `search`, `ping`) through npm. Falling back keeps the workflow usable instead
   of hard-failing.

4. **No version gating in vp.** The floors differ per tool: npm needs CLI
   ≥ 11.15.0 **and** Node ≥ 22.14.0, pnpm needs ≥ 11.3.0, yarn routes through its
   npm plugin, and npm uniquely also gates on Node. Rather than tracking and
   chasing these across four package managers as the feature stabilizes, pass
   through and let the underlying tool emit its own authoritative,
   version-specific error. (`approve-builds` does gate, but that gate guards a
   destructive flag shape; staging is too new/fast-moving to pin. See Open
   Question 2.)

5. **No caching.** Staging mutates registry state or queries live state; results
   must never be cached.

## Open Questions (please weigh in during review)

1. **yarn strategy: native plugin vs. npm delegation.**
   - **(A) Recommended:** map to `yarn npm publish --staged` + `yarn npm stage …`
     (uses yarn's own auth/registry; most correct for yarn projects).
   - **(B) Simpler/consistent:** delegate all yarn `stage` to `npm stage …`,
     matching the existing `publish.rs` (yarn → npm). Lower complexity, but
     `npm` may not be authenticated in a yarn-managed project.

2. **Version gating.** Recommended: none (pass through, let the PM error),
   especially since the floors differ (npm ≥ 11.15.0 + Node ≥ 22.14.0; pnpm
   ≥ 11.3.0). Do you want a friendly pre-check instead?

3. **`view` / `download` for yarn.** yarn berry has no equivalent. Recommended:
   warn + fall back to `npm stage view/download`. Alternative: treat as
   unsupported and error.

4. **`--registry` modeling.** Worth a first-class flag, or rely solely on
   `-- --registry <url>` passthrough? (Recommended: model it, since the npm-fallback
   paths benefit from threading it explicitly.)

## Error Handling

```bash
# Underlying tool too old (passthrough surfaces the real error)
$ vp pm stage publish
npm error staged publishing requires npm ≥ 11.15.0
# vp exits non-zero with the tool's message

# bun / yarn-classic
$ vp pm stage approve 1a2b3c4d
warning: bun does not support staged publishing, falling back to npm stage
…

# Missing required stage id
$ vp pm stage approve
error: the following required arguments were not provided:
  <STAGE_ID>
```

## Testing Strategy

### Unit tests (`crates/vite_install/src/commands/stage.rs`)

Mirror `dist_tag.rs` / `publish.rs` mock-PM tests, asserting `bin_path` + `args`
for each (PM, subcommand) pair:

```rust
#[test] fn pnpm_stage_publish()    // pnpm, ["stage", "publish"]
#[test] fn npm_stage_publish()     // npm,  ["stage", "publish"]
#[test] fn yarn_berry_stage_publish_uses_npm_plugin() // yarn, ["npm","publish","--staged"]
#[test] fn yarn_berry_stage_list()                    // yarn, ["npm","stage","list"]
#[test] fn yarn_berry_stage_view_falls_back_to_npm()  // npm,  ["stage","view","<id>"]
#[test] fn yarn1_stage_falls_back_to_npm()            // npm,  ["stage", ...]
#[test] fn bun_stage_falls_back_to_npm()              // npm,  ["stage", ...]
#[test] fn pnpm_stage_publish_recursive_filter()      // ["--filter","x","stage","publish"] ordering
#[test] fn stage_approve_otp()                        // ["stage","approve","<id>","--otp","123456"]
```

Also add a clap-parsing test in `cli.rs` (e.g. `stage approve` without an id
errors with `MissingRequiredArgument`).

### Snap tests

Add fixtures alongside the existing `command-publish-*` / `command-pm-*` ones:

- Global: `packages/cli/snap-tests-global/command-pm-stage-pnpm10`,
  `…-npm11`, `…-yarn4`, `…-bun` (assert the resolved command line per PM).
- Local: `packages/cli/snap-tests/command-pm-stage-pnpm10`.
- `vp pm stage --help` / `vp pm --help` snapshots will change, so regenerate and
  inspect the diff (snap tests can pass even when output changes).

Run: `pnpm -F vite-plus snap-test-local command-pm-stage` and
`pnpm -F vite-plus snap-test-global command-pm-stage`, then review `git diff`.

## Documentation

- `docs/guide/install.md`: the `vp pm <command>` "Advanced" section lists
  forwarded commands; add `vp pm stage` with a short staged-publishing blurb and
  a pointer to npm's docs.
- Note the yarn caveat (`vp pm stage` ≠ `yarn stage`) where relevant.
- Regenerate any affected help snapshots (`command-pm-*`).

## Compatibility Matrix

| Subcommand | pnpm                    | npm                    | yarn ≥ 2                       | yarn 1   | bun      | Notes                  |
| ---------- | ----------------------- | ---------------------- | ------------------------------ | -------- | -------- | ---------------------- |
| `publish`  | ✅ `pnpm stage publish` | ✅ `npm stage publish` | ✅ `yarn npm publish --staged` | ⚠️ → npm | ⚠️ → npm | no 2FA                 |
| `list`     | ✅                      | ✅                     | ✅ `yarn npm stage list`       | ⚠️ → npm | ⚠️ → npm |                        |
| `view`     | ✅                      | ✅                     | ⚠️ → npm                       | ⚠️ → npm | ⚠️ → npm | yarn has no `view`     |
| `download` | ✅                      | ✅                     | ⚠️ → npm                       | ⚠️ → npm | ⚠️ → npm | yarn has no `download` |
| `approve`  | ✅                      | ✅                     | ✅ `yarn npm stage approve`    | ⚠️ → npm | ⚠️ → npm | 2FA                    |
| `reject`   | ✅                      | ✅                     | ✅ `yarn npm stage reject`     | ⚠️ → npm | ⚠️ → npm | 2FA                    |

✅ native · ⚠️ warn + fall back to `npm stage …`

## Alternatives Considered

1. **Thin free-string passthrough** (`Stage { subcommand: String, args }`, like
   `vp pm cache`). Simplest to add, but loses typed `--help`/flags and makes the
   yarn divergence (`yarn npm publish --staged`) impossible to express cleanly.
   Rejected in favor of typed subcommands, matching `dist-tag`.

2. **Always delegate yarn → `npm stage`** (matching `publish.rs`). Simpler, but
   ignores yarn's native plugin and the project's yarn auth/registry config.
   Captured as Open Question 1 rather than decided unilaterally.

3. **Hard version-gating in vp.** Rejected: high maintenance across 4 PMs for a
   fast-moving feature; the underlying tool's own error is more accurate.

4. **Top-level `vp stage`** instead of `vp pm stage`. Rejected: staging is a
   package-manager passthrough and belongs in the `vp pm` group with its
   publishing siblings.

## Backward Compatibility

Additive only: a new `vp pm` subcommand. No existing command, config, or
caching behavior changes.
