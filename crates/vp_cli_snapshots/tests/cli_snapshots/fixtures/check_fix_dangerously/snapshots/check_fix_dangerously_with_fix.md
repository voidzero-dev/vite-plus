# check_fix_dangerously_with_fix

## `vp check --fix --fix-dangerously`

```
pass: Formatting completed for checked files (<duration>)
pass: Found no warnings or lint errors in 2 files (<duration>, <n> threads)
```

## `vpt print-file src/index.js`

```
export function greet(enabled) {
  if (enabled) {
    return "hello";
  }
  return "world";
}
```

## `vp check`

```
pass: All 3 files are correctly formatted (<duration>, <n> threads)
pass: Found no warnings or lint errors in 2 files (<duration>, <n> threads)
```
