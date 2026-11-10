# check_continue_on_error_fix_reformats_after_lint_failure

## `vpt write-file src/index.ts 'export const value: string = 42;
if (value) eval("value");
'`

```
```

## `vp check --fix src/index.ts`

default fail-fast skips the final format pass after curly fixes and remaining lint/type failures

**Exit code:** 1

```
error: Lint or type issues found
× eslint(no-eval): eval can be harmful.
   ╭─[src/index.ts:2:12]
 1 │ export const value: string = 42;
 2 │ if (value) eval("value");
   ·            ────
   ╰────
  help: Avoid eval(). For JSON parsing use JSON.parse(); for dynamic property access use bracket notation (obj[key]); for other cases refactor to avoid evaluating strings as code.

  × typescript(TS2322): Type 'number' is not assignable to type 'string'.
   ╭─[src/index.ts:1:14]
 1 │ export const value: string = 42;
   ·              ─────
 2 │ if (value) {eval("value");}
   ╰────

Found 2 errors and 0 warnings in 1 file (<duration>, <n> threads)
pass: Formatting completed for checked files (<duration>)
```

## `vpt print-file src/index.ts`

```
export const value: string = 42;
if (value) {eval("value");}
```

## `vp fmt --check src/index.ts`

lint fixes left the source unformatted

**Exit code:** 1

```
Checking formatting...

src/index.ts (<duration>)

Format issues found in above 1 files. Run without `--check` to fix.
Finished in <duration> on 1 files using <n> threads.
No config found, using defaults. Please add a config file or try `vp fmt --init` if needed.
```

## `vpt write-file src/index.ts 'export const value: string = 42;
if (value) eval("value");
'`

```
```

## `vp check --continue-on-error --fix src/index.ts`

curly is fixed and the final format pass runs, but remaining lint/type errors keep the exit nonzero

**Exit code:** 1

```
error: Lint or type issues found
× eslint(no-eval): eval can be harmful.
   ╭─[src/index.ts:2:12]
 1 │ export const value: string = 42;
 2 │ if (value) eval("value");
   ·            ────
   ╰────
  help: Avoid eval(). For JSON parsing use JSON.parse(); for dynamic property access use bracket notation (obj[key]); for other cases refactor to avoid evaluating strings as code.

  × typescript(TS2322): Type 'number' is not assignable to type 'string'.
   ╭─[src/index.ts:1:14]
 1 │ export const value: string = 42;
   ·              ─────
 2 │ if (value) {eval("value");}
   ╰────

Found 2 errors and 0 warnings in 1 file (<duration>, <n> threads)
pass: Formatting completed for checked files (<duration>)
```

## `vpt print-file src/index.ts`

```
export const value: string = 42;
if (value) {
  eval("value");
}
```

## `vp fmt --check src/index.ts`

the final formatting pass succeeded even though the overall check failed

```
Checking formatting...

All matched files use the correct format.
Finished in <duration> on 1 files using <n> threads.
No config found, using defaults. Please add a config file or try `vp fmt --init` if needed.
```
