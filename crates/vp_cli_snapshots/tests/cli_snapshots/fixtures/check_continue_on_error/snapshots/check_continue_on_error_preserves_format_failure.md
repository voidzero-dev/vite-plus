# check_continue_on_error_preserves_format_failure

## `vpt write-file src/index.ts 'export const value = 42;
'`

```
```

## `vp check --continue-on-error src/clean-lint.ts`

successful lint and type checks must not hide a formatting failure

**Exit code:** 1

```
error: Formatting issues found
src/clean-lint.ts (<duration>)

Found formatting issues in 1 file (<duration>, <n> threads). Run `vp check --fix` to fix them.
pass: Found no warnings, lint errors, or type errors in 1 file (<duration>, <n> threads)
```

## `vp check --continue-on-error --no-lint --no-fmt src/clean-lint.ts`

type-check-only still succeeds

```
pass: Found no type errors in 1 file (<duration>, <n> threads)
```
