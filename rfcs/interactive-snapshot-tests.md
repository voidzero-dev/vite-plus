# RFC: Interactive CLI Snapshot Tests

## Summary

Replace the current snap-test runner with a new snapshot-test solution that runs every test case inside a real pseudo-terminal (PTY) backed by a vt100 screen emulator. Test cases can script full interactive sessions: they send keystrokes (arrows, enter, ctrl-c, free text) and synchronize on render milestones emitted by the CLI, so prompts, pickers, spinners, and watch modes become first-class testable surfaces. Snapshots are Markdown files containing rendered terminal screens, compared with real pass/fail semantics (`UPDATE_SNAPSHOTS=1` to accept changes) instead of the current regenerate-and-inspect-git-diff model.

The runner reuses the PTY/terminal-emulation/milestone/snapshot crates that already exist in the vite-task repository (`pty_terminal`, `pty_terminal_test`, `pty_terminal_test_client`, `snapshot_test`), which vite-plus already consumes as git dependencies for other crates. The two existing trees (`snap-tests/` and `snap-tests-global/`) merge into a single fixture tree where each case declares whether it runs under the global Rust `vp` binary or the local JS CLI (or both).

This is a clean break: the new format is not compatible with the old `steps.json`/`snap.txt` format and does not try to be. A migration tool converts old case directories to new fixtures in one command, and the old runner is deleted once the corpus is migrated.

## Motivation

### The interactive gap

The current runner cannot test interactive behavior at all:

- Every command runs with `stdin: null` and `CI=true`, so the CLI always takes its non-interactive path.
- 364 fixtures pass `--no-interactive` explicitly; zero fixtures drive a prompt.
- Interactive UX regressions (spinner-over-prompt rendering, picker navigation, ctrl-c cancellation) can only be caught by hand, via a tmux-based manual workflow.

This blocks real coverage that current work needs. PR #2031 (`vp -C` and app-root resolution) lists its interactive package picker as an untestable follow-up, and its "single-runnable auto-select in interactive terminals" branch ships without a test that runs in an interactive terminal. `vp create` and `vp migrate` prompt flows (template selection, approve-builds confirmation, overwrite prompts) have no automated coverage of the actual prompt loop. The parked `snap-tests-todo/command-pack-watch-restart` case exists precisely because watch-mode restart needs a terminal you can type into.

### Structural problems in the current runner

The audit of `packages/tools/src/snap-test.ts` and the ~529-case corpus surfaced problems that patching cannot fix well:

1. **No assertions.** The runner always overwrites `snap.txt` and exits 0. Failure detection lives outside the runner (`git diff` in CI, human discipline locally). AGENTS.md has to warn about this twice.
2. **No terminal.** Output is captured from redirected pipes, so anything that depends on a TTY (prompt rendering, spinner behavior, color decisions, terminal width) is untested or tested in a mode users never see.
3. **Normalization debt.** `replaceUnstableOutput` is ~50 regex substitutions masking spinner frames, per-package-manager progress lines, ANSI remnants, durations, and more. Much of it exists because raw byte streams from concurrent writers are inherently unstable. A rendered screen grid eliminates whole classes of this.
4. **Timeouts leak.** Timed-out commands are not killed; the process survives only because the runner ends with `process.exit(0)`.
5. **Shared global state.** All cases share `VP_HOME=~/.vite-plus`, forcing `serial: true` on 19 cases and leaving latent cross-case interference for the rest. The global runner also hard-requires `pnpm bootstrap-cli` before every run so the installed binary byte-matches the checkout.
6. **Shell semantics drift.** Commands run through `@yarnpkg/shell`, an in-process JS shell with its own comment-stripping and no-glob rules. 182 fixtures skip Windows, and shell/tool differences are a major reason.
7. **Flakiness is managed, not removed.** CI reruns changed cases up to twice (`retry-failed-snap-tests.sh`) and accepts the result if the diff stops moving.

### Why not extend the old runner

Adding a PTY mode to `snap-test.ts` would keep the no-assertion model, the shell layer, the shared `VP_HOME`, and the two-tree split, while adding the hardest part (deterministic interactive synchronization) on top of a Node PTY stack that would need to relearn platform lessons (ConPTY output reordering, musl PTY crashes, macOS EIO truncation) that the vite-task crates already encode. A clean second system with a migration path is cheaper than an in-place rebuild.

## Prior art: vite-task's snapshot suite

The vite-task repository has a working implementation of exactly this design, used by ~190 snapshot files today, including interactive selector navigation and ctrl-c cancellation cases. Its pieces:

| Crate                      | Role                                                                                                                                                                                                                                                                            |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `pty_terminal`             | Spawns a child in a PTY (`portable-pty`), feeds output through a `vt100` emulator, answers cursor-position queries, handles resize and ctrl-c. Encodes platform workarounds: ConPTY on Windows, a global lock for musl PTY crashes, macOS slave-fd lifetime for EIO truncation. |
| `pty_terminal_test`        | `TestTerminal` wrapper plus `Reader::expect_milestone(name)`: block until the child emits a named milestone, then return the rendered screen.                                                                                                                                   |
| `pty_terminal_test_client` | Child-side helper that encodes milestones as window-title updates (`OSC 2 ; pty-terminal-test:<32-hex-id>:<base64url(name)>`, a fresh id per emission), which survive both Unix PTYs and Windows ConPTY and arrive in-order with the output they mark.                          |
| `snapshot_test`            | Minimal snapshot store: compare or update via `UPDATE_SNAPSHOTS=1`, write `<name>.new` on mismatch, return a unified diff as the failure message.                                                                                                                               |

On top of these, `vite_task_bin/tests/e2e_snapshots` implements a `libtest-mimic` custom test target: fixtures declare cases in `snapshots.toml`, steps are argv arrays (no shell), interactive steps carry an ordered `interactions` list, and each case produces one Markdown snapshot containing the command lines, the interaction log, and fenced terminal screenshots captured at each milestone and at exit.

vite-plus already depends on vite-task crates via git (`fspy`, `vite_glob`, `vite_path`, `vite_str`, `vite_task`, `vite_workspace`), and the Rust CLI's interactive picker work is planned on `vite_select`, which already exposes the `after_render` hook that milestone emission needs. Reusing this stack is the lowest-risk path to deterministic interactive testing.

## Goals

- Test interactive sessions deterministically: scripted keystrokes, synchronized on explicit render milestones, never on sleeps or output polling.
- Real assertions: a snapshot mismatch fails the test with a unified diff; updates are an explicit opt-in.
- One fixture tree; each case declares the `vp` flavor (global Rust binary, local JS CLI, or both).
- Full process isolation: per-case `VP_HOME`, cleared environment, controlled `PATH`, killed-on-timeout children.
- Keep what works today: fixture-directory model, local npm registry integration, platform filters, sharded CI.
- A one-command migration tool for the existing `steps.json` corpus, with an explicit report of what needs human attention.

## Non-goals

- Compatibility with the old `steps.json`/`snap.txt` format. Old snapshots are not translated; new baselines are recorded and reviewed.
- Replacing unit tests or the ecosystem-ci/e2e suites. This RFC only replaces the snap-test layer.
- Testing real terminal emulators (iTerm, Windows Terminal). The vt100 emulation is the contract, same as vite-task.

## Design overview

New layout (names open to bikeshedding):

```
crates/vite_cli_snapshots/          # dev-only crate, never published or packaged
├── Cargo.toml                      # publish = false; dev-deps: libtest-mimic,
│                                   #   pty_terminal_test, snapshot_test
├── src/bin/vpt.rs                  # test utility multitool (own bin target)
└── tests/cli_snapshots/
    ├── main.rs                     # libtest-mimic runner (harness = false)
    ├── redact.rs                   # output normalization
    ├── flavor.rs                   # global/local vp provisioning
    └── fixtures/
        └── <case-dir>/
            ├── snapshots.toml      # case declarations
            ├── package.json        # fixture files, copied verbatim
            ├── ...
            ├── mock-manifest.json  # optional, local-registry cases
            ├── tarballs/           # optional, local-registry cases
            └── snapshots/
                └── <case>.md       # recorded snapshots
```

The runner is a dedicated workspace crate, not a test target of `crates/vite_global_cli`. The product crate stays untouched: no test-only bin in its target list, no `vpt` dependencies in its dependency graph, no packaging exclusions to maintain. `vpt` is a bin target of the runner crate itself, so `CARGO_BIN_EXE_vpt` still resolves it for free.

The one thing this layout gives up is `CARGO_BIN_EXE_vp` (Cargo only sets it for tests of the package that defines the binary). Instead the runner resolves `vp` at runtime from its own executable location: test binaries run from `target/<profile>/deps/`, so the global binary sits in the parent directory (the same technique `assert_cmd::cargo_bin` uses). A runtime lookup is also friendlier to the Windows CI flow, where nextest archives are built on Linux and run on another machine, than a compile-time absolute path baked in by `env!`. Build ordering is handled by the entry-point recipe (`just snapshot-test` and the pnpm wrapper run `cargo build -p vite_global_cli` before `cargo test -p vite_cli_snapshots`); if the binary is missing, the runner fails fast with that instruction rather than testing a stale build.

Execution flow per case:

1. Copy the fixture directory to a fresh temp dir (so workspace-root discovery stops there, and cases can mutate files freely).
2. Provision the case environment: temp `VP_HOME`, temp npm global prefix, cleared env rebuilt from a fixed baseline, `PATH` composed from a per-case bin dir (the selected `vp` flavor, `vpt`, node) plus minimal system dirs.
3. For each step: spawn the argv in a fresh PTY (500x500 grid, `TERM=xterm-256color`), run its `interactions` (each `expect-milestone` captures a screen), wait for exit or kill on timeout, capture the final screen, redact, and append to the case's Markdown document.
4. Compare the document against `snapshots/<case>.md`; on mismatch write `<case>.md.new` and fail with a unified diff. With `UPDATE_SNAPSHOTS=1`, write the snapshot instead.

Trial names are `<fixture-dir>::<case>[::<flavor>]`, so `cargo test --test cli_snapshots -- create` filters like today's substring filter, and `libtest-mimic` gives parallelism, `--ignored`, and exact-name selection for free.

## Test case format

Each fixture directory contains one `snapshots.toml` declaring one or more cases.

### Case fields

```toml
[[case]]
name = "create_interactive_template_pick"   # [A-Za-z0-9_]+, names the snapshot file
vp = "local"                                # "local" | "global" | ["local", "global"]
comment = """
Arrow-down in the template picker selects the second template.
"""
cwd = "packages/app"                        # optional, relative to fixture root
skip-platforms = ["windows"]                # optional; values: "windows", "linux",
                                            #   "macos", { os = "linux", libc = "musl" }
ignore = false                              # optional; registers the trial as #[ignore]
local-registry = false                      # optional; serve checkout packages + fixture
                                            #   tarballs through the local npm registry
seed-runtime = true                         # default; symlink a provisioned managed JS
                                            #   runtime into the case VP_HOME (false for
                                            #   runtime-provisioning tests)
env = { VITE_DISABLE_AUTO_INSTALL = "1" }   # optional; case-wide env additions
unset-env = ["GITHUB_ACTIONS"]              # optional; remove from the baseline env
steps = [ ... ]
after = [ ... ]                             # optional cleanup steps, never snapshotted
```

Notes:

- `vp` replaces the tree split. `"local"` puts the JS CLI (`packages/cli/bin`) first on `PATH`; `"global"` uses the freshly built Rust binary. The list form registers one trial per flavor with separate snapshot files (`<case>.local.md`, `<case>.global.md`); it exists for parity cases such as command routing, where global and local must behave identically and a shared fixture keeps them honest.
- `skip-platforms` keeps the exclude semantics of today's `ignoredPlatforms` (the include-one `platform` field from vite-task is less convenient for this corpus, where Windows exclusion dominates).
- There is no `serial` field. Per-case `VP_HOME` and npm-prefix isolation removes the shared state that forced it in the old runner; the migration tool drops old `serial: true` flags and reports them.

### Step fields

A step is either a bare argv array or a table:

```toml
steps = [
  ["vp", "check"],                                # shorthand
  { argv = ["vp", "create"],
    cwd = "sub",                                  # optional, relative to staged root
    comment = "second run hits the cache",        # optional, rendered in the snapshot
    envs = [["ADBLOCK", "1"]],                    # optional per-step env
    timeout = 120000,                             # optional ms; default 50s (120s local-registry)
    snapshot = true,                              # false: run but omit screen from snapshot
    tty = true,                                   # false: pipe mode (no PTY), for explicit
                                                  #   "stdout is not a TTY" behavior tests
    formatted-snapshot = false,                   # true: keep SGR color/style escapes
    interactions = [ ... ] },
]
```

Semantics:

- `argv[0]` may be `vp`, `vpr`, `vpx`, `vpt`, or an allow-listed real tool (`node`, `git`, `npm`, `pnpm`, `yarn`, `bun`). Everything else (file inspection, setup, assertions) goes through `vpt` so behavior is identical on Windows. There is no shell: no `&&`, no redirection, no comment stripping, no glob surprises.
- Steps run sequentially. Nonzero exit codes are recorded in the snapshot (`**Exit code:** N`) and execution continues; a step timeout kills the child, records `timeout`, and skips the remaining steps.
- `snapshot = false` replaces today's `ignoreOutput` with the same on-success-only semantics: the step heading and exit code still appear and a failing step keeps its screen for diagnosis; only successful output is omitted.
- `tty = false` spawns with pipes instead of a PTY for cases that specifically test piped/CI-style output. The default is a real PTY, which flips today's default: the CLI under test sees a TTY unless the case says otherwise, matching what users see.

### Interactions

Interactive steps carry an ordered `interactions` list, executed against the PTY:

```toml
interactions = [
  { "expect-milestone" = "text:project-name" },      # wait, then capture the screen
  { write = "my-app" },                              # raw bytes, no newline
  { "expect-milestone" = "text:project-name:my-app" },
  { "write-key" = "enter" },
  { "expect-milestone" = "select:template:0" },
  { "write-key" = "down" },
  { "expect-milestone" = "select:template:1" },
  { "write-key" = "enter" },
]
```

- `expect-milestone` blocks until the CLI emits the named milestone, then captures the rendered screen into the snapshot. EOF before the milestone fails the step with the screen content in the error.
- `write` / `write-line` send text (`write-line` appends the platform newline).
- `write-key` sends a named key: `up`, `down`, `left`, `right`, `enter`, `escape`, `space`, `tab`, `backspace`, `ctrl-c`. The set extends as prompt components need (clack multiselect uses `space`, for example).

Waiting is event-driven on explicit milestones only. There is no wait-for-text, no idle detection, and no sleep primitive; those are the mechanisms that made PTY-based testing flaky everywhere else.

### Example: the PR #2031 follow-up

The interactive package picker that PR #2031 could not test becomes:

```toml
[[case]]
name = "app_root_interactive_picker"
vp = "global"
comment = "Bare `vp dev` at a workspace root opens the package picker; arrow-down selects the web app, ctrl-c on the next prompt exits cleanly."
steps = [
  { argv = ["vp", "dev"], interactions = [
    { "expect-milestone" = "select:app-target:0" },
    { "write-key" = "down" },
    { "expect-milestone" = "select:app-target:1" },
    { "write-key" = "ctrl-c" },
  ] },
]
```

The snapshot then contains the rendered picker at cursor position 0, at cursor position 1, and the cancellation output, all as plain-text screens.

## Choosing and provisioning the vp flavor

### Global (`vp = "global"`)

The runner runs the freshly built Rust binary, resolved from the target directory next to the test executable (see Design overview), linked into the per-case bin dir under the names `vp`, `vpr`, and `vpx`. `VITE_GLOBAL_CLI_JS_SCRIPTS_DIR` points at the checkout's `packages/cli/dist`, as today.

This removes two standing costs of the current global runner:

- No `pnpm bootstrap-cli` requirement and no byte-match assertion against `~/.vite-plus/bin/vp`; the binary under test is always the checkout's build.
- No shared `~/.vite-plus`. Each case gets a temp `VP_HOME`, so `vp env` mutations, global installs, and default-version changes cannot interfere across cases and `serial` disappears. Because an empty home makes any runtime-touching command download a ~50MB managed Node archive, the runner seeds each case's `VP_HOME/js_runtime` as a symlink to an already-provisioned runtime: `VP_SNAP_JS_RUNTIME_DIR` when set (CI restores a cached runtime there), else the developer's real `~/.vite-plus/js_runtime`. The seed is read-mostly; cases that test runtime provisioning itself opt out with `seed-runtime = false` and pay the download.

### Local (`vp = "local"`)

The per-case bin dir fronts `packages/cli/bin` (the JS dispatch), which requires `node` on `PATH` and a built `packages/cli`, both already prerequisites of today's local runner. `VP_SNAP_LOCAL_CLI_BIN_DIR` overrides the default `<repo>/packages/cli/bin` when the built `dist/` lives elsewhere (another checkout, a CI artifact directory); the runner fails fast with a `pnpm build` instruction when the dist entry is missing.

### Both

`vp = ["local", "global"]` is the parity tool. It produces two trials and two snapshots from one fixture. Command-surface cases (help output, routing, `-C` handling, error messages) are the intended users; heavyweight cases should pick one flavor.

## Execution model

- **PTY and screen.** Each step spawns in a fresh PTY with a 500x500 grid (large enough that wrapping and scrolling do not distort output; scrollback capture is an open question for outputs beyond 500 rows). Output feeds a vt100 emulator continuously; screens are read from the emulator, never from raw bytes. Plain capture flattens all styling; `formatted-snapshot = true` preserves SGR color codes (rendered as escaped `\x1b[31m` text) for cases that assert color behavior, with the ConPTY parity fix (bare SGR-reset stripping) inherited from `pty_terminal`.
- **Environment.** The child env is cleared and rebuilt: fixed baseline (`VP_CLI_TEST=1`, `VP_EMIT_MILESTONES=1`, `TERM=xterm-256color`, git identity, temp `VP_HOME`, temp `NPM_CONFIG_PREFIX`, controlled `HOME`), a small platform allow-list (Windows needs `SYSTEMROOT`, `APPDATA`, etc.), then case `env`/`unset-env`, then step `envs`. Today's ~40-pattern passthrough allow-list and its leakage risks go away. Notably absent from the baseline: `CI=true` and `NO_COLOR` (the PTY makes real interactive behavior the default, and the grid render makes color stripping unnecessary).
- **Timeouts.** Per-step, default 50s (120s for `local-registry` cases), overridable per step. On timeout the child is killed, the exit is recorded as `timeout`, and remaining steps are skipped. No leaked processes.
- **Exit codes.** Recorded in the snapshot when nonzero. On failure, execution skips the rest of the step's "line" (everything up to and including the next `continue-on-failure = true` step) and resumes after it; with no boundary ahead, the case stops, shell-style. The migrator reproduces legacy semantics exactly: `&&` chain members keep the stopping default and each legacy command line's final step carries the boundary marker, so a mid-chain failure skips its line while the next line still runs, and later steps never bless output from a broken same-line setup.
- **Concurrency.** `libtest-mimic` runs trials in parallel. vite-task serializes on Linux because of ctrl-c signal-routing flakiness in parallel PTY tests; with a corpus this size we should scope that serialization to signal-sensitive cases (a `serial-signals` marker or a dedicated shard) rather than the whole suite. This needs measurement during implementation.

## Milestone protocol and CLI instrumentation

A milestone is an invisible marker the CLI writes into its output stream at a deterministic render point. The encoding is the vite-task protocol: a window-title update (`OSC 2 ; pty-terminal-test:<32-hex-id>:<base64url(name)>`) with a fresh random id per emission so repeated names stay observable as distinct title changes. It survives Unix PTYs and Windows ConPTY, arrives in-order with the output it marks, and renders as nothing in a real terminal's screen content.

Emission is gated on `VP_EMIT_MILESTONES=1`, which only the runner sets. vp is a widely distributed CLI whose output gets piped into logs and other tools, so unconditional emission (vite-task's choice) is not appropriate here.

Instrumentation points:

- **Rust prompts** (`crates/vite_global_cli`, `crates/vite_shared`): emit via `pty_terminal_test_client` (a new git dependency, same source as the existing vite-task crates). The planned interactive package picker builds on `vite_select`, whose `after_render(RenderState)` hook was designed for exactly this; the picker emits `select:app-target:<index>` per render.
- **TS prompts** (`packages/prompts`): a small helper (`emitMilestone(name)`) writes the same byte sequence, wired into each prompt component's render loop. Naming convention: `<kind>:<id>:<state>`, for example `text:project-name:my-app`, `select:template:1`, `confirm:approve-builds:yes`, `spinner:install:stop`. Prompt call sites gain a stable `id` (the component kind plus an explicit name where ambiguous).
- **Non-prompt sync points**: long-running commands may mark stable lifecycle points (`dev-server:ready`, `watch:rebuilt`) so tests of servers and watch modes have something to wait on before sending the next keystroke or ctrl-c. This is what unblocks the parked `command-pack-watch-restart` case.

The milestone name deliberately encodes post-render state (query text, cursor index, selection), not just an event name. Waiting for `select:template:1` after pressing `down` means the screen capture cannot race the render.

## Snapshot format and update workflow

One Markdown file per case (per flavor), mirroring vite-task:

````markdown
# create_interactive_template_pick

Arrow-down in the template picker selects the second template.

## `vp create`

**→ expect-milestone:** `select:template:0`

```
◆  Select a template:
│  › vite-react
│    vite-vue
│    library
```

**← write-key:** `down`

**→ expect-milestone:** `select:template:1`

```
◆  Select a template:
│    vite-react
│  › vite-vue
│    library
```

**← write-key:** `enter`

```
◇  Select a template:
│  vite-vue
...final screen after exit...
```
````

Step headings include `cd <cwd> &&` and `ENV=val` prefixes so the snapshot is self-describing; nonzero exits appear as `**Exit code:** N`.

Comparison semantics come from the `snapshot_test` crate:

- Default run: mismatch writes `snapshots/<case>.md.new` and fails the trial; the failure message is a unified diff, printed by `cargo test` in the failures summary.
- `UPDATE_SNAPSHOTS=1 just snapshot-test`: writes snapshots, removes stale `.new` files.
- A missing snapshot is a failure that writes the `.new` file for review, so brand-new cases follow the same review path.

This replaces the regenerate-always model. CI stops depending on `git diff` over `snap.txt` and the retry script for correctness; a failing snapshot is a failing test.

## Output normalization

Redaction runs on captured screens before they enter the snapshot, ported from vite-task's `redact.rs` and extended with the vp-specific rules that remain necessary:

- Paths: staged temp root, `VP_HOME`, home dir, workspace root, Windows separators and `\\?\` prefixes.
- Durations, sizes, thread counts, UUIDs, content hashes.
- Version numbers of bundled tools where the case is not about versions.
- Registry host (npmjs vs mirror).
- Unordered diagnostic blocks (sorted, as vite-task does for multithreaded lint output).

What should shrink or disappear relative to today's ~50 regexes: spinner-frame masking (the grid shows the rendered final state, not animation frames), ANSI cleanup (plain capture strips styling), package-manager progress-line masking (progress renders in place on a grid instead of accumulating in a byte stream), and stdout/stderr interleaving hacks (both feed one terminal, which is what users see anyway). The redaction module should start minimal and grow only on demonstrated need; every regex added back is a determinism bug worth understanding first.

## The vpt test utility

`vpt` is a small Rust multitool (a bin target of `crates/vite_cli_snapshots`, so its dependencies never touch the product crates) replacing the shell built-ins that dominate the old corpus (427 `cat`, 141 `test`, plus `mkdir`/`rm`/`ls`/`echo`/`cp`/`chmod`/`printf` and `json-edit`).

It is deliberately not a new design. vite-task's `vtt` multitool already covers almost all of this surface with 20 subcommands, so `vpt` adopts `vtt`'s subcommand names and semantics verbatim wherever they overlap and ports the implementations (they are std-only by design, a few dozen lines each). Keeping the contract identical means fixtures, snapshots, and habits transfer between the two repos.

Setup and assertion subcommands (replacing shell built-ins in old cases), all `vtt`-aligned:

| Subcommand                                                       | Replaces                                          |
| ---------------------------------------------------------------- | ------------------------------------------------- |
| `vpt print-file <file>`                                          | `cat` (snapshot file contents)                    |
| `vpt stat-file <path>`                                           | `test -f x && echo ...` existence checks          |
| `vpt write-file` / `vpt touch-file` / `vpt replace-file-content` | `echo`/`printf` redirects, in-place fixture edits |
| `vpt list-dir` / `vpt mkdir` / `vpt rm` / `vpt cp`               | coreutils usage                                   |
| `vpt grep-file <pattern> <file>`                                 | content assertions                                |
| `vpt pipe-stdin <data> -- <argv>...`                             | piped-stdin scenarios without a shell             |

Payload subcommands, for cases where the command under test spawns other commands (`vp run` task execution, caching, cancellation, stdio passthrough), same as their `vtt` counterparts:

| Subcommand                                                          | Purpose                                                               |
| ------------------------------------------------------------------- | --------------------------------------------------------------------- |
| `vpt print` / `vpt print-color` / `vpt print-env` / `vpt print-cwd` | deterministic task output; color, env, and cwd propagation into tasks |
| `vpt check-tty` / `vpt read-stdin`                                  | stdio wiring of spawned tasks                                         |
| `vpt exit <code>` / `vpt exit-on-ctrlc` / `vpt barrier`             | exit-code handling, cancellation, concurrency synchronization         |

vp-specific additions with no `vtt` counterpart: `vpt json-edit <file> <dot-path> <value>` (the existing snap-tests `json-edit` helper for fixture manifest edits) and `vpt chmod`.

Reusing `vtt` itself was considered and rejected. Cargo git dependencies provide library code only, never a dependency's binaries, so obtaining the `vtt` executable would require an out-of-band `cargo install --git` pinned in lockstep with the other vite-task git deps across local dev, CI, and nextest archives. Reusing it as a library would mean depending on `vite_task_bin` and dragging the entire `vt` product tree (task engine, TUI, server, fspy) into the runner build for a handful of trivial helpers. And vp-specific subcommands would then need upstream PRs plus dep bumps before tests here could use them. If the duplication ever becomes a maintenance burden, the designated path is upstream extraction: vite-task moves the subcommands into a small library crate (as `pty_terminal` already is for the emulator) and `vtt`/`vpt` become thin bin wrappers over it.

Everything `vpt` prints is deterministic and platform-identical, which directly attacks the biggest cause of the 182 Windows skips. The subcommand list grows as migration finds patterns worth first-classing; anything not worth a subcommand is a sign the old case was testing the shell, not vp.

## Local registry integration

`local-registry = true` on a case replaces both `localVitePlusPackages` and the `node $SNAP_LOCAL_REGISTRY -- ...` wrapper convention:

- The runner packs the checkout's `vite-plus` and `@voidzero-dev/vite-plus-core` once per run (reusing `packages/tools/src/pack-local-vite-plus.ts`).
- Per case, it starts `packages/tools/src/local-npm-registry.ts` and injects the per-package-manager registry env (npm/pnpm/yarn/bun) into every step, so fixture commands are plain `vp migrate ...` instead of wrapper invocations.
- The `mock-manifest.json` + `tarballs/` sidecar convention carries over unchanged for org-package fixtures.

The registry tool itself is unchanged; it already serves packed tarballs overlaid on upstream packuments with correct integrity and min-release-age-safe publish times, and it stays shared with ecosystem-ci and local `vp create`/`vp migrate` iteration.

## CI integration

- The suite is a cargo test target: `cargo build -p vite_global_cli` followed by `cargo test -p vite_cli_snapshots --test cli_snapshots`, wrapped in a `just snapshot-test` recipe. Sharding uses `cargo nextest --partition` instead of the custom `--shard=i/n` logic.
- Both flavors run in CI from day one, in dedicated jobs. `cli-snapshot-test` (Linux and macOS, one leg per OS) builds `packages/cli/dist`, installs the release binary, and runs the full suite with the global flavor pointed at the installed binary via `VP_SNAP_GLOBAL_VP`, so no second `vite_global_cli` compile is needed. (`VP_SNAP_SKIP_FLAVORS` remains available for environments that cannot provide one of the flavors, e.g. local runs without a built `dist/`.)
- The Windows story reuses the existing cross-compile infrastructure: `build-windows-tests` produces a dedicated `-p vite_cli_snapshots` nextest archive (carrying the test binary and `vpt`), and the `cli-snapshot-test-windows` job runs it on `windows-latest` with no Rust toolchain. The global `vp` comes prebuilt from `build-windows-cli` via `VP_SNAP_GLOBAL_VP`, the JS CLI is built on the runner for the local flavor, and nextest's `--workspace-remap` rewrites `CARGO_MANIFEST_DIR`/`CARGO_BIN_EXE_vpt` at run time so the relocated binaries find fixtures and helpers in the checkout (the runner prefers those runtime values over compile-time paths for exactly this reason).
- musl coverage keeps its Alpine container leg; `pty_terminal` already serializes PTY spawn on musl internally.
- Pass/fail is the test exit code. The `git diff` gate and `retry-failed-snap-tests.sh` do not apply to the new suite. If a case proves flaky, the fix is a milestone or a redaction rule, not a rerun; a temporary quarantine (`ignore = true` plus an issue) is the pressure valve.

Developer commands:

```bash
just snapshot-test                                                  # build vp, run all
just snapshot-test create                                           # substring filter
UPDATE_SNAPSHOTS=1 just snapshot-test create_basic                  # accept changes
cargo test -p vite_cli_snapshots --test cli_snapshots -- create     # direct, if vp is built
```

Thin pnpm wrappers (`pnpm snapshot-test [filter]`) keep DX parity with today's scripts.

## Migration tool

A one-command converter, `tool migrate-snap-tests`, lives next to the old runner in `packages/tools` (TypeScript, because the old format's shell strings are parsed with `@yarnpkg/parsers`, the grammar the old runner actually used):

```bash
tool migrate-snap-tests packages/cli/snap-tests --vp local [name-filter]
tool migrate-snap-tests packages/cli/snap-tests-global --vp global [name-filter]
```

For each old case directory it emits a new fixture under `crates/vite_cli_snapshots/tests/cli_snapshots/fixtures/` and appends to a migration report.

### Field mapping

| Old (`steps.json`)                                 | New (`snapshots.toml`)                                       |
| -------------------------------------------------- | ------------------------------------------------------------ |
| tree location (`snap-tests` / `snap-tests-global`) | `vp = "local"` / `vp = "global"` (from `--vp`)               |
| `commands: [...]`                                  | `steps = [...]` via command translation (below)              |
| `env` (value `""`)                                 | `env` table (`unset-env` entry)                              |
| `ignoredPlatforms: ["win32", {os, libc}]`          | `skip-platforms` (`win32` to `windows`, `darwin` to `macos`) |
| `ignoreOutput: true`                               | `snapshot = false`                                           |
| `timeout`                                          | `timeout`                                                    |
| `serial: true`                                     | dropped, reported (isolation replaces it)                    |
| `localVitePlusPackages: true`                      | `local-registry = true`                                      |
| `linkCheckoutPackages: true`                       | case flag, carried over                                      |
| `after: [...]`                                     | `after` steps                                                |
| fixture files, `mock-manifest.json`, `tarballs/`   | copied verbatim                                              |
| ` # trailing comment` on a command                 | step `comment`                                               |

### Command translation

Each old command string is parsed into a shell AST and translated:

- Simple command: one argv step.
- `a && b && c`: consecutive steps (the snapshot's recorded exit codes now carry the "must succeed" assertion; granularity changes are fine because snapshots are re-recorded anyway).
- Coreutils and helpers (`cat`, `test -f ... && echo`, `mkdir`, `rm`, `ls`, `cp`, `chmod`, `echo`/`printf` redirects, `json-edit`): mapped to `vpt` equivalents.
- `node $SNAP_LOCAL_REGISTRY -- <cmd>`: sets `local-registry = true` and unwraps to `<cmd>`.
- Anything else (pipes, `||`, subshells, unrecognized redirects): emitted as a step with a `TODO(migrate)` comment and listed in the report for hand conversion.

The corpus makes this tractable: of 2,343 commands, 1,403 start with `vp`, and the long tail is dominated by exactly the coreutils patterns above.

### Snapshot re-baselining

Old `snap.txt` files are not converted; the formats measure different things (byte stream vs rendered screens). The migration workflow per batch is:

1. `tool migrate-snap-tests <old-dir> --vp <flavor> <filter>`
2. `UPDATE_SNAPSHOTS=1 just snapshot-test <filter>` to record baselines
3. Review each new `.md` against the old `snap.txt` (this review is well-suited to an agent pass: same commands, same fixture, "does the new snapshot assert everything the old one did")
4. Delete the migrated old case directories in the same PR, so every case lives in exactly one tree at all times

## Decisions

### Rust runner reusing vite-task crates, not a Node reimplementation

A TypeScript runner (node-pty plus a JS vt100 such as `@xterm/headless`) was considered, since the current runner and the local CLI are TS. Rejected because: the milestone protocol, ConPTY ordering quirks, musl PTY crashes, macOS EIO truncation, and the snapshot/diff mechanics are already solved and battle-tested in crates this repo can consume with an existing dependency pattern (git deps on vite-task); node-pty is a native module with its own build/prebuilt matrix; and a Rust `libtest-mimic` target integrates with the workspace's existing `just test` / nextest / xwin CI machinery. The TS side still participates (milestone emission in `packages/prompts`, the migration tool, the local registry), but process orchestration is Rust.

### Dedicated runner crate, not a test target of `vite_global_cli`

vite-task hosts its runner inside the product bin crate (`vite_task_bin`), which is what makes `CARGO_BIN_EXE_vt` available to its tests. Mirroring that here was considered and rejected. Bin targets cannot use dev-dependencies, so `vpt`'s dependencies would become regular dependencies of `vite_global_cli`, growing the product build graph with test-only code; every release build of the package would produce an extra binary that packaging must exclude forever; and the product crate's manifest would stop describing the product. A dedicated `crates/vite_cli_snapshots` (with `publish = false`, excluded from release builds entirely) keeps all of that out of the product. The price is resolving `vp` at runtime from the target directory instead of `env!("CARGO_BIN_EXE_vp")`, plus a build-ordering wrapper recipe; the runtime lookup is also the more robust choice for relocated nextest archives on Windows.

### argv steps and `vpt`, not a shell

The in-process JS shell is one of the old runner's biggest sources of platform drift and hidden semantics. Argv arrays plus a deterministic multitool make every step's behavior identical across platforms and make the snapshot's command headings honest. The cost, translating existing shell one-liners, is paid once by the migration tool.

### Milestones, not wait-for-text or idle detection

Wait-for-text races partial renders (the text can appear before the frame finishes) and breaks silently when copy changes. Idle detection is timing-dependent by definition. Milestones cost a small amount of CLI instrumentation and buy exact synchronization on named render states; vite-task's selector tests demonstrate keystroke-by-keystroke determinism on all three OS families.

### Env-gated milestone emission

vite-task emits milestones unconditionally (they are invisible in real terminals). vp gates emission on `VP_EMIT_MILESTONES=1` because its output is routinely piped, logged, and parsed by other tooling; invisible-in-a-terminal is not the same as absent-from-a-byte-stream. The gate is a single env check per render.

### Real assertions with `UPDATE_SNAPSHOTS`, not regenerate-always

The regenerate model made every run "pass" and outsourced correctness to `git diff` discipline plus a CI retry loop. Compare-by-default with an explicit update flag is how every mainstream snapshot tool works, and it lets CI drop the retry script for this suite entirely.

### One tree with per-case flavor, plus a both-flavors matrix

The global/local split duplicated fixtures and let the two surfaces drift (PR #2031 shipped local snap tests and deferred the global ones as a follow-up). A per-case `vp` field removes the duplication; the `["local", "global"]` form turns parity from a convention into a test.

### Per-case VP_HOME, not shared `~/.vite-plus`

Shared global state forced `serial`, ordering hazards, and the bootstrap byte-match check. Per-case homes cloned from a per-run template make cases order-independent and make the binary under test unambiguous. The cost is provisioning the template once per run, which replaces the `pnpm bootstrap-cli` prerequisite rather than adding to it.

## Alternatives considered

- **Extend `snap-test.ts` with a PTY mode.** Keeps all structural problems (no assertions, shell semantics, shared home, two trees) and still requires solving interactive synchronization, the hardest part, from scratch on node-pty.
- **tmux-driven testing.** Works for manual verification (it is how interactive bugs are caught today) but depends on a host tmux, adds a second emulation layer with its own timing, and has no cross-platform story for Windows CI.
- **`expect`/`expectrl` style pattern-waiting.** Pattern-on-byte-stream is wait-for-text with extra steps; same nondeterminism, plus no rendered-screen snapshots.
- **Keep two trees, add interactivity only to new cases.** Preserves fixture duplication and drift indefinitely and leaves the old corpus on the retry-loop model; the migration tool makes unification cheap enough to not want this.

## Rollout plan

Phase 1, runner: add the git deps (`pty_terminal_test`, `pty_terminal_test_client`, `snapshot_test`), the `crates/vite_cli_snapshots` crate with its `cli_snapshots` test target and `vpt` bin, flavor provisioning, redaction, the `just snapshot-test` recipe, and CI wiring. Land with a handful of hand-written cases covering both flavors, one interactive case, and one `local-registry` case.

Phase 2, instrumentation: milestone emission in `packages/prompts` (clack components) and in the Rust prompt/selector paths. Land the PR #2031 follow-up picker tests and a `vp create` interactive flow as the proof cases, plus the parked watch-restart case.

Phase 3, migration: land `tool migrate-snap-tests`, then migrate in reviewable batches (suggested order: `snap-tests-global` first, since it gains the most from VP_HOME isolation, then `snap-tests`). Old and new suites run side by side in CI during this phase; each batch PR deletes the old cases it migrates.

Phase 4, removal: delete `snap-test.ts`, the retry script, the `snap-test*` package scripts, the old CI legs, and the `snap-tests*/` trees. Update AGENTS.md and CONTRIBUTING.md.

New tests are written in the new format from the moment Phase 1 lands.

## Open questions

1. **Scrollback capture.** A 500-row grid covers almost all cases; for the few commands with longer output, do we capture vt100 scrollback into the snapshot or treat over-long output as a case smell?
2. **Linux parallelism.** vite-task forces `--test-threads=1` on Linux for signal-routing flakiness in ctrl-c tests. With ~529 cases we need that scoped (serialize only signal-sensitive cases) or solved; needs measurement in Phase 1.
3. **Runtime-download cases.** Resolved during Phase 1: each case's `VP_HOME/js_runtime` is seeded via symlink from `VP_SNAP_JS_RUNTIME_DIR` (or the real `~/.vite-plus/js_runtime`), and `seed-runtime = false` opts a case into a genuinely empty home. What remains open is whether runtime-provisioning cases should download from the network in CI or from a local archive fixture.
4. **Build profile.** The wrapper recipe decides which profile `vp` is built with; if a debug-build vp is too slow for install-heavy cases, building it with `--release` (or a dedicated profile) while the runner itself stays on the test profile is the likely answer. The runtime lookup must then resolve the binary from the matching profile directory.
5. **Prompt ids.** The `<kind>:<id>:<state>` naming needs stable `id`s for every interactive call site in `packages/prompts` and the Rust prompts; whether ids are explicit arguments everywhere or derived-with-override is an implementation detail to settle in Phase 2.
