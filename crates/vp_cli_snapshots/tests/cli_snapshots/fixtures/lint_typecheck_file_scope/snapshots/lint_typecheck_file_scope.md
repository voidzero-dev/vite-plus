# lint_typecheck_file_scope

## `vp lint src/index.ts fixtures/bad.ts`

ignored imported files provide types without reporting their own errors

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp check --no-fmt src/index.ts fixtures/bad.ts`

```
pass: Found no warnings, lint errors, or type errors in 1 file (<duration>, <n> threads)
```

## `vpt replace-file-content vite.config.ts 'fixtures/**' `

```
```

## `vp lint src/index.ts`

command-line paths also limit diagnostics when no files are ignored

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint fixtures/bad.ts`

the same error is reported when its file is selected

**Exit code:** 1

```

  × typescript(TS2322): Type 'number' is not assignable to type 'string'.
   ╭─[fixtures/bad.ts:1:14]
 1 │ export const bad: string = 123;
   ·              ───
   ╰────

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
