# RFC: Use clap to Parse Arguments for JavaScript Commands

## Summary

The local Vite+ CLI runs in Node.js. It uses JavaScript for five commands:

- `create`
- `migrate`
- `config`
- `hooks`
- `staged`

This RFC moves argument parsing for these commands to Rust `clap`. The existing NAPI binding connects Node.js to Rust.

JavaScript continues to run each command. It keeps prompts, file operations, and calls to JavaScript libraries.

The global Rust CLI continues to forward the arguments without parsing them. This rule protects compatibility with different local Vite+ versions.

## Terms

| Term                | Meaning                                                         |
| ------------------- | --------------------------------------------------------------- |
| Argument grammar    | The valid options, values, aliases, and positions for a command |
| Local CLI           | The Node.js CLI from the selected `vite-plus` package           |
| NAPI binding        | The native interface between Node.js and Rust                   |
| Positional argument | A value that does not have an option name                       |
| Parse outcome       | A typed object with an `ok`, `exit`, or `error` status          |

## Motivation

The local entry point is `packages/cli/src/bin.ts`. It sends most commands to the Rust CLI through NAPI.

The five commands in this RFC run in JavaScript. Each command uses `mri` and local TypeScript code to parse arguments.

Thus, Vite+ has two argument parsers:

- Rust commands use `clap`.
- JavaScript commands use `mri`.

[Issue #2488](https://github.com/voidzero-dev/vite-plus/issues/2488) showed a defect in this design. `mri` returns `false` for `--no-concurrent`.

The old `staged` code converted `false` to `0`. It then gave `concurrent: 0` to `lint-staged`.

The task queue did not start. [PR #2501](https://github.com/voidzero-dev/vite-plus/pull/2501) added JavaScript checks for this value.

That fix protects `staged`, but the two parsers remain. The other commands have related risks:

- `vp config --no-hooks-dir` can return `false` where JavaScript expects a path.
- `mri` accepts many unknown options and unused positional arguments.
- `create` and `migrate` use TypeScript assertions on values from `mri`.
- TypeScript assertions do not check values at run time.
- Each command defines its own rules for negation and repeated options.

Vite+ already includes a native binding and `clap`. This design uses those components and adds no new runtime parser.

## Goals

1. Use `clap` to parse the five JavaScript command grammars.
2. Return typed values through generated NAPI declarations.
3. Build and print option help from the same `clap` command.
4. Reject invalid arguments before command work starts.
5. Keep the local CLI in Node.js.
6. Keep command work in JavaScript.
7. Preserve all template arguments after `--` for `vp create`.
8. Keep environment defaults and file-system rules in JavaScript.
9. Migrate one command at a time.
10. Migrate `staged` first.

## Non-goals

This RFC does not:

- move command work to Rust;
- replace the Node.js local CLI;
- make the global CLI parse local command options;
- send these commands through the existing `run()` executor;
- replace `renderCliDoc()` for commands outside this migration;
- generate Rust schemas from TypeScript;
- add a neutral schema language;
- change `-C`, `vpr`, command routing, or package-manager routing.

## Ownership

| Layer                           | Responsibility                                             |
| ------------------------------- | ---------------------------------------------------------- |
| Global `vp` binary              | Select the local package and forward the command arguments |
| Local `packages/cli/src/bin.ts` | Apply `-C` and `vpr` rules, then select the command        |
| `binding/src/js_command_args/`  | Parse command arguments and request help output            |
| `vp_cli_help`                   | Build, format, and print Rust-backed help                  |
| `packages/cli/src/help.ts`      | Keep static help for tool-backed commands                  |
| JavaScript command modules      | Apply defaults and run command operations                  |

The global CLI must keep these command arguments as `Vec<String>`. It must not parse the local option grammar.

A global binary can run an older or newer local package. Global parsing can reject a valid option from that package.

## Design

```text
process.argv
    |
    v
local Node.js CLI
packages/cli/src/bin.ts
    |
    | raw arguments for one JavaScript command
    v
NAPI parser function
packages/cli/binding/src/js_command_args/
    |
    v
clap command grammar
    |
    +-- checks aliases and option boundaries
    +-- converts values
    +-- checks explicit negation
    +-- rejects unknown options
    +-- rejects invalid positional arguments
    |
    v
validated Rust arguments
    |
    v
typed NAPI parse outcome
    |
    v
JavaScript command operations
```

JavaScript sends raw arguments to one NAPI parser. It does not parse the returned values again.

## Rust module structure

Add these files:

```text
crates/vp_cli_help/
  Cargo.toml
  src/lib.rs

packages/cli/binding/src/js_command_args/
  mod.rs
  parse.rs
  create.rs
  migrate.rs
  config.rs
  hooks.rs
  staged.rs
```

The `vp_cli_help` crate contains the shared help model, `clap` adapter, formatter, and output function. The global CLI and the NAPI binding use this crate.

The `js_command_args` name shows that these files support JavaScript commands. The existing `binding/src/cli/` module parses and runs Rust commands.

`parse.rs` contains the shared parser and error conversion. Each command file contains these items:

- its `clap` type;
- its value conversion;
- its NAPI output type;
- its focused tests.

`create` and `migrate` can share a private setup type. Share a field only when both commands use the same rules.

## Shared clap parser

Each command type derives `clap::Args`. Each command file adds that type to a configured `clap::Command`.

A shared function parses with that command.

```rust
fn try_parse_args<T>(
    mut command: clap::Command,
    argv: Vec<String>,
) -> Result<T, clap::Error>
where
    T: clap::Args + clap::FromArgMatches,
{
    let bin_name = command.get_name().to_owned();
    let mut matches = command.try_get_matches_from_mut(
        std::iter::once(bin_name).chain(argv),
    )?;
    T::from_arg_matches_mut(&mut matches)
}
```

The implementation can change Rust ownership details. It must keep these rules:

- Use the same parse path for each command.
- Add a synthetic argument at index zero.
- Use a fallible match conversion.
- Return parse errors as data.
- Do not call `get_matches`, `parse`, `exit`, or a print function.

Tests can call this function directly. They do not need to change `process.argv`.

See these `clap` APIs:

- [`Args::augment_args`](https://docs.rs/clap/latest/clap/trait.Args.html)
- [`Command` reflection methods](https://docs.rs/clap/latest/clap/struct.Command.html#method.get_arguments)
- [`Arg` reflection methods](https://docs.rs/clap/latest/clap/builder/struct.Arg.html#method.get_help)
- [`FromArgMatches`](https://docs.rs/clap/latest/clap/trait.FromArgMatches.html)
- [fallible `Parser` methods](https://docs.rs/clap/latest/clap/trait.Parser.html)

## NAPI contract

Export one synchronous function for each command:

```ts
parseStagedArgs(argv: string[]): ParseStagedArgsOutcome
parseConfigArgs(argv: string[]): ParseConfigArgsOutcome
parseHooksArgs(argv: string[]): ParseHooksArgsOutcome
parseMigrateArgs(argv: string[]): ParseMigrateArgsOutcome
parseCreateArgs(argv: string[]): ParseCreateArgsOutcome
```

Each function returns a command-specific union:

```ts
type ParseStagedArgsOutcome =
  | { status: 'ok'; value: StagedArgs }
  | { status: 'exit'; code: number }
  | { status: 'error'; error: CliParseError };

interface CliParseError {
  kind: string;
  message: string;
}
```

A union has one object shape for each status. napi-rs generates this union from a structured Rust enum.

The statuses have these meanings:

- `ok`: The value contains valid arguments.
- `exit`: Rust printed help. JavaScript exits with the specified code.
- `error`: JavaScript prints the `clap` diagnostic.

Map `clap::error::ErrorKind::DisplayHelp` to `exit` with code 0. Map all other argument errors to `error`.

JavaScript prints the Vite+ header before a diagnostic. It exits with code 1 for invalid arguments.

Rust does not call `clap::Error::exit()` or `std::process::exit()`. JavaScript controls the Node.js process lifetime.

Do not throw a NAPI exception for user input. A native exception must identify a binding or conversion failure.

Do not return `serde_json::Value`. The generated `index.d.cts` file must show each field and union member.

See these napi-rs references:

- [NAPI attributes](https://napi.rs/docs/concepts/napi-attributes)
- [NAPI objects](https://napi.rs/docs/concepts/object)
- [NAPI error handling](https://napi.rs/docs/concepts/error-handling)

## Rust and JavaScript types

The `clap` type defines the parser rules. A small NAPI type converts Rust-only values for JavaScript.

```text
clap StagedCliArgs
    concurrent: Option<Concurrent>
              |
              v
NAPI StagedArgs
    concurrent?: boolean | number
```

Use these rules for NAPI output:

- Use `Option<T>` when the user does not specify a value.
- Do not set TTY or environment defaults in Rust.
- Use camel-case JavaScript names.
- Use generated TypeScript declarations.
- Keep Rust-only enums inside the parser.
- Use integers that JavaScript can represent exactly.

JavaScript can keep types for larger command inputs. Do not use an assertion to hide an unchecked parser value.

## Help

Use `vp_cli_help` as the Rust help model and renderer.

`clap` owns the `-h` and `--help` actions. The parser reports either action as `DisplayHelp`.

The NAPI function builds a help document from the command metadata. Rust prints the Vite+ header and help document to stdout. It then returns `exit` with code 0.

The shared adapter reads public metadata from the built `clap::Command`. It reads these values:

- usage;
- command summary;
- visible arguments;
- visible subcommands;
- short and long names;
- value names;
- help descriptions;
- help headings.

The global CLI uses the same document types and formatter. The formatter keeps help text within the terminal width. JavaScript does not receive the document or the rendered text.

The upstream tool help workflow does not change. `dev`, `build`, `preview`, `test`, `lint`, `fmt`, and `pack` continue to use `commandHelpDocs` and `renderCliDoc()`.

The daily dependency upgrade continues to use `.claude/skills/sync-upstream-cli-help/SKILL.md`. This RFC does not move these static documents to `clap` or `vp_cli_help`.

A help flag takes priority over a later invalid option. This behavior matches the current JavaScript command.

The `clap` schema owns option names, value names, aliases, and descriptions. JavaScript does not keep a second option list.

Documentation URLs and custom examples do not define accepted arguments. A command can add this presentation data before Rust prints the document.

The shared adapter uses the `clap::Arg` display form for option labels. This form shows optional values with square brackets.

## Strict parsing and negation

The new schemas reject these inputs:

- unknown options;
- invalid positional arguments;
- missing values;
- repeated scalar options, unless this RFC defines another rule;
- `--no-*` forms that the schema does not define.

This behavior is stricter than `mri`. The documented CLI defines the supported inputs.

Define each supported negative option in `clap`. Use overrides when a positive and negative form set the same value.

Use these repetition rules:

| Option                             | Rule                                 |
| ---------------------------------- | ------------------------------------ |
| Repeated `--agent <name>`          | Keep all names in command-line order |
| `--agent` and `--no-agent`         | The last form sets the state         |
| A new `--agent` after `--no-agent` | Start a new enabled selection        |
| Repeated `--editor <name>`         | Keep the last value                  |
| Positive and negative booleans     | The last form sets the state         |
| Other repeated scalar options      | Return an argument conflict          |

Do not add automatic negation. For example, reject `--no-cwd`, `--no-diff`, and `--no-hooks-dir`.

## Command grammars

### `staged`

Migrate `staged` first. It tests these parser features:

- aliases;
- explicit negation;
- optional values;
- custom value parsing;
- string options;
- Boolean options.

Use this Rust type for concurrency:

```rust
enum Concurrent {
    Enabled,
    Disabled,
    Limit(std::num::NonZeroU32),
}
```

Set `num_args = 0..=1` on `--concurrent`. Set `default_missing_value = "true"`.

Define `--no-concurrent` as a separate argument. Make each concurrency form override the earlier form.

See [the `clap::Arg` documentation](https://docs.rs/clap/latest/clap/builder/struct.Arg.html) for optional values.

Accept these inputs:

```text
--concurrent
--no-concurrent
--concurrent true
--concurrent false
--concurrent=4
-p 4
```

Reject these inputs before JavaScript calls `lint-staged`:

```text
--concurrent=0
--concurrent=-1
--concurrent=1.5
--concurrent=NaN
--concurrent=4294967296
```

Use `NonZeroU32`, not `NonZeroUsize`. `NonZeroU32` has the same range on each supported platform.

JavaScript can represent all `u32` values exactly. A fractional task count has no clear scheduler meaning.

PR #2501 accepts all positive finite JavaScript numbers. This RFC intentionally rejects fractional values.

Return `concurrent?: boolean | number`. Return other `lint-staged` options with camel-case names.

Use `Option` fields so JavaScript sends only explicit values to `lint-staged`.

The schema includes these options:

```text
--allow-empty
-p, --concurrent [number|boolean]
--no-concurrent
--continue-on-error
--cwd <path>
-d, --debug
--diff <string>
--diff-filter <string>
--fail-on-changes
--hide-partially-staged
--hide-unstaged
--no-stash
-q, --quiet
-r, --relative
--revert
-v, --verbose
-h, --help
```

`--no-stash` is the supported negative stash option. Do not infer `--stash` or `--no-debug` from `mri`.

`packages/cli/src/staged/bin.ts` reads the NAPI outcome. It converts an `ok` value to `lint-staged` options.

Remove `packages/cli/src/staged/args.ts` when it has no callers.

### `config`

Use these options:

```text
--hooks-dir <path>
--hooks / --no-hooks
--agent / --no-agent
-h / --help
```

Keep the positive `--hooks` and `--agent` forms. These forms match the current defaults, but the current parser accepts them.

Reject `--no-hooks-dir`. Reject `--hooks-dir` when it has no path.

Rust returns the hooks-directory string without changes. JavaScript keeps these operations:

- Git lookup;
- path rules;
- lifecycle-event handling;
- prompts;
- environment opt-outs.

### `hooks`

Define `enable`, `disable`, and `status` as `clap` subcommands. Put `--hooks-dir <path>` on each subcommand.

The current grammar puts `--hooks-dir` after the subcommand. Keep this order.

`vp hooks` and help flags show the current top-level help. Reject unknown subcommands, unknown options, and extra positional arguments.

Reject invalid arguments before `enable` or `disable` changes the repository. Remove `packages/cli/src/hooks/args.ts` after the migration.

### `migrate`

Use this grammar:

```text
vp migrate [PATH] [OPTIONS]
```

Accept zero or one path. Reject additional positional arguments.

Use the repetition rules from this RFC for these options:

```text
--agent <name> / --no-agent
--editor <name> / --no-editor
--hooks / --no-hooks
--interactive / --no-interactive
--full
-h / --help
```

Return the path without changes. JavaScript resolves it against `process.cwd()`.

Return `interactive?: boolean`. JavaScript calculates `parsed.interactive ?? defaultInteractive()`.

JavaScript keeps agent and editor lookup. The JavaScript catalogs define compatibility aliases.

### `create`

Migrate `create` last. It has this grammar:

```text
vp create [TEMPLATE] [OPTIONS] [-- TEMPLATE_OPTIONS]
```

Vite+ accepts these options before `--`:

```text
--directory <dir>
--agent <name> / --no-agent
--editor <name> / --no-editor
--git / --no-git
--hooks / --no-hooks
--package-manager <pnpm|npm|yarn|bun>
--approve-builds
--verbose
--interactive / --no-interactive
--list
-h / --help
```

Define the final `templateArgs` positional with `last = true`. This setting requires `--` before template arguments.

`clap` treats each later token as a value. This includes options that start with `-` and another literal `--`.

See the `clap` [argument escape and `last` rules](https://docs.rs/clap/latest/clap/_concepts/).

Preserve these inputs:

```text
vp create vite -- --template react-ts
vp create <template> -- <arbitrary hyphenated args>
vp create -- --template react-ts
```

Remove the manual JavaScript split only after equivalent tests pass. Reject a second positional argument before `--`.

The current code ignores that second positional argument. This RFC intentionally rejects it.

Accept only `pnpm`, `npm`, `yarn`, or `bun` for `--package-manager`.

Reuse `PackageManagerType::from_name()` from `vp_pm_cli`. Do not define a second Rust list.

Keep agent and editor name lookup in JavaScript. Reject the undocumented `--all` option.

The current `mri` configuration accepts `--all`, but no command code reads it.

## JavaScript-owned rules

Rust checks argument syntax and values. JavaScript keeps rules that depend on runtime state or JavaScript data.

| Rule                                    | Owner      |
| --------------------------------------- | ---------- |
| TTY default from `defaultInteractive()` | JavaScript |
| Create and migrate path resolution      | JavaScript |
| Agent names, aliases, and files         | JavaScript |
| Editor names, aliases, and files        | JavaScript |
| Git repository rules                    | JavaScript |
| Hooks-directory rules                   | JavaScript |
| Conversion to `lint-staged` options     | JavaScript |
| Option names and required values        | Rust       |
| Positional argument rules               | Rust       |
| Package-manager value                   | Rust       |
| Concurrency value                       | Rust       |

Return `undefined` for an omitted environment-dependent value. The Rust parser must not do these operations:

- call TTY APIs;
- read environment variables;
- inspect the file system;
- resolve paths.

## Compatibility decisions

This RFC preserves these behaviors:

- The local CLI runs in Node.js.
- The global CLI selects the local `dist/bin.js`.
- The local CLI handles `-C` and `vpr`.
- The selected local CLI prints help for its JavaScript commands.
- Documented aliases continue to work.
- Documented positive and negative options continue to work.
- `create` does not change arguments after `--`.
- JavaScript keeps command operations and runtime defaults.
- Invalid arguments exit with code 1.

This RFC intentionally changes these inputs:

| Input                         | Current `mri` result                         | New result              |
| ----------------------------- | -------------------------------------------- | ----------------------- |
| Unknown option                | Creates an unused property in many cases     | `clap` error            |
| Extra positional argument     | Leaves an unused value in `_`                | `clap` error            |
| Negative string option        | Can return `false`                           | `clap` error            |
| `--stash` or `--no-debug`     | Uses automatic `mri` negation                | `clap` error            |
| Repeated scalar option        | Can return an array or an order-based value  | `clap` conflict         |
| Fractional staged concurrency | Passes the PR #2501 check                    | `clap` value error      |
| Create `--all`                | Accepts and ignores the option               | `clap` error            |
| Invalid package manager       | JavaScript reports the error during `create` | `clap` reports it first |

`clap` changes the text of some diagnostics. The Vite+ header and exit code stay the same.

Review each diagnostic change with its command migration.

## Migration plan

Use small, focused pull requests when repository rules permit them.

### Phase 1: Shared parser and `staged`

1. Add the `vp_cli_help` crate.
2. Move the global help model and formatter to the new crate.
3. Add `binding/src/js_command_args/`.
4. Add the shared parser and error conversion.
5. Add the NAPI outcome and error types.
6. Add the shared `clap` metadata adapter.
7. Add `StagedArgs` and concurrency parsing.
8. Export `parseStagedArgs`.
9. Generate and inspect `binding/index.d.cts`.
10. Print `staged` help in Rust.
11. Remove the JavaScript parser and help rows for `staged`.
12. Keep the #2488 and #2501 regression tests.
13. Review the result before another migration.

### Phase 2: `config` and `hooks`

Migrate each command in a separate change. Remove obsolete JavaScript checks after the new tests pass.

### Phase 3: `migrate`

Move the argument grammar to `clap`. Keep runtime defaults, path resolution, and catalog lookup in JavaScript.

### Phase 4: `create`

Move the argument grammar and the `--` boundary to `clap`. Remove the manual split after pass-through tests succeed.

### Phase 5: Cleanup

Run:

```bash
rg "from 'mri'" packages/cli/src
```

Remove `mri` if no unrelated command uses it. Then update the lockfile.

Remove unused TypeScript parser types and conversion functions.

### Phase 6: Global clap adapter cleanup

The global CLI uses the shared help model and formatter in Phase 1. It still converts some rendered `clap` text into help documents.

Replace that text conversion with the shared `clap` metadata adapter after the five command migrations.

Do not make this cleanup a requirement for the local parser migration.

## Tests

### Rust parser tests

Test each grammar without NAPI. Test these cases:

- each documented long and short option;
- missing and invalid values;
- positive and negative forms in both orders;
- each repetition rule;
- unknown options;
- unsupported negative options;
- valid and invalid positional arguments;
- help as `DisplayHelp`;
- create arguments after `--`.

For `staged`, test these concurrency values:

- no value;
- `true`;
- `false`;
- valid integer limits;
- zero;
- negative integers;
- fractions;
- non-numbers;
- the upper limit.

### NAPI contract tests

Build the binding. Then check these results:

- Each parser returns the generated status and field names.
- Omitted Rust `Option` fields become optional JavaScript properties.
- Staged concurrency returns `boolean | number | undefined`.
- Help prints in Rust and returns `exit` with code 0.
- Errors do not return success objects.
- `index.d.cts` contains the exact runtime types.

### JavaScript command tests

Replace tests of `mri` output with tests at the NAPI boundary. Test the JavaScript conversion separately.

Keep tests for JavaScript defaults and command rules in their current modules.

### CLI snapshot tests

Use the PTY suite in `crates/vp_cli_snapshots/tests/cli_snapshots/`. Check public output and exit codes.

For each command migration, test these cases:

- valid execution;
- help;
- an unknown option;
- a missing value;
- an unsupported negative option.

Keep the `vp staged --no-concurrent` test. It must show that staged tasks run.

Add `create` tests that compare each template argument after `--`. Use local and global test modes when a fixture supports both.

## Risks

### Native builds

An option change requires a new Rust binding build. Vite+ already requires this binding for the local CLI.

The change adds build time, but it adds no JavaScript runtime dependency. Small command modules keep tests focused.

### Help metadata conversion and output

`clap` does not provide the Vite+ help document shape. The shared adapter converts public command metadata.

Keep the adapter small. Test optional values, aliases, positionals, headings, subcommands, hidden items, and display order.

Do not parse the complete rendered `clap` help text. Its headings, spaces, and wrapping are presentation details.

Native stdout is process-wide. Unit tests call the renderer with data and check its returned string. PTY tests check the final stdout output.

### NAPI value conversion

Some Rust types cannot cross NAPI without conversion. Keep each conversion small and explicit.

Generated declarations check field names and union shapes. Runtime tests check the JavaScript values.

### Strict parsing

Some external scripts can use options that `mri` ignored. Snapshot tests cannot find every external script.

Release notes must identify the new strict checks. A diagnostic must show the rejected token and command usage.

## Alternatives

### Keep `mri`

PR #2501 shows that local checks can fix one defect. This option gives the smallest first change.

Each command still needs a parser contract next to `clap`. TypeScript still receives unchecked values.

This option fixes symptoms but keeps two parsers.

### Use `util.parseArgs`

Supported Node.js versions include `util.parseArgs`. The [Node.js API](https://nodejs.org/api/util.html#utilparseargsconfig) supports strict checks and negative Boolean options.

It also supports strings, repeated values, token order, and unknown-option checks.

Vite+ still has two runtime parsers. Concurrency needs custom conversion. `create` also needs token handling for `--`.

This option is useful when a project has no native `clap` binding. Vite+ already has that binding.

### Add another JavaScript CLI framework

`cac`, Commander, and similar packages can add strict checks. They also add another dependency and command grammar.

Rust commands continue to use `clap`. Therefore, Vite+ still has two parsers.

### Parse local options in the global CLI

The global CLI can reject arguments before it starts Node.js. This design links option support to the global binary version.

That design breaks the version difference rule. The selected local package must define its supported options.

### Add commands to the top-level `CLIArgs`

The binding can parse all local arguments into one enum. JavaScript must select a command before it calls the current `run()` executor.

This design needs a new parse-and-dispatch protocol. It also creates one large NAPI union for unrelated commands.

Command-specific functions match the current dispatch point. They can still use one shared parser.

### Parse through `run()`

`run()` parses and runs Rust commands. A parse-only path mixes two command ownership models.

Command-specific parser functions keep parsing separate from execution.

### Move command operations to Rust

This option removes the parser language boundary. It also moves prompts, JavaScript library calls, and established file logic.

That work is larger than the parser change.

### Generate both languages from a neutral schema

A generator can define parsing and help for both languages. It must support subcommands, overrides, optional values, and custom parsers.

It must also support NAPI types and the JavaScript help layout. The generator becomes a third schema system.

`clap` and generated NAPI declarations provide the required contract with less code.

### Return rendered clap help

The NAPI function can return the complete text from `Command::render_help()`. This option removes the metadata adapter.

It also gives `clap` control of layout, wrapping, and styles. Vite+ then loses its shared Rust presentation.

The shared adapter keeps one argument source and one Vite+ renderer. Rust prints the result without a NAPI string conversion.

### Throw NAPI errors

A synchronous NAPI `Result<T>` can throw an error with a stable code. This design gives the success case a smaller type.

TypeScript declarations do not show thrown errors. JavaScript must also separate user errors from native failures.

The parse outcome keeps user errors in typed control flow. It reserves native exceptions for binding failures.

## Acceptance criteria

The complete migration must meet these criteria:

1. The five JavaScript commands parse arguments through `clap`.
2. JavaScript starts command work only after an `ok` outcome.
3. Generated NAPI declarations describe each outcome and value.
4. Each help request prints from the same `clap` command and returns exit code 0.
5. JavaScript command files do not duplicate option rows.
6. The five commands do not use `mri` directly.
7. Invalid arguments fail before command work.
8. Staged concurrency cannot be zero, negative, fractional, non-finite, or platform-dependent.
9. `create` forwards each token after the first `--` without changes.
10. The global CLI forwards local command arguments without parsing them.
11. JavaScript keeps runtime defaults, path rules, catalogs, prompts, and command operations.
12. Rust tests, NAPI tests, JavaScript tests, and CLI snapshots pass.

## References

- [Issue #2488: `vp staged --no-concurrent` does not start tasks](https://github.com/voidzero-dev/vite-plus/issues/2488)
- [PR #2501: Check staged concurrency options](https://github.com/voidzero-dev/vite-plus/pull/2501)
- [RFC: Add `vp config` and `vp staged`](./config-and-staged-commands.md)
- [RFC: Merge the Global and Local CLIs](./merge-global-and-local-cli.md)
- [CLI snapshot runner](../crates/vp_cli_snapshots/tests/cli_snapshots/README.md)
- [`clap::Args`](https://docs.rs/clap/latest/clap/trait.Args.html)
- [`clap` argument concepts](https://docs.rs/clap/latest/clap/_concepts/)
- [napi-rs objects](https://napi.rs/docs/concepts/object)
- [napi-rs attributes and structured enums](https://napi.rs/docs/concepts/napi-attributes)
