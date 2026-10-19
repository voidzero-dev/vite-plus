# CLI snapshot tests (PTY runner)

This is the snapshot test suite for the `vp` CLI. Every step runs inside a
real pseudo-terminal backed by a vt100 emulator, so interactive flows
(prompts, pickers, watch modes, ctrl-c) are first-class testable surfaces.
Snapshots are Markdown files with real pass/fail semantics.

**Write new CLI tests here.** Design rationale:
`rfcs/interactive-snapshot-tests.md`.

## Quick start

1. Create a fixture directory: `fixtures/<name>/` (name uses `[a-z0-9_]`).
   Add the project files the test needs (`package.json`, sources, ...).
2. Declare one or more cases in `fixtures/<name>/snapshots.toml`:

   ```toml
   [[case]]
   name = "check_reports_lint_error"
   vp = "local"
   steps = [["vp", "check"]]
   ```

3. Record the snapshot and review it like code:

   ```bash
   UPDATE_SNAPSHOTS=1 just snapshot-test check_reports_lint_error
   git diff  # plus the new fixtures/<name>/snapshots/*.md files
   ```

4. Run in compare mode to confirm it is deterministic, then commit the
   fixture together with its snapshots:

   ```bash
   just snapshot-test check_reports_lint_error
   ```

A failing comparison prints a unified diff and writes `<case>.md.new` next to
the stored snapshot. Never hand-edit `.md` snapshots; re-record instead.

## Running

```bash
just snapshot-test                    # build vp, run everything
just snapshot-test <substring>        # filter by trial name
just snapshot-test-global             # no JS build needed (skips local flavor)
UPDATE_SNAPSHOTS=1 just snapshot-test # accept snapshot changes
pnpm snapshot-test                    # same, via pnpm
cargo test -p vp_cli_snapshots -- <filter>      # if vp is already built
```

Trial names are `<fixture>::<case>` (plus `::<flavor>` for multi-flavor
cases). Prerequisites: both flavors need `cargo build -p vp_global_cli`
(the `just` recipe does it); the local flavor also needs a built
`packages/cli/dist` (`pnpm build`). The runner fails fast when `dist` is
older than `src`, so a forgotten rebuild never silently tests stale
local-CLI code.

CI runs three shards per platform. Linux and macOS use `VP_SNAP_SHARD=1/3`
(then `2/3` and `3/3`) to distribute the ordered trials round-robin. Sharding
happens before name filtering, so filtered runs keep the same assignment.
Leave the variable unset to run the whole suite. Windows uses the existing
nextest runner with `--partition hash:1/3` (then `2/3` and `3/3`).
Each CI shard uses eight workers so process and network waits can overlap.
Local runs retain libtest's default worker count; override it with
`--test-threads <count>`.

Within each shard, the in-process runner executes parallel cases before isolated
cases. This keeps workers available for ready work instead of blocking them on
the execution gate. Scheduling happens after partitioning and does not change
shard membership or test listing order.
The existing gate still protects `serial = true` and Ctrl-C cases, including
against other runner processes.
Npm command cases use local registry fixtures and shared downloads, so they no
longer need serialization to limit concurrent connections to npm. Signal-sensitive
and explicitly isolated cases retain the execution gate.

Environment overrides, mainly for CI:

| Variable                      | Effect                                                                                                  |
| ----------------------------- | ------------------------------------------------------------------------------------------------------- |
| `VP_SNAP_GLOBAL_VP`           | Path to a prebuilt global `vp` binary (skips the target-dir lookup)                                     |
| `VP_SNAP_LOCAL_CLI_BIN_DIR`   | Local CLI bin dir (default `<repo>/packages/cli/bin`)                                                   |
| `VP_SNAP_JS_RUNTIME_DIR`      | Provisioned managed runtime to seed case homes with                                                     |
| `VP_SNAP_SH_BIN`              | POSIX `sh` binary for cases that execute the generated `env` file                                       |
| `VP_SNAP_BASH_BIN`            | Bash binary for cases that execute the generated `env` file                                             |
| `VP_SNAP_ZSH_BIN`             | Zsh binary for cases that execute the generated `env` file                                              |
| `VP_SNAP_CMD_BIN`             | System cmd.exe for cases that execute generated batch files                                             |
| `VP_SNAP_FISH_BIN`            | Fish binary for cases that execute generated `env.fish` files                                           |
| `VP_SNAP_NU_BIN`              | Nushell binary for cases that execute generated `env.nu` files                                          |
| `VP_SNAP_PWSH_BIN`            | PowerShell binary for cases that execute generated `env.ps1` files                                      |
| `VP_SNAP_SKIP_FLAVORS`        | Comma-separated flavors to skip registering (e.g. `local`)                                              |
| `VP_SNAP_REGISTRY_CACHE_DIR`  | Fresh run-scoped response directory shared across nextest processes                                     |
| `VP_SNAP_REGISTRY_MODE`       | `live`: count upstream requests without caching or shared fixtures; `direct`: original registry routing |
| `VP_SNAP_SHARED_REGISTRY_ENV` | Registry environment JSON from the `--snapshot-runner` wrapper, shared across runner processes          |
| `VP_SNAP_PACKED_REGISTRY_ENV` | Registry environment JSON for the wrapper that serves packed checkout packages                          |
| `VP_SNAP_PACKAGES_DIR`        | Run-scoped directory for sharing packed packages across test processes                                  |
| `VP_SNAP_ARTIFACTS_DIR`       | Directory for phase timings and failure diagnostics; unset disables artifacts                           |
| `VP_SNAP_NEXTEST_CONFIG`      | With `--list`, write nextest overrides for the discovered isolated cases                                |

Windows CI generates its nextest configuration from the same case definitions.
Exact nextest runs read only the selected fixture; listing and native shard
assignment still discover all cases.

The overrides reserve all test workers for an isolated case and schedule these
cases last. The file lock remains a fallback when running without the generated
configuration. To use these overrides locally:

```bash
VP_SNAP_NEXTEST_CONFIG="$PWD/target/snapshot-nextest.toml" cargo nextest list -p vp_cli_snapshots
cargo nextest run -p vp_cli_snapshots --config-file target/snapshot-nextest.toml
```

`VP_SNAP_PACKAGES_DIR` packs the checkout on the first registry case, under a
cross-process file lock. Later cases reuse the completed tarballs. Use a fresh
directory for each test run and keep the build unchanged while tests run. The
runner checks the checkout path and input/tarball file sizes and modification
times; a changed build fails instead of silently testing stale packages. This
is a per-run preparation directory, not a persistent content-addressed cache.
Windows CI uses this to avoid repacking in every nextest process.
Cases without fixture overlays share two servers per runner: one for published
packages and one that also serves the packed checkout. Cases with a
`mock-manifest.json`, `tarballs/`, or `.npmrc` retain a dedicated server.
Use `isolated-registry = true` when a test needs private registry state, such as
the request counter used to verify metadata caching. All package-manager caches
remain isolated per case.

Windows CI wraps nextest with `local-npm-registry.ts --snapshot-runner` so the
two servers also survive across test processes. The wrapper passes registry
environments to the Rust runner without changing its package-manager settings.
The packed server loads archives on its first request, after the runner prepares
and validates them under the existing package lock. Both servers exit when the
wrapped command finishes, and the wrapper preserves its exit code.

All cases use a local registry by default. The small test packages in
[`registry/`](registry/README.md) are served from checked-in metadata and tarballs.
Other successful registry GET responses are cached for the duration of the run,
including package-manager downloads. Concurrent requests share one download,
including across nextest processes when `VP_SNAP_REGISTRY_CACHE_DIR` is set.
Use a fresh directory per run; this is not a persistent dependency cache.
Package-manager caches and installed packages remain isolated per case.

`local-registry = true` additionally serves the packed checkout packages.
Published versions remain available for cases that deliberately install an older
release. Their upstream metadata is fetched once per run and merged with the
local version. The standalone registry tool and ecosystem tests retain the same
upstream version merging without the snapshot response cache. Fixture
`mock-manifest.json` entries override shared metadata, and fixture tarballs
override shared tarballs. Explicit case and step registry settings still take
precedence over runner defaults.

With artifacts enabled, the shared server writes `registry.jsonl` in the run
artifact directory. Windows wrappers write separate `shared-registry/registry.jsonl`
and `packed-registry/registry.jsonl` files in the artifact root.
Per-case servers write the same file in their case artifact
directory. Combine both locations when counting requests: `fixture` events
identify locally served data, `request` events count proxy requests, `hit` events
identify reused responses, and `upstream` events count actual upstream HTTP
requests (including redirects). Non-GET requests are forwarded without caching.
`upstream-stalled` identifies requests with no response progress for 15 seconds,
including connection setup. These requests release their cache locks so clients
can retry. Each upstream request also has a 60-second total deadline.
These counts cover the registry; requests to other services, such as Node.js
release downloads or Git hosts, are outside this log.

For a network comparison, run the same cases with eight workers first with
`VP_SNAP_REGISTRY_MODE=live`, then with the variable unset. Both runs route
registry traffic through the logger; `live` disables published recordings and
response cache. `VP_SNAP_REGISTRY_MODE=direct` restores the original routing for
an independent wall-time baseline, but cannot log traffic that bypasses the
local registry. Keep builds unchanged and use fresh response directories.
Local tarball URLs can cause snapshot differences in comparison modes.
Fixture overlays remain active in both comparison modes. Cases with a
`mock-manifest.json` or `tarballs/` start a private registry even in `direct`
mode, so fixed templates remain the same test inputs. Cases that use shared
locally authored packages must set `local-registry = true` so `direct` mode
also starts their registry; these packages have no published version.

With `VP_SNAP_ARTIFACTS_DIR` set, each runner process creates a unique `run-*`
directory. `runner/timing.json` records discovery, test execution, and final
cleanup. `cases/<fixture>/<snapshot>/timing.json` records gate waiting,
provisioning, package preparation/reuse, registry startup, individual steps,
rendering, comparison, and cleanup. `registry-pack` appears only in the process
that actually packs; it is nested inside `registry-pack-or-reuse`. Phase
durations are milliseconds; nested phases must not be added together. The
existing console timings still exclude gate waiting.

The `case-setup/*` phases divide `case-setup` into home creation, binary and
package installation, preference seeding, and first-start setup. These phases are
nested inside `case-setup`; use them to identify preparation costs without
counting the parent duration twice.

`workspace-cleanup` removes the case workspace after comparison, while other
workers can still run tests. The final `run-cleanup` removes shared run files and
any case files left by a panic or an unsuccessful earlier cleanup attempt.

Failed cases also write `error.txt`, `expected.md` (when present), `actual.md`
(when a complete or partial snapshot was rendered), and `output.txt`. The output
contains the last 1 MiB of unredacted rendered step output, including successful
hidden steps; it is not the raw PTY byte stream. After a PTY timeout, the runner kills the
child and waits up to two seconds for the reader to save the final screen,
including when a milestone wait ends at EOF. Artifacts survive temporary
workspace cleanup. CI uploads these files and timings on successful and failed
runs. Listing tests with `--list` neither packs packages nor creates artifacts.

## Case reference

```toml
[[case]]
name = "my_case"              # [A-Za-z0-9_]+, names the snapshot file
vp = "local"                  # "local" | "global" | ["local", "global"]
comment = "What this proves." # rendered into the snapshot
cwd = "packages/app"          # optional, relative to the fixture root
skip-platforms = ["windows"]  # or { os = "linux", libc = "musl" }
requires = ["bash"]           # "sh" | "bash" | "zsh" | "cmd" | "fish" | "nu" | "pwsh"
ignore = false                # true: only runs with `-- --ignored`
serial = false                # true: run alone across cargo test/nextest workers
seed-runtime = true           # false: start from an empty VP_HOME
link-node-modules = false     # true: expose the run-root node_modules as
                              #   the workspace's parent-dir node_modules,
                              #   for `../node_modules/vite-plus/...` paths
env = { MY_VAR = "1" }        # case-wide env additions
unset-env = ["SOME_VAR"]      # remove baseline env entries
steps = [ ... ]
after = [ ... ]               # cleanup steps, never snapshotted
```

`vp` picks which CLI runs the case. Both flavors install the built Rust binary
into the case's `VP_HOME/current/bin`, install the checkout package under that
case home, and run first-start setup before steps. `"global"` exposes only
`VP_HOME/bin`; `"local"` also exposes the case-local
`VP_HOME/current/node_modules/vite-plus/bin` package bin. On Windows, local
flavor exposes sibling `.cmd` shims under
`VP_HOME/current/node_modules/.vite-plus-bin` instead. The list form registers
one trial and one snapshot per flavor; use it for parity cases (help output,
routing, error messages) where both surfaces must agree.

Each case starts with explicit managed-mode preferences for npm, pnpm, Yarn,
and Bun. The runner writes these preferences before starting the unmarked
`vp` binary with no arguments. First-start setup creates the environment files
and shims, then exits. This avoids a second `vp env setup --refresh`. Tests of
preference inference or an empty installation must create a separate home, as the
`shim_package_manager_setup` fixture does.
The preparation command sets `VP_SELF_SETUP_NO_MODIFY_PATH=1` so Windows
self-setup does not add temporary case homes to the user's persistent `PATH`.
This override applies only to runner preparation, not to fixture commands.

A step is a bare argv array or a table:

```toml
{ argv = ["vp", "create"],
  cwd = "sub",                # per-step working dir
  comment = "...",            # rendered under the step heading
  envs = [["K", "V"]],        # per-step env; values expand `${NAME}`:
                              #   `${workspace}` is the step's working dir,
                              #   `${PATH_SEPARATOR}` is `:` on Unix and `;` on Windows,
                              #   any other name resolves from the case env
                              #   (`PATH = "${workspace}/bin${PATH_SEPARATOR}${PATH}"`
                              #   is the shell's `PATH="$(pwd)/bin:$PATH"`)
  timeout = 120000,           # ms, default 120s (includes cold package-manager downloads)
  snapshot = false,           # omit the screen while the step succeeds
                              #   (failures always keep their output)
  continue-on-failure = true, # a failing step skips the rest of its line,
                              #   up to the next continue-on-failure step;
                              #   with none ahead the case stops (shell &&)
  tty = false,                # piped stdio instead of a PTY (non-TTY tests)
  interactions = [ ... ] }
```

`argv[0]` may be `vpt`, a runner-approved shell (including system `cmd.exe`
exposed as `cmd`), or any executable exposed by the case's Vite+ installation,
such as `vp`, `node`, `npm`, and `pnpm` and globally installed package
binaries. There is no shell: no `&&`, no
redirects, no globs. File setup and assertions go through `vpt` so behavior
is identical on every platform:

`vpt print-file` (cat), `vpt stat-file` (prints `file`/`dir`/`symlink`/`missing`;
`--assert <state>` / `--assert-not <state>` also fail on mismatch, so
`test -f x && cmd` guards keep their short-circuit), `vpt write-file`,
`vpt touch-file`, `vpt replace-file-content`, `vpt list-dir`, `vpt mkdir`,
`vpt rm`, `vpt cp`, `vpt chmod`, `vpt grep-file`, `vpt json-edit`,
`vpt pipe-stdin <data> -- <argv>`,
`vpt head-lines <count> -- <argv>` (closes the child's stdout after the selected
lines), plus task payloads for `vp run` tests:
`vpt print`, `vpt print-color`, `vpt print-env`, `vpt print-cwd`,
`vpt print-native-path` (prints OS-native separators, for redaction
self-tests), `vpt check-tty`, `vpt read-stdin`, `vpt exit <code>`,
`vpt exit-on-ctrlc`, `vpt barrier`, and the Unix-only
`vpt backpressure-run [--digest <head>,<tail>] -- <argv...>`
for running a command with deliberately backpressured, non-blocking stdout.

## Interactive cases

Interactive steps script keystrokes synchronized on milestones: invisible
markers the CLI emits at deterministic render points (only when the runner
sets `VP_EMIT_MILESTONES=1`). Waiting is always on a named milestone, never
on sleeps or output polling; that is what keeps keystroke-driven UI
deterministic.

```toml
[[case]]
name = "picker_selects_second_entry"
vp = "local"
steps = [
  { argv = ["vp", "create"], interactions = [
    { "expect-milestone" = "select:template:0" }, # wait, then capture screen
    { "write-key" = "down" },
    { "expect-milestone" = "select:template:1" },
    { "write-key" = "enter" },
  ] },
]
```

- `expect-milestone` blocks until the named milestone arrives, then captures
  the rendered screen into the snapshot.
- `write` sends raw text, `write-line` adds the platform newline.
- `write-key` sends one of: `up`, `down`, `left`, `right`, `enter`,
  `escape`, `space`, `tab`, `backspace`, `ctrl-c`.

Milestone names follow `<kind>:<id>:<state>`. The prompt components in
`packages/prompts` (`select`, `confirm`, `text`) already emit them; `id`
defaults to the kind and is overridden with the `testId` prompt option when
a flow shows several prompts of the same kind. States: cursor index for
select, `yes`/`no` for confirm, the typed value for text, plus
`submit`/`cancel`. When you instrument a new prompt component or a
non-prompt sync point (`dev-server:ready` style), keep the name a pure
function of the rendered state.

`vpt probe` is a self-contained interactive payload useful for testing the
runner itself (see `fixtures/interactive_probe/`).

## What a step sees

Each case gets a cleared environment: controlled `PATH` (runner `vpt`, then
case-owned tool dirs, then a system tail for child processes and direct `git` steps),
`TERM=xterm-256color`, `VP_CLI_TEST=1`, `VP_EMIT_MILESTONES=1`, a fresh
`HOME`, `VP_HOME`, and npm prefix. The runner still rejects direct step tools
that resolve outside the case-owned dirs, except for `git`; `vpt` is the only
required runner helper on PATH, while optional POSIX, Fish, and Nushell binaries
are linked there when available. PowerShell runs from its canonical path so it
can load files installed beside `pwsh.exe`. `cmd.exe` also runs from its
canonical system path. `CI` and `NO_COLOR` are deliberately NOT
set: with a PTY attached, the CLI behaves interactively by default, which is
the point.
`seed-runtime = true` (default) symlinks a provisioned managed Node runtime
into the case `VP_HOME` so commands do not download ~50MB per case.

Fixture configs may import bare `vite-plus` and
`@voidzero-dev/vite-plus-core`: the runner links the checkout packages
into the run root's `node_modules`, where Node's upward walk finds them
from any staged workspace. Anything else a fixture imports must be
vendored inside the fixture itself.

Snapshots are plain-text screen grids: styling is flattened, and redaction
masks paths, durations, versions, UUIDs, thread counts, byte-size numbers
(units kept: `<size> kB`), and content-hash asset suffixes (see
`redact.rs`; sizes and hashes because output bytes differ across OSes). If
a case produces nondeterministic output, fix it with controlled fixture data,
a milestone, or a redaction rule; never rerun until green. Set
`formatted-snapshot = true` on a step only when the test is about colors.

For registry fallback tests, use `local-registry = true` and a fixture
`mock-manifest.json` entry such as `"npm/latest"` to control the resolved version.
Keep the CLI input unpinned so the fallback is still tested, and keep exact
version assertions. Unmocked registry responses are fetched upstream once per run, so
`local-registry = true` alone does not freeze dist-tags across runs.

Fixture trees are excluded from repo-wide fmt, lint, typecheck, and vitest
(`vite.config.ts`, `tsconfig.json`); recorded snapshots and
`snapshots.toml` are runner metadata and never appear inside the staged
workspace a test runs in.

Test servers must request port `0` so the OS selects an available port at bind
time. Fixed ports can be reserved on Windows, where `EACCES` does not trigger
Vite's port-in-use fallback. Vitest 5.0.4 replaces a plain `test.api.port: 0`
with its default, including when it comes from `--api.port=0`. Set the port in
a post-configuration hook instead; see
[`test_v5_browser_defines/config.mjs`](fixtures/test_v5_browser_defines/config.mjs)
for independent, inherited, referenced, and injected projects, and
[`test_v5_optional_packages/verify-ui.mjs`](fixtures/test_v5_optional_packages/verify-ui.mjs)
for the UI server. Assert that each resolved API port is `0` so regressions fail
on every platform, even when the default port is available.
