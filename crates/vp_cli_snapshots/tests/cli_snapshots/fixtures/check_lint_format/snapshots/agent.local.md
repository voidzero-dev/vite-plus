# agent

## `vp check --format=agent clean.js`

The agent reporter is silent on success; fmt still runs.

```
pass: All 1 file are correctly formatted (<duration>, <n> threads)
```

## `vp check --no-fmt --format agent clean.js`

A completely silent lint phase still succeeds.

```
```

## `vp check --format=agent warning.js`

Warnings do not fail by default.

```
pass: All 1 file are correctly formatted (<duration>, <n> threads)
warning.js:1:1: warning eslint(no-console): Unexpected console statement. help: Delete this console statement.
```

## `vp check --format=agent --quiet warning.js`

```
pass: All 1 file are correctly formatted (<duration>, <n> threads)
```

## `vp check --format=agent --deny-warnings warning.js`

**Exit code:** 1

```
pass: All 1 file are correctly formatted (<duration>, <n> threads)
warning.js:1:1: warning eslint(no-console): Unexpected console statement. help: Delete this console statement.
```

## `vp check --no-fmt --format=agent --quiet --deny-warnings warning.js`

Silent output does not turn a failure into success.

**Exit code:** 1

```
```

## `vp check --format=agent error.js`

**Exit code:** 1

```
pass: All 1 file are correctly formatted (<duration>, <n> threads)
error.js:1:1: error eslint(no-eval): eval can be harmful. help: Avoid eval(). For JSON parsing use JSON.parse(); for dynamic property access use bracket notation (obj[key]); for other cases refactor to avoid evaluating strings as code.
```
