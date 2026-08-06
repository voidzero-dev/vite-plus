# command_exec_monorepo_filter_v2

## `vp exec -F app-a -- echo hello`

-F short flag for --filter

```
hello
```

## `vp exec -F app-a -F lib-c -- echo hello`

-F short flag multiple

```
lib-c$ echo hello
hello
app-a$ echo hello
hello
```

## `vp exec -F app-* -- echo hello`

-F short flag with glob

```
app-a$ echo hello
hello
app-b$ echo hello
hello
```

## `vp exec --filter 'app-a lib-c' -- node -e console.log(process.env.VP_PACKAGE_NAME)`

whitespace splitting: 'a b' => two filters

```
lib-c$ node -e console.log(process.env.VP_PACKAGE_NAME)
lib-c
app-a$ node -e console.log(process.env.VP_PACKAGE_NAME)
app-a
```

## `vp exec --filter 'app-a  app-b' -- node -e console.log(process.env.VP_PACKAGE_NAME)`

whitespace splitting with extra spaces

```
app-a$ node -e console.log(process.env.VP_PACKAGE_NAME)
app-a
app-b$ node -e console.log(process.env.VP_PACKAGE_NAME)
app-b
```

## `vp exec --filter nonexistent-pkg -- echo hello`

unmatched filter warning

```
warn: No packages matched the filter 'nonexistent-pkg'
warn: No packages matched the filter(s)
```

## `vp exec --fail-if-no-match --filter nonexistent-pkg -- echo hello`

unmatched filter strict error

**Exit code:** 1

```
error: No packages matched the filter: nonexistent-pkg
```

## `vp exec --fail-if-no-match --filter nonexistent-pkg --filter app-a -- echo hello`

unmatched + matched strict error

**Exit code:** 1

```
error: No packages matched the filter: nonexistent-pkg
```

## `vp exec --fail-if-no-match --filter nope1 --filter nope2 -- echo hello`

multiple unmatched strict error

**Exit code:** 1

```
error: No packages matched the filter: nope1, nope2
```

## `vp exec --fail-if-no-match --filter app-a -- echo hello`

strict succeeds when filter matches

```
hello
```

## `vp exec --filter nonexistent-pkg --filter app-a -- node -e console.log(process.env.VP_PACKAGE_NAME)`

unmatched + matched filter

```
warn: No packages matched the filter 'nonexistent-pkg'
app-a
```

## `vp exec -w --filter app-a -- node -e console.log(process.env.VP_PACKAGE_NAME)`

-w + --filter is additive (root + filtered)

```
app-a$ node -e console.log(process.env.VP_PACKAGE_NAME)
app-a
exec-monorepo-filter-v2$ node -e console.log(process.env.VP_PACKAGE_NAME)
exec-monorepo-filter-v2
```

## `vp exec -w --filter app-* -- node -e console.log(process.env.VP_PACKAGE_NAME)`

-w + glob filter additive

```
app-a$ node -e console.log(process.env.VP_PACKAGE_NAME)
app-a
exec-monorepo-filter-v2$ node -e console.log(process.env.VP_PACKAGE_NAME)
exec-monorepo-filter-v2
app-b$ node -e console.log(process.env.VP_PACKAGE_NAME)
app-b
```

## `vp exec -r --filter app-a -- echo hello`

-r + --filter conflict error

**Exit code:** 1

```
error: --filter and --recursive cannot be used together
```
