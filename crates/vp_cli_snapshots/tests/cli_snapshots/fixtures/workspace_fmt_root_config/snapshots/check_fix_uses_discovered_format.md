# check_fix_uses_discovered_format

## `vp check --fix fix.js`

```
pass: Formatting completed for checked files (<duration>)
pass: Found no warnings or lint errors in 1 file (<duration>, <n> threads)
```

## `vpt print-file fix.js`

Formatting after the root curly lint fix uses package quotes and semicolons.

```
export function greet(show) {
  if (show) {
    console.log("hello");
  }
}
```

## `vp check fix.js`

```
pass: All 1 file are correctly formatted (<duration>, <n> threads)
pass: Found no warnings or lint errors in 1 file (<duration>, <n> threads)
```
