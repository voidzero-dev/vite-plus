# CLI snapshot tests (PTY runner)

This is the snapshot test suite for the `vp` CLI. Every step runs inside a
real pseudo-terminal backed by a vt100 emulator, so interactive flows
(prompts, pickers, watch modes, ctrl-c) are first-class testable surfaces.
Snapshots are Markdown files with real pass/fail semantics.

**Write new CLI tests here.** The legacy trees (`packages/cli/snap-tests/`,
`packages/cli/snap-tests-global/`) are being migrated to this suite and
must not receive new cases. Design rationale: `rfcs/interactive-snapshot-tests.md`.

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
cargo test -p vite_cli_snapshots -- <filter>      # if vp is already built
```

Trial names are `<fixture>::<case>` (plus `::<flavor>` for multi-flavor
cases). Prerequisites: both flavors need `cargo build -p vite_global_cli`
(the `just` recipe does it); the local flavor also needs a built
`packages/cli/dist` (`pnpm build`). The runner fails fast when `dist` is
older than `src`, so a forgotten rebuild never silently tests stale
local-CLI code.

Environment overrides, mainly for CI:

| Variable                    | Effect                                                              |
| --------------------------- | ------------------------------------------------------------------- |
| `VP_SNAP_GLOBAL_VP`         | Path to a prebuilt global `vp` binary (skips the target-dir lookup) |
| `VP_SNAP_LOCAL_CLI_BIN_DIR` | Local CLI bin dir (default `<repo>/packages/cli/bin`)               |
| `VP_SNAP_JS_RUNTIME_DIR`    | Provisioned managed runtime to seed case homes with                 |
| `VP_SNAP_SKIP_FLAVORS`      | Comma-separated flavors to skip registering (e.g. `local`)          |

## Case reference

```toml
[[case]]
name = "my_case"              # [A-Za-z0-9_]+, names the snapshot file
vp = "local"                  # "local" | "global" | ["local", "global"]
comment = "What this proves." # rendered into the snapshot
cwd = "packages/app"          # optional, relative to the fixture root
skip-platforms = ["windows"]  # or { os = "linux", libc = "musl" }
ignore = false                # true: only runs with `-- --ignored`
seed-runtime = true           # false: start from an empty VP_HOME
env = { MY_VAR = "1" }        # case-wide env additions
unset-env = ["SOME_VAR"]      # remove baseline env entries
steps = [ ... ]
after = [ ... ]               # cleanup steps, never snapshotted
```

`vp` picks which CLI runs the case. Both flavors install the built Rust binary
into the case's `VP_HOME/current/bin`, install the checkout package under that
case home, and run `vp env setup` before steps. `"global"` exposes only
`VP_HOME/bin`; `"local"` also exposes the case-local
`VP_HOME/current/node_modules/vite-plus/bin` package bin. On Windows, local
flavor exposes sibling `.cmd` shims under
`VP_HOME/current/node_modules/.vite-plus-bin` instead. The list form registers
one trial and one snapshot per flavor; use it for parity cases (help output,
routing, error messages) where both surfaces must agree.

A step is a bare argv array or a table:

```toml
{ argv = ["vp", "create"],
  cwd = "sub",                # per-step working dir
  comment = "...",            # rendered under the step heading
  envs = [["K", "V"]],        # per-step env
  timeout = 120000,           # ms, default 50s
  snapshot = false,           # omit the screen while the step succeeds
                              #   (failures always keep their output)
  continue-on-failure = true, # a failing step skips the rest of its line,
                              #   up to the next continue-on-failure step;
                              #   with none ahead the case stops (shell &&)
  tty = false,                # piped stdio instead of a PTY (non-TTY tests)
  interactions = [ ... ] }
```

`argv[0]` may be `vp`, `vpr`, `vpx`, `vpt`, `oxfmt`, `oxlint`, `node`,
`git`, `npm`, `pnpm`, `yarn`, or `bun`, but everything except `vpt` must
resolve from the flavor-owned tool directories described above. There is no
shell: no `&&`, no redirects, no globs. File setup and assertions go through
`vpt` so behavior is identical on every platform:

`vpt print-file` (cat), `vpt stat-file` (prints `file`/`dir`/`missing`;
`--assert <state>` / `--assert-not <state>` also fail on mismatch, so
`test -f x && cmd` guards keep their short-circuit), `vpt write-file`,
`vpt touch-file`, `vpt replace-file-content`, `vpt list-dir`, `vpt mkdir`,
`vpt rm`, `vpt cp`, `vpt chmod`, `vpt grep-file`, `vpt json-edit`,
`vpt pipe-stdin <data> -- <argv>`, plus task payloads for `vp run` tests:
`vpt print`, `vpt print-color`, `vpt print-env`, `vpt print-cwd`,
`vpt print-native-path` (prints OS-native separators, for redaction
self-tests), `vpt check-tty`, `vpt read-stdin`, `vpt exit <code>`,
`vpt exit-on-ctrlc`, `vpt barrier`.

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
    { "expect-milestone" = "select:template:0" },  # wait, then capture screen
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
case-owned tool dirs, then a system tail for child processes),
`TERM=xterm-256color`, `VP_CLI_TEST=1`, `VP_EMIT_MILESTONES=1`, a fresh
`HOME`, `VP_HOME`, and npm prefix. The runner still rejects direct step tools
that resolve outside the case-owned dirs; `vpt` is the only runner helper on
PATH, and the system tail is only for child processes such as `git` inside real
`vp create` flows. `CI` and `NO_COLOR` are deliberately NOT set: with a PTY
attached, the CLI behaves interactively by default, which is the point.
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
a case produces nondeterministic
output, fix it with a milestone or a redaction rule; never rerun until
green. Set
`formatted-snapshot = true` on a step only when the test is about colors.

Fixture trees are excluded from repo-wide fmt, lint, typecheck, and vitest
(`vite.config.ts`, `tsconfig.json`); recorded snapshots and
`snapshots.toml` are runner metadata and never appear inside the staged
workspace a test runs in.

## Migrating a legacy case

```bash
node packages/tools/src/bin.js migrate-snap-tests packages/cli/snap-tests --vp local <name-filter>
UPDATE_SNAPSHOTS=1 just snapshot-test <name_filter>
# review each new .md against the deleted snap.txt in git diff, then commit
```

The migrator converts `steps.json` fields, splits `&&` chains, maps shell
built-ins to `vpt`, removes cleanly converted (TODO-free) old case
directories (`--keep-old` defers that; git history keeps the originals),
and reports anything needing hand conversion in `MIGRATION-REPORT.md`
(generated, gitignored). Cases with TODOs keep their legacy dir until the
hand conversion lands, so placeholder steps never silently replace real
coverage. A case whose target fixture already exists is skipped and
reported: the same name in both legacy trees means a hand merge, usually a
second `[[case]]` in the fixture or a `vp = ["local", "global"]` matrix.
