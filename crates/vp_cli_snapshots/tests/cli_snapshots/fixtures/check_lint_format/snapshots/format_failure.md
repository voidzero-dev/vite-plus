# format_failure

## `vp check --format=agent unformatted.js error.js`

A format failure stops before lint diagnostics.

**Exit code:** 1

```
error: Formatting issues found
unformatted.js (<duration>)

Found formatting issues in 1 file (<duration>, <n> threads). Run `vp check --fix` to fix them.
```

## `vp check --fix --format=typo unformatted.js`

Invalid reporter names are rejected before formatting writes.

**Exit code:** 2

```
error: invalid value 'typo' for '--format <FORMAT>'
  [possible values: default, agent, github, gitlab, json, junit, sarif, stylish, unix, checkstyle]

For more information, try '--help'.
```

## `vpt print-file unformatted.js`

```
export const value=1
```
