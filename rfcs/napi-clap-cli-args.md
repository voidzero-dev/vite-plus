# RFC: Parse JavaScript-backed CLI Arguments with clap

## Summary

Use the local `vite-plus` NAPI binding and Rust `clap` schemas to parse arguments for the five commands that JavaScript executes:

- `create`
- `migrate`
- `config`
- `hooks`
- `staged`

Node.js will remain the local CLI process. JavaScript will keep command dispatch, prompts, filesystem work, and calls to JavaScript libraries. Rust will own the runtime grammar, coercion, and argument validation for these commands.

The global Rust CLI will keep forwarding their arguments without parsing them. This preserves delegation from a global binary to a project-local `vite-plus` version with a different command surface.

## Motivation

The local entry point in `packages/cli/src/bin.ts` sends most commands through the NAPI-backed Rust CLI. It imports the five commands in this RFC as JavaScript modules. Each module uses `mri` and TypeScript assertions to interpret its own arguments.

That split creates two runtime argument systems:

- Rust-backed commands use `clap`.
- JavaScript-backed commands use permissive `mri` output plus local normalization.

[Issue #2488](https://github.com/voidzero-dev/vite-plus/issues/2488) showed a concrete failure. `mri` returns the boolean `false` for `--no-concurrent`, even when the caller declares `concurrent` as a string. The old `staged` code converted `false` to `0` and passed it to `lint-staged`. Its task queue did not start work with a concurrency limit of zero. [PR #2501](https://github.com/voidzero-dev/vite-plus/pull/2501) fixed the failure with JavaScript normalization and added regression tests.

The fix protects `staged`, but it leaves the parser split in place. Other examples remain:

- `vp config --no-hooks-dir` can put a boolean into a value that JavaScript treats as a path.
- Most JavaScript-backed commands accept unknown options and unused positional arguments because `mri` collects them without an error.
- `create` and `migrate` use assertions such as `as Options` and `as MigrationOptions`; those assertions do not validate runtime values.
- Each command implements negation, repeated options, and missing-value behavior on its own.

Vite+ already ships and loads a native binding for the local CLI. Reusing its `clap` dependency removes the second runtime parser without adding a new runtime component.

## Goals

1. Make `clap` the runtime parser and validator for the five JavaScript-backed command grammars.
2. Give JavaScript typed values that match the generated NAPI declarations.
3. Reject unknown options, extra positional arguments, missing values, and unsupported negations before command work starts.
4. Preserve documented command behavior and the local Node.js execution model.
5. Preserve the exact `--` pass-through boundary for template arguments in `vp create`.
6. Keep help output, environment-dependent defaults, and JavaScript business rules in their present ownership layer.
7. Migrate one command at a time, starting with `staged`.

## Non-goals

This RFC does not:

- move command execution, prompts, or filesystem work into Rust;
- replace the Node.js local CLI process;
- make the global Rust CLI validate project-local command options;
- route these commands through the binding's existing `run()` executor;
- replace the current `renderCliDoc()` output with clap help text;
- generate Rust schemas from TypeScript or TypeScript schemas from Rust source;
- change `-C`, `vpr`, top-level command routing, or package-manager routing;
- add a third CLI framework or a neutral schema language.

## Ownership boundaries

| Layer                                       | Responsibility after this RFC                                                                                                       |
| ------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| Global `vp` binary                          | Parse global options, select a command, find the matching local package, and forward JavaScript-command arguments as opaque strings |
| Local `packages/cli/src/bin.ts`             | Apply local `-C` and `vpr` rewrites, dispatch commands, and preserve current help routing                                           |
| `packages/cli/binding/src/js_command_args/` | Parse and validate the five JavaScript-command grammars with clap                                                                   |
| JavaScript command modules                  | Apply runtime defaults, run prompts and filesystem operations, and adapt parsed values to JavaScript APIs                           |

The global CLI must keep the Category B variants in `crates/vp_global_cli/src/cli.rs` as `Vec<String>` forwarding contracts. A global binary can invoke an older or newer project-local `vite-plus`. If the global binary parsed the local option schema, version skew could reject an option that the selected local package supports.

## Proposed architecture

```text
process.argv
    |
    v
local Node.js CLI
packages/cli/src/bin.ts
    |
    | argv for one JavaScript-backed command
    v
NAPI parser function
packages/cli/binding/src/js_command_args/
    |
    v
clap command schema
    |
    +-- aliases and option boundaries
    +-- value parsing and coercion
    +-- explicit negation
    +-- unknown-option checks
    +-- positional checks
    |
    v
semantic Rust arguments
    |
    v
typed NAPI parse outcome
    |
    v
JavaScript command business logic
```

JavaScript will pass raw command arguments into one NAPI parser. It will not parse the returned values again or serialize them back into argv.

## Rust module structure

Add a module beside the existing binding CLI executor:

```text
packages/cli/binding/src/js_command_args/
  mod.rs
  parse.rs
  create.rs
  migrate.rs
  config.rs
  hooks.rs
  staged.rs
```

The name `js_command_args` distinguishes these schemas from `binding/src/cli/`, which parses and executes Rust-backed local commands.

`mod.rs` will export the NAPI functions and shared transport types. `parse.rs` will contain the common clap helper and error conversion. Each command module will contain its clap type, semantic conversion, NAPI output type, and focused tests.

Create and migrate can flatten a private shared setup-options type when their grammar matches. They must not share fields that have different repetition or default rules.

## Shared clap helper

Each schema will derive `clap::Args`. A shared helper will add it to a synthetic command, prepend argv element zero, and use fallible clap APIs:

```rust
fn try_parse_args<T>(
    bin_name: &'static str,
    argv: Vec<String>,
) -> Result<T, clap::Error>
where
    T: clap::Args + clap::FromArgMatches,
{
    let command = T::augment_args(clap::Command::new(bin_name));
    let mut matches = command.try_get_matches_from(
        std::iter::once(bin_name.to_owned()).chain(argv),
    )?;
    T::from_arg_matches_mut(&mut matches)
}
```

The implementation can adjust ownership and iterator types. It must retain these properties:

- All wrappers use the same parse path.
- `try_get_matches_from` receives a synthetic argv element zero.
- The helper uses `from_arg_matches_mut` or the matching fallible conversion.
- The helper does not call `get_matches`, `parse`, `exit`, or a print method.
- Tests can invoke the helper without replacing `process.argv` or capturing a process exit.

The relevant clap APIs support this composition: [`Args::augment_args`](https://docs.rs/clap/latest/clap/trait.Args.html), [`FromArgMatches`](https://docs.rs/clap/latest/clap/trait.FromArgMatches.html), and the fallible parser methods on [`Parser`](https://docs.rs/clap/latest/clap/trait.Parser.html).

## NAPI contract

Export one synchronous function per command:

```ts
parseStagedArgs(argv: string[]): ParseStagedArgsOutcome
parseConfigArgs(argv: string[]): ParseConfigArgsOutcome
parseHooksArgs(argv: string[]): ParseHooksArgsOutcome
parseMigrateArgs(argv: string[]): ParseMigrateArgsOutcome
parseCreateArgs(argv: string[]): ParseCreateArgsOutcome
```

Each function will return a command-specific discriminated union:

```ts
type ParseStagedArgsOutcome =
  | { status: 'ok'; value: StagedArgs }
  | { status: 'help' }
  | { status: 'error'; error: CliParseError };

interface CliParseError {
  kind: string;
  message: string;
}
```

The binding can generate this shape from a napi-rs structured enum with `status` as its discriminant. napi-rs v3 supports structured enums and emits TypeScript unions for them through [`#[napi]` attributes](https://napi.rs/docs/concepts/napi-attributes). It also emits interfaces for [`#[napi(object)]`](https://napi.rs/docs/concepts/object) output types.

The union separates three cases:

- `ok` contains typed, validated arguments.
- `help` tells JavaScript to render the existing help document and exit zero.
- `error` contains a stable error kind and clap's rendered diagnostic.

The binding must map clap `DisplayHelp` to `help`. It must map other argument errors to `error`. The JavaScript command will print the Vite+ header, print the diagnostic, and exit with code 1. This retains the current exit code for invalid JavaScript-command arguments even though standalone clap applications often use code 2.

The parser functions should return data for user input errors instead of throwing a NAPI exception. A thrown native exception will then mean that the binding contract or native conversion failed, not that a user misspelled an option. napi-rs supports both patterns, but its error documentation notes that TypeScript declarations do not encode thrown errors. See [napi-rs error handling](https://napi.rs/docs/concepts/error-handling).

Do not return `serde_json::Value`. The generated declaration in `packages/cli/binding/index.d.cts` must list the real fields and union members.

## Rust types and NAPI output types

The clap type should express parser semantics. A separate, small NAPI output type may adapt Rust-only values to JavaScript:

```text
clap StagedArgs
    concurrent: Option<Concurrent>
              |
              v
StagedArgsJs
    concurrent?: boolean | number
```

That transport object does not define a second grammar. It converts a validated Rust value into the shape that JavaScript needs.

Use these rules for every output object:

- Use `Option<T>` for values the user did not specify. Do not insert TTY or environment defaults in Rust.
- Expose JavaScript field names in camel case, such as `diffFilter` and `allowEmpty`.
- Use generated TypeScript declarations instead of handwritten copies in command modules.
- Keep Rust enums and newtypes inside the parser when JavaScript only needs a boolean, number, or string.
- Use an integer type that converts to a JavaScript number without precision loss.

JavaScript may keep interfaces that describe broader command business inputs. It must not use assertions to pretend that raw parser output matches those interfaces.

## Help handling

Keep `renderCliDoc()` as the help renderer in this RFC.

clap will retain its built-in `-h` and `--help` action. The fallible parser reports that action as `DisplayHelp`; the NAPI wrapper converts it to the `help` outcome and discards clap's text. JavaScript then prints the existing Vite+ help document. This also preserves the current behavior where a help flag takes precedence over a later invalid option.

The clap schemas still need accurate names, value names, aliases, and short descriptions. A later RFC or follow-up can expose clap command metadata through NAPI and generate the `renderCliDoc()` rows. Until then, implementation PRs must update the clap schema and the JavaScript help document together.

The `staged` help label should change from `<number|boolean>` to `[number|boolean]` because Vite+ accepts a bare `--concurrent`. This corrects the documented value requirement; it does not change parsing behavior.

## Strict parsing and negation

The new schemas will reject:

- unknown options;
- positional arguments outside each command's declared positions;
- missing option values;
- repeated scalar options unless this RFC defines a repetition rule;
- `--no-*` spellings that the schema does not declare.

This strictness changes the permissive behavior of `mri`. The accepted command surface comes from documented options and compatibility cases, not from every property that `mri` happened to create.

Define positive and negative forms as separate clap arguments when the public CLI supports both. Use clap overrides so the last positive or negative spelling wins. [`ArgAction`](https://docs.rs/clap/latest/clap/enum.ArgAction.html) rejects repeated scalar values by default and supports explicit override behavior.

Apply these repetition rules:

| Option type                    | Rule                                                                 |
| ------------------------------ | -------------------------------------------------------------------- |
| Repeated `--agent <name>`      | Collect names in command-line order                                  |
| `--agent` and `--no-agent`     | The last form wins; a later `--agent` starts a new enabled selection |
| Repeated `--editor <name>`     | The last value wins, matching the existing create normalization      |
| Positive and negative booleans | The last form wins                                                   |
| Other repeated scalar options  | Reject as an argument conflict                                       |

Do not add generic negation. For example, `--no-cwd`, `--no-diff`, and `--no-hooks-dir` must fail in clap because each target expects a string.

## Command schemas

### `staged`

Migrate `staged` first. It exercises aliases, explicit negation, optional values, custom value parsing, strings, and booleans.

Represent concurrency with a semantic Rust type:

```rust
enum Concurrent {
    Enabled,
    Disabled,
    Limit(std::num::NonZeroU32),
}
```

Configure `--concurrent` with `num_args = 0..=1` and `default_missing_value = "true"`. Define `--no-concurrent` as a separate argument and make both forms override each other. clap supports optional values through `num_args` and `default_missing_value`; see [`Arg`](https://docs.rs/clap/latest/clap/builder/struct.Arg.html).

Accept:

```text
--concurrent
--no-concurrent
--concurrent true
--concurrent false
--concurrent=4
-p 4
```

Reject before JavaScript calls `lint-staged`:

```text
--concurrent=0
--concurrent=-1
--concurrent=1.5
--concurrent=NaN
--concurrent=4294967296
```

Use `NonZeroU32` instead of `NonZeroUsize`. It produces the same range on every supported platform and converts to a precise JavaScript number. A fractional task count has no clear scheduling meaning, so this RFC treats fractions as invalid even though the normalization from PR #2501 accepts any positive finite JavaScript number.

Return `concurrent?: boolean | number`. Return all other `lint-staged` options with camel-case names and `Option` semantics so JavaScript passes only explicit values to the programmatic API.

The schema must include the full staged surface:

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

Declare `--no-stash` as the supported stash negation. Do not infer forms such as `--no-debug` or `--stash` from `mri` behavior unless the public help and tests add those forms first.

After this migration, `packages/cli/src/staged/bin.ts` will consume the NAPI outcome and adapt the `ok` value to `lint-staged` options. Remove `packages/cli/src/staged/args.ts` when no caller needs it.

### `config`

Model these options:

```text
--hooks-dir <path>
--hooks / --no-hooks
--agent / --no-agent
-h / --help
```

The positive `--hooks` and `--agent` forms preserve existing accepted input, even though each positive form matches the default today. clap must reject `--no-hooks-dir` and missing path values.

Rust will return the raw hooks-directory string. JavaScript will keep Git lookup, path policy, lifecycle-event handling, prompts, and environment opt-outs.

### `hooks`

Model `enable`, `disable`, and `status` as clap subcommands. Put `--hooks-dir <path>` on each subcommand because the current grammar places the option after the subcommand.

`vp hooks` and help flags will render the existing top-level hooks help. Unknown subcommands, unknown options, and extra positional arguments will fail in clap before `enable` or `disable` can mutate repository state.

Remove `packages/cli/src/hooks/args.ts` after clap replaces `unexpectedHooksArgsError()`.

### `migrate`

Model:

```text
vp migrate [PATH] [OPTIONS]
```

The schema will accept one optional path and reject extra positionals. It will parse `interactive`, `agent`, `editor`, `hooks`, `full`, and help options with the repetition rules in this RFC.

The complete option surface is:

```text
--agent <name> / --no-agent
--editor <name> / --no-editor
--hooks / --no-hooks
--interactive / --no-interactive
--full
-h / --help
```

Return the path as written. JavaScript will resolve it against `process.cwd()`. Return `interactive?: boolean`; JavaScript will calculate `parsed.interactive ?? defaultInteractive()`. Agent and editor catalog lookup will stay in JavaScript because those catalogs and compatibility aliases live there.

### `create`

Migrate create last:

```text
vp create [TEMPLATE] [OPTIONS] [-- TEMPLATE_OPTIONS]
```

The Vite+ option surface before `--` is:

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

Use a final clap positional with `last = true` for `templateArgs`. clap then requires the first `--` separator before this positional and treats every later token as a value, including hyphenated options and another literal `--`. This matches clap's documented [argument escape and `last` behavior](https://docs.rs/clap/latest/clap/_concepts/).

The schema must preserve these cases:

```text
vp create vite -- --template react-ts
vp create <template> -- <arbitrary hyphenated args>
vp create -- --template react-ts
```

Remove the manual JavaScript split around `--` after the clap parser has equivalent tests. Reject a second positional before `--`; the current code ignores it.

Parse and validate `--package-manager` as one of `pnpm`, `npm`, `yarn`, or `bun`. The binding already depends on `vp_pm_cli`, so its value parser should reuse `PackageManagerType::from_name()` instead of copying that list into a new Rust enum. Keep agent and editor name resolution in JavaScript. Reject the undocumented and unused `--all` option that the present `mri` configuration lists but never reads.

## JavaScript-owned defaults and validation

Rust should validate syntax and values that belong to the CLI grammar. JavaScript should retain rules that depend on runtime state or JavaScript-owned catalogs:

| Rule                                                       | Owner            |
| ---------------------------------------------------------- | ---------------- |
| TTY-based `defaultInteractive()`                           | JavaScript       |
| Resolving create and migrate paths against `process.cwd()` | JavaScript       |
| Agent IDs, aliases, and target files                       | JavaScript       |
| Editor IDs, deprecated aliases, and config files           | JavaScript       |
| Git repository and hooks-directory policy                  | JavaScript       |
| Mapping staged config into `lint-staged`                   | JavaScript       |
| Option spelling, required values, and positionals          | Rust clap schema |
| Package-manager enum and concurrency value parsing         | Rust clap schema |

The parser should return `undefined` for an omitted environment-dependent value. It must not call TTY APIs, read environment variables, inspect the filesystem, or resolve paths.

## Compatibility decisions

This RFC preserves:

- local Node.js command execution;
- global delegation to the selected local `dist/bin.js`;
- current `-C` handling and `vpr` rewriting;
- `vp help <command>` and JavaScript-rendered command help;
- documented aliases and positive or negative option forms;
- create template arguments after `--` without modification;
- JavaScript ownership of command operations and defaults;
- exit code 1 for invalid arguments to these five commands.

This RFC changes these cases on purpose:

| Input class                                                      | `mri` behavior                               | Proposed behavior                                 |
| ---------------------------------------------------------------- | -------------------------------------------- | ------------------------------------------------- |
| Unknown option                                                   | Often creates an unused property             | clap error                                        |
| Extra positional                                                 | Often remains unused in `_`                  | clap error                                        |
| Negated string option                                            | Can produce boolean `false`                  | clap error                                        |
| Undocumented inferred boolean, such as `--stash` or `--no-debug` | Accepted through generic `mri` negation      | clap error                                        |
| Repeated scalar                                                  | Can produce arrays or order-dependent values | clap conflict unless this RFC defines an override |
| Fractional staged concurrency                                    | Passes the PR #2501 positive-number check    | clap value error                                  |
| Undocumented create `--all`                                      | Accepted and ignored                         | clap error                                        |
| Invalid package manager                                          | JavaScript detects it during create          | clap detects it before command work               |

Argument diagnostics will use clap wording, so invalid-input snapshots can change. The Vite+ header and exit code will remain stable. Review each output change as part of the command migration PR.

## Migration plan

Use a stack of focused PRs where repository and review constraints allow it.

### Phase 1: Parser infrastructure and `staged`

1. Add `binding/src/js_command_args/` and the shared helper.
2. Add the NAPI outcome and error types.
3. Add `StagedArgs`, semantic concurrency parsing, and `parseStagedArgs`.
4. Generate and inspect `binding/index.d.cts`.
5. Replace `mri` and staged normalization with the typed NAPI outcome.
6. Keep the #2488 and #2501 regression coverage.
7. Review error output, help behavior, and the NAPI shape before another command migrates.

### Phase 2: `config` and `hooks`

Migrate each command in its own reviewable change. Remove unsafe string assertions and the obsolete hooks validator after tests move to clap.

### Phase 3: `migrate`

Move argument grammar to clap. Keep path resolution, interactive defaults, agent and editor lookup, and migration work in JavaScript.

### Phase 4: `create`

Move the create grammar and `--` boundary to clap. Remove the manual separator split after pass-through tests cover all supported forms.

### Phase 5: Cleanup

Run:

```bash
rg "from 'mri'" packages/cli/src
```

If no unrelated caller remains, remove `mri` from `packages/cli/package.json` and update the lockfile. Remove TypeScript parser interfaces and normalizers that no code uses.

### Phase 6: Optional help unification

After the parser migrations settle, evaluate a separate change that exports clap metadata and builds `renderCliDoc()` rows from it. That change should preserve the Vite+ help layout and should not block this RFC.

## Testing strategy

### Rust parser tests

Test each schema without NAPI. Cover:

- every documented long and short spelling;
- missing and invalid values;
- supported positive and negative pairs in both orders;
- repeated options according to the rules in this RFC;
- unknown options and unsupported negations;
- missing, valid, and extra positionals;
- help as `DisplayHelp`;
- create pass-through after `--`.

For staged, test bare concurrency, booleans, valid integer limits, zero, negatives, fractions, non-numbers, and the upper bound.

### NAPI contract tests

Build the binding and test the JavaScript runtime shape:

- each parser returns the generated discriminant and field names;
- omitted Rust `Option` fields become optional JavaScript properties;
- staged concurrency becomes `boolean | number | undefined`;
- help and errors do not appear as successful argument objects;
- `index.d.cts` describes the runtime values without a handwritten override.

### JavaScript command tests

Replace tests of `mri` output and JavaScript normalization with tests at the parser boundary and command adapter. Keep tests for JavaScript-owned defaults and domain rules in their existing modules.

### CLI snapshots

Use the PTY suite under `crates/vp_cli_snapshots/tests/cli_snapshots/` for public output and exit codes. Each command migration should cover valid execution, help, unknown options, missing values, and one unsupported negation.

Keep the `vp staged --no-concurrent` snapshot that proves tasks run. Add create snapshots for exact template pass-through. Run both local and global flavors when the fixture supports them so the suite proves that global forwarding stays opaque.

## Risks and mitigations

### Native rebuilds for option changes

An option grammar change will require a Rust binding build. Vite+ already needs that binding for the local CLI, so this adds build cost but no new runtime dependency. Small command-specific modules and focused parser tests will keep iteration bounded.

### Help metadata remains duplicated

JavaScript help rows and clap schemas can drift during the first phases. Each migration must add contract tests for every help spelling and update both sources in one change. The optional help phase can remove this duplication later.

### Cross-language transport adds conversion code

Rust-only semantic types cannot cross NAPI in their ideal form. Command modules should keep the conversion explicit and small. Generated declarations and runtime-shape tests will catch field or union drift.

### Strict parsing can expose accidental dependencies

Some scripts may pass options that `mri` ignored. Snapshot coverage cannot discover every external script. Release notes should call out strict validation, and clap diagnostics should name the rejected token and usage.

## Alternatives

### Keep `mri` and expand normalization

This has the smallest first diff, as PR #2501 showed. Each command would still maintain a parser contract beside clap, and TypeScript would still receive unvalidated union values. More normalization treats individual symptoms without removing the split.

### Use Node.js `util.parseArgs`

The supported Node versions include `util.parseArgs` with strict parsing and negative boolean options. The [Node.js API](https://nodejs.org/api/util.html#utilparseargsconfig) supports string and boolean option types, repeated values, token order, and strict unknown-option checks.

It would improve on `mri`, but Vite+ would still have two runtime parser systems. Concurrency needs custom coercion, and create still needs token-level handling for `--`. This option suits a project without a native clap binding; Vite+ already has one.

### Adopt another JavaScript CLI framework

`cac`, Commander, or a similar package could enforce stricter JavaScript parsing. It would add another command schema and dependency while Rust-backed commands continue to use clap.

### Parse these commands in the global Rust CLI

This would let the global binary reject input before starting Node.js. It would also bind global option support to the global binary version. That breaks the current version-skew contract where a global binary finds and runs a project-local package. Opaque forwarding belongs at the global boundary.

### Add the five commands to the binding's top-level `CLIArgs`

The binding could parse the full local argv into one enum before JavaScript dispatch. `bin.ts` needs to choose and import a JavaScript command before it calls the existing `run()` executor, so this design would require a new parse-then-dispatch protocol for every command or a split of `run()` into parse and execute stages. It would also create one large NAPI union for unrelated commands. Command-specific functions fit the current dispatch boundary and still share the parser helper.

### Route parsing through `run()`

The existing binding `run()` function parses and executes Rust-backed commands. Sending JavaScript commands through it for parsing would mix two execution ownership models and require a parse-only escape path from an executor. Command-specific parser functions keep the boundary explicit.

### Move command execution to Rust

This would remove the language boundary for argument handling, but it would also move prompt flows, JavaScript library calls, and mature filesystem logic. The cost and review surface exceed the parser problem.

### Define a neutral schema and generate Rust and TypeScript

A schema generator could drive parsing and help in both languages. It would need to express clap subcommands, overrides, optional values, semantic parsers, NAPI types, and JavaScript help layout. The generator would become a third abstraction for five commands. clap plus generated NAPI declarations covers the runtime contract with less machinery.

### Throw NAPI errors for invalid argv

A synchronous NAPI `Result<T>` can throw an error with a stable code. That produces a smaller success signature. It also makes JavaScript distinguish an expected CLI parse failure from a binding failure through exception metadata that TypeScript cannot express. The discriminated outcome keeps normal CLI control flow typed and reserves exceptions for native faults.

## Acceptance criteria

The work is complete when:

1. The five JavaScript-backed commands parse argv through command-specific NAPI functions backed by clap.
2. JavaScript command work starts only after an `ok` outcome.
3. Generated NAPI declarations describe every success object and parse outcome.
4. Direct `mri` parsing and parser-only TypeScript assertions disappear from these commands.
5. Invalid string negations, unknown options, missing values, and extra positionals fail before command work.
6. Staged concurrency cannot become zero, negative, fractional, non-finite, or platform-dependent.
7. Create forwards every token after the first `--` without interpretation.
8. The global CLI continues to forward these command arguments as opaque strings.
9. JavaScript retains TTY defaults, path resolution, agent and editor catalogs, prompts, and business operations.
10. Rust parser tests, NAPI shape tests, JavaScript command tests, generated declaration review, and relevant CLI snapshots pass.

## References

- [Issue #2488: `vp staged --no-concurrent` silently stalls](https://github.com/voidzero-dev/vite-plus/issues/2488)
- [PR #2501: normalize staged concurrency options](https://github.com/voidzero-dev/vite-plus/pull/2501)
- [RFC: Built-in Pre-commit Hook via `vp config` and `vp staged`](./config-and-staged-commands.md)
- [RFC: Merge Global and Local CLI into a Single Package](./merge-global-and-local-cli.md)
- [CLI snapshot runner](../crates/vp_cli_snapshots/tests/cli_snapshots/README.md)
- [clap `Args`](https://docs.rs/clap/latest/clap/trait.Args.html)
- [clap argument concepts](https://docs.rs/clap/latest/clap/_concepts/)
- [napi-rs objects](https://napi.rs/docs/concepts/object)
- [napi-rs attributes and structured enums](https://napi.rs/docs/concepts/napi-attributes)
