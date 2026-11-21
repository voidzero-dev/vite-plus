# fix_failure

## `vp check --fix --format=agent unformatted.js error.js`

A lint failure still reports formatting completion.

**Exit code:** 1

```
error.js:1:1: error eslint(no-eval): eval can be harmful. help: Avoid eval(). For JSON parsing use JSON.parse(); for dynamic property access use bracket notation (obj[key]); for other cases refactor to avoid evaluating strings as code.
pass: Formatting completed for checked files (<duration>)
```

## `vpt print-file unformatted.js`

The first format pass changed the file before lint failed.

```
export const value = 1;
```
