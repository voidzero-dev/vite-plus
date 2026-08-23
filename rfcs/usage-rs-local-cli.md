# RFC: Use usage-rs for the Local Rust CLI

Use [`usage-rs`](https://usage.jdx.dev/rust/) for argument parsing in the local
Vite+ CLI and in Vite Task. Remove `clap` from the NAPI binary dependency graph.

Keep `clap` in the separate global `vp` binary. The global binary must continue
to select a local Vite+ package and forward its arguments without parsing the
local command grammar.

This RFC replaces the parser-library decision in
[`napi-clap-cli-args.md`](./napi-clap-cli-args.md). It does not change that RFC's
command ownership or NAPI result types.

## Motivation

PR #2523 moves five JavaScript command grammars from `mri` to Rust `clap`. This
gives Vite+ strict parsing and one source for parser data and help data. It also
adds `clap` to the NAPI binary.

PR #2534 tests `usage-rs` with the same architecture. `usage-rs` generates
static parser tables at compile time. A successful parse does not construct a
parser graph at run time.

The first local measurements show this result for the `staged` parser:

| Parser                   | Successful parse |
| ------------------------ | ---------------: |
| `mri` on the main design |          1.20 us |
| `clap` in PR #2523       |          6.36 us |
| `usage-rs` in PR #2534   |          1.02 us |

The full CLI takes approximately 130 to 154 ms in the same test. Thus, the
parser improvement does not make process startup measurably faster.

The first `usage-rs` change did not reduce the uncompressed NAPI binary. It was
880 bytes larger than PR #2523. It was 8,074 bytes smaller after `gzip -9`.
`clap` stayed in that dependency graph for these reasons:

- the error path builds a `clap` command to keep the old diagnostics;
- the NAPI command router uses `clap` types;
- `vp_pm_cli` uses `clap` types;
- Vite Task exports `clap` parser types;
- `vt_workspace::PackageQueryArgs` derives `clap::Args`.

A partial parser change cannot test the main size hypothesis. Vite+ must remove
`clap` from the complete NAPI dependency graph before it compares the result.

The complete implementation removes that dependency path. A paired macOS ARM64
build is 94,832 bytes smaller than PR #2523 before compression and 74,392 bytes
smaller after `gzip -9`. Thus, the complete implementation passes the native
size gate.

## Goals

1. Use `usage-rs` for all argument parsing in the local NAPI binary.
2. Remove `clap` from the normal dependency graph of `vite-plus-cli`.
3. Use one typed Rust grammar for parsing, help, diagnostics, and completion.
4. Preserve the strict argument rules from PR #2523.
5. Preserve the NAPI boundary and the JavaScript command behavior.
6. Add direct Rust tests for command grammars, help, errors, and completion.
7. Measure parser speed, full CLI speed, and all distributed artifact sizes.
8. Keep the selected local Vite+ package responsible for its command grammar.

## Non-goals

This RFC does not:

- move JavaScript command work to Rust;
- make the global `vp` binary parse local command options;
- replace `clap` in the global `vp` binary or installer;
- change command names, option names, aliases, defaults, or exit codes;
- install shell completion without an explicit user command;
- require exact diagnostic text from `clap`;
- adopt `usage-rs` if the complete result does not improve the size and
  maintenance trade-off.

## Decision

Use a reviewed `usage-rs` revision based on 6.1.0 for this change. Pin the exact
revision in Vite+ and Vite Task. Both repositories must use the same revision.

`usage-rs` is still experimental. A later update must be an explicit dependency
change with parser, help, completion, performance, and size checks.

Use these features only where they are necessary:

| Feature            | Consumer                                              | Purpose                                                 |
| ------------------ | ----------------------------------------------------- | ------------------------------------------------------- |
| `spec`             | production parser crates                              | Read static command metadata for help                   |
| `help`             | standalone CLI entry points                           | Render built-in help and version output                 |
| `diagnostics`      | NAPI parser and `vt` binary                           | Render user-facing parse errors                         |
| `completions`      | global and local completion providers and `vt` binary | Parse requests, generate scripts, and return candidates |
| `clap-coexistence` | shared `vp_pm_cli` declarations during migration      | Let the same type use gated clap and usage derives      |
| `test`             | development dependencies only                         | Test parse outcomes, help, and completion               |

Do not enable all default features without checking their binary-size cost.

The reviewed usage-rs revision also stores common flag metadata separately from
rare value, help, and relationship metadata. It uses small cold-path sorts for
help and diagnostics. These changes reduce generated code and static table cost.
They do not change the parser result.

## Ownership

| Layer               | Responsibility                                                                     |
| ------------------- | ---------------------------------------------------------------------------------- |
| Global `vp` binary  | Select the local package, merge completion candidates, and forward local arguments |
| Local Node.js CLI   | Apply `-C` and `vpr`, select the local command, and call NAPI                      |
| NAPI command router | Parse local Rust commands and return typed results                                 |
| `js_command_args`   | Parse the five JavaScript command grammars                                         |
| `vp_cli_help`       | Convert static metadata to the shared Vite+ help document                          |
| `vp_pm_cli`         | Parse package-manager command arguments                                            |
| Vite Task           | Parse `run` and `cache` arguments and provide task completion                      |
| JavaScript commands | Apply defaults and run JavaScript operations                                       |

## Architecture

The command path stays the same:

```text
Node.js local CLI
  -> raw argv
  -> NAPI parser
  -> usage-rs static typed parser
  -> typed NAPI result
  -> JavaScript or Rust command operation
```

The local dependency path changes:

```text
vite-plus-cli NAPI binary
  +-> js_command_args -> usage-rs
  +-> local command router -> usage-rs
  +-> vp_pm_cli -> usage-rs
  +-> vt -> usage-rs
  +-> vt_workspace -> parser-neutral package query data
  +-> vp_cli_help -> usage-rs spec

clap: no normal dependency path
```

The global path stays separate:

```text
global vp binary
  -> clap parses global options
  -> select local vite-plus package
  -> forward raw local argv
  -> local Node.js and NAPI parser
```

This split prevents a new global binary from rejecting options that belong to
an older local package, or the reverse.

## Strict parser contract

Each root `usage-rs` grammar must set these policies:

```rust
#[usage(
    unknown_flags = "error",
    args_override_self = false
)]
```

`usage-rs` accepts unknown flags and repeated scalar flags by default. These
attributes are necessary to preserve the strict contract from PR #2523.

Keep these rules:

- reject unknown options;
- reject missing values;
- reject invalid positional arguments;
- reject repeated scalar options unless the command defines an override;
- preserve all values after `--` where the command forwards them;
- keep documented short and long aliases;
- accept only explicit negative flags;
- let the last positive or negative Boolean form take effect where specified;
- reject empty values when the previous JavaScript parser rejected them.

Use `FromStr`, `ValueEnum`, `choices`, and validation functions for typed values.
Do not parse a value again in JavaScript.

## Diagnostics

Remove the cold `clap` diagnostic adapter from
`packages/cli/binding/src/js_command_args/parse.rs`.

Use `usage-rs` diagnostics and `render_failure` for all structural and value
errors. Convert the structured error to the existing `CliParseError` kind. Keep
the existing NAPI result union:

```text
ok    -> typed command value
exit  -> help or another successful early exit
error -> error kind and rendered diagnostic
```

Rust prints help. JavaScript keeps process lifetime and error exit ownership.
The Vite+ header and final exit codes must not change.

Diagnostic wording can change from `clap` wording. Snapshot changes must be
reviewed. Tests must assert the error category, the invalid token, the relevant
option, and the help hint. They must not depend on unrelated punctuation.

## Help

Keep the shared `vp_cli_help` renderer. Build its document from the
`usage-rs` static `Spec` and `CommandMeta` values.

One grammar must define these values:

- option and subcommand names;
- aliases;
- value names;
- short and long help;
- groups and headings;
- choices and defaults;
- visibility;
- command usage.

Do not keep a second help-only option table. A command can still add examples
and documentation links because those values do not change the grammar.

The help output must still use a stacked layout when an option label consumes
the available terminal width.

## Completion

Enable the `completions` feature and add `#[usage(completion)]` to the local root
grammar. The completion implementation must use the same static parser tables
as normal parsing.

When an embedded caller uses `parse_from`, it must call `completion_request`
before normal parsing. The hidden `__complete_word__` request is a protocol
message, not a user command.

The completion call graph is:

```text
shell completion script
  -> global vp completion request parser
  +-> clap_complete candidates for global-only options
  +-> select the same local vite-plus package as command execution
      -> local Node.js completion entry
      -> NAPI completion request
      -> usage-rs completion engine
      -> neutral tab-delimited candidates and file markers
  -> merge and de-duplicate candidates
  -> usage-rs shell renderer
  -> shell
```

Keep the current global completion setup command and startup-file behavior. The
global binary can keep `clap_complete` for global-only options. It must delegate
local command candidates to the selected local package.

Generate scripts for Bash, Zsh, Fish, Nu, and PowerShell. Use
`completion_script_for_alias` for `vpr`, or provide an equivalent alias view
that inserts the `run` command before completion.

The generated POSIX startup file must not evaluate the Bash completion script
when Bash runs in POSIX mode as `sh`. Bash process substitution is not valid in
that mode.

Use static completion for command names, flags, aliases, and value choices. Use
custom completers for data that is available only at run time:

- configured Vite Task names;
- package names and package filters;
- file or directory values when the grammar has a path hint.

Completion must not run a task or change a project. A dynamic completer can read
the same configuration that the current `run_tasks_completions` function reads.

## Vite Task changes

Vite Task must replace `clap` in its production CLI crates before Vite+ updates
the pinned git revision.

The draft implementation is [Vite Task PR #694](https://github.com/voidzero-dev/vite-task/pull/694).

The upstream change must:

1. derive `Cli`, `Args`, `Subcommands`, and `ValueEnum` on the Vite Task parser
   types;
2. parse the standalone `vt` binary with `usage-rs`;
3. parse intercepted `vt`, `vp`, and `vpr` commands with the same grammar;
4. keep `RunCommand`, `RunFlags`, `CacheSubcommand`, and `LogMode` as typed public
   API values;
5. keep the rule that arguments after a task name pass to the task unchanged;
6. keep `--cache` and `--no-cache` mutually exclusive;
7. expose completion for `run`, `cache`, log modes, task names, and package
   selectors;
8. remove production `clap` dependencies from `vt` and `vt_workspace`;
9. migrate the `vt_plan` snapshot-test argument parser, or keep `clap` only as a
   test dependency with a documented reason.

`PackageQueryArgs` is domain input as well as parser input. Its fields must not
force downstream crates to depend on one parser framework. Prefer a public
constructor or a parser-neutral input type. The parser can convert its parsed
values to that type.

The upstream PR must land before Vite+ points its git dependencies at the new
revision. During review, PR #2534 can use the pushed upstream commit by exact
revision.

## Vite+ command router and package manager

Replace the remaining NAPI `clap` roots in `binding/src/cli/` and
`binding/src/exec/args.rs` with `usage-rs` types.

`vp_pm_cli` is shared by the global and local binaries. Keep its command action
types parser-neutral. Put parser-specific derives and entry points behind
features if the two artifacts need different parsers:

```text
vp_pm_cli
  +-> clap-parser feature  -> global vp binary
  +-> usage-parser feature -> local NAPI binary
```

Do not enable both parser features in the NAPI artifact. Cargo unifies features
for one build graph. Artifact checks must use the same package and feature set
as the distributed NAPI build.

## Unit tests

Use `usage-rs` with the `test` feature in development dependencies. Prefer its
process-free test helpers for one grammar. A composite completion provider can
test `Request`, `complete`, and `render` directly because it merges more than
one grammar.

Each parser module must directly test:

- successful values;
- each documented short option;
- each documented long option;
- aliases;
- unknown options;
- missing and empty values;
- repeated scalar values;
- allowed repeated values;
- positive and negative Boolean order;
- `--` forwarding;
- help and missing-subcommand outcomes;
- error kinds and stable diagnostic content.

Use `usage::test::parse` or `usage::test::outcome` for parser tests. Use help-page
and `help_tree` snapshots for metadata and help coverage.

Completion tests must cover:

- root command and subcommand candidates;
- short and long options;
- value choices;
- a partial task name;
- a package-filter value;
- `vpr` alias behavior;
- tokens before and after `--`;
- all supported shells at script-generation level.

During migration, run one table of representative argument vectors through the
old `clap` parser and the new `usage-rs` parser. Compare the typed values or the
error category. Remove this dual-parser test when `clap` leaves the production
and test dependency graphs.

Keep the NAPI contract tests and PTY snapshots. These tests cover the language
boundary, output streams, terminal width, exit codes, and selected local-version
behavior.

Vite Task must test its parser without starting a process. It must also keep its
existing end-to-end snapshots for task argument forwarding and cache commands.

## Dependency checks

Add or document these checks for the distributed NAPI configuration:

```bash
cargo tree -p vite-plus-cli -i clap --edges normal
cargo tree -p vite-plus-cli -i usage-rs --edges normal
```

The first command must report that `clap` does not match a package in the normal
dependency graph. The second command must show only expected parser and help
consumers.

In Vite Task, check the production crates separately from test-only dependencies.
No production path from `vt` or `vt_workspace` can require `clap`.

## Performance and size checks

The parser benchmarks ran on macOS ARM64 with Node.js 22.22.0. The successful
case used 100,000 warm-up calls and ten batches of 200,000 calls. The error,
help, and completion cases used at least 20,000 warm-up calls and ten batches of
50,000 calls. The full CLI cases used 25 separate processes after four warm-up
runs.

The successful parser input was
`--allow-empty --concurrent=2 --diff-filter ACMR --no-stash`. Help generation
includes parsing `--help` and building the shared help document. It does not
include terminal output. Static completion includes request parsing, candidate
selection, and rendering.

| Metric                          |   main / `mri` | PR #2523 / `clap` | complete `usage-rs` change |
| ------------------------------- | -------------: | ----------------: | -------------------------: |
| Successful parser call          |        1.20 us |           6.36 us |                   1.007 us |
| Unknown option                  | Not comparable |           4.42 us |                   2.158 us |
| Parse and build staged help     |  Not available |         21.572 us |                   1.977 us |
| Static staged completion        |  Not available |     Not available |                   1.571 us |
| `vp --version`                  |      130.76 ms |         130.38 ms |                   123.8 ms |
| `vp staged --help`              |      140.27 ms |         132.42 ms |                   124.4 ms |
| `vp staged --unknown`           | Not comparable |      Not measured |                   125.1 ms |
| Static completion process       |  Not available |     Not available |                   122.9 ms |
| Dynamic task completion process |  Not available |     Not available |                   125.3 ms |

The complete usage-rs parser is 84% faster than the Clap parser for successful
input. It is 51% faster for an unknown option. Parse and help-document creation
is 10.9 times faster. The complete CLI stays within process-startup noise.

The final CLI measurements ran in a later local session than the earlier paired
`mri` and Clap measurements. Use them to confirm the startup-scale result, not
as a claim that this change saves 6 to 17 ms.

The native size gate used paired release builds with the same source checkout,
toolchain, target, profile, and machine:

| macOS ARM64 NAPI library | PR #2523 / `clap` | Complete `usage-rs` |     Change |
| ------------------------ | ----------------: | ------------------: | ---------: |
| Binary                   |      41,869,648 B |        41,774,816 B |  -94,832 B |
| `gzip -9`                |      17,221,832 B |        17,147,440 B |  -74,392 B |
| Mach-O `__text`          |      19,438,000 B |        19,207,460 B | -230,540 B |

The raw binary is 0.23% smaller. The compressed binary is 0.43% smaller. The
machine code is 1.19% smaller. This result passes the directional size gate,
but it is not a material package-size improvement. Binary size alone does not
justify the migration.

The package JavaScript does not change after PR #2523. The canonical Linux
artifact workflow for that PR supplies the exact dist totals. The complete
usage-rs change keeps those files unchanged:

| Distributed JavaScript                  | main / `mri` | PR #2523 / `clap` | Complete `usage-rs` |
| --------------------------------------- | -----------: | ----------------: | ------------------: |
| `packages/cli/dist`                     |  1,685,719 B |       1,666,423 B |         1,666,423 B |
| `packages/core/dist`, excluding `.node` |  4,097,460 B |       4,097,460 B |         4,097,460 B |
| Combined dist                           |  5,783,179 B |       5,763,883 B |         5,763,883 B |

The complete artifact is smaller than PR #2523, and all compatibility tests
pass. Therefore, the result meets the directional adoption condition. The main
reasons to adopt this change are the unified grammar, completion support, and
lower parser cost.

## Migration order

1. Add this RFC to PR #2534.
2. Review [Vite Task PR #694](https://github.com/voidzero-dev/vite-task/pull/694),
   which replaces its production `clap` parsers.
3. Add Vite Task parser, help, completion, and equivalence tests.
4. Point PR #2534 at the pushed Vite Task commit.
5. Migrate the local Vite+ command router and `vp_pm_cli` usage path.
6. Remove the cold `clap` diagnostic adapter and direct NAPI dependency.
7. Run dependency, unit, NAPI, snapshot, completion, performance, and size checks.
8. Update the PR #2534 description with the complete results.

Each step must keep typed command operations separate from parser-framework
types. A temporary commit can contain both parsers for equivalence tests. The
final NAPI build cannot contain both.

## Risks

### Experimental dependency

`usage-rs` can change while its API develops. Exact revision pins and focused
upgrade checks limit this risk.

The compact metadata work changes the public `FlagMeta` struct layout while it
keeps field reads compatible through `Deref`. External struct literals are not
source-compatible. The upstream project must either publish this layout in a
breaking release or provide a compatible construction API before Vite+ updates
to a published crate. Vite+ and Vite Task must pin the reviewed revision until
that decision is complete.

### Diagnostic differences

The native renderer does not produce byte-for-byte `clap` messages. Stable
semantic assertions and reviewed PTY snapshots protect the user contract.

### Argument forwarding

Vite Task forwards all tokens after the task name. A parser can accidentally
consume a task option such as `-v`. Direct parser tests and end-to-end task
snapshots must cover this boundary.

### Completion version skew

The global binary can differ from the selected local package. Delegating local
completion to that package keeps its candidates consistent with its parser.

### Cargo feature unification

Shared crates can enable both parser features in one graph. Package-specific
artifact builds and `cargo tree` checks must verify the actual distributed
binary.

### Size does not improve

Diagnostics and completion add code. The complete binary can stay the same size
or grow after `clap` is removed. In that case, keep PR #2523 as the baseline and
do not adopt this RFC without a new trade-off decision.

## Alternatives

### Keep PR #2523

This is the fallback. `clap` is mature, complete, and already used by the global
binary. It has a larger parser-only cost in the current measurement.

### Keep the partial adapter in PR #2534

This gives fast successful parses, but it keeps two parser libraries in the NAPI
graph. It does not test the expected size benefit and has extra adapter code.

### Write a custom parser

A custom parser can be small and fast. Vite+ would then own strict parsing,
diagnostics, help metadata, completion metadata, and all edge cases. That cost is
larger than the current command-parser scope.

### Generate parsers from a neutral schema

A neutral schema can feed both `clap` and `usage-rs`. It adds a generator and a
new schema language. `usage-rs` already exposes static metadata from typed Rust
definitions, so a second schema is not necessary for this experiment.

## Acceptance criteria

The implementation is complete when:

- all local command grammars use `usage-rs`;
- Vite Task production CLI crates use `usage-rs`;
- `clap` has no normal path to the distributed NAPI binary;
- the global `vp` binary still forwards local arguments without parsing them;
- help, diagnostics, aliases, strict parsing, negation, repetition, and `--`
  behavior pass focused and PTY tests;
- static and dynamic completion tests pass;
- Vite Task task-argument forwarding tests pass on Unix and Windows;
- the PR reports parser, CLI, NAPI, gzip, and dist measurements;
- the complete artifact result is better than the PR #2523 baseline;
- PR #2534 links the Vite Task PR and pins a reviewed upstream commit.

## References

- [`usage-rs` Rust quickstart](https://usage.jdx.dev/rust/quickstart)
- [`usage-rs` migration from clap](https://usage.jdx.dev/rust/migrating-from-clap)
- [`usage-rs` help and diagnostics](https://usage.jdx.dev/rust/help)
- [`usage-rs` completion](https://usage.jdx.dev/rust/completions)
- [`usage-rs` testing](https://usage.jdx.dev/rust/testing)
- [`usage-rs` validation](https://usage.jdx.dev/rust/validation)
- [`usage-rs` parser specification](https://usage.jdx.dev/rust/spec)
- [`usage-rs` performance notes](https://usage.jdx.dev/rust/performance)
- [RFC: Use clap to Parse Arguments for JavaScript Commands](./napi-clap-cli-args.md)
