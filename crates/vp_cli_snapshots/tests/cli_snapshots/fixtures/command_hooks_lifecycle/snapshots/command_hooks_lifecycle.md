# command_hooks_lifecycle

## `git init`


## `vp hooks status`

preference not set before enable

```
Preference:     not set
Hooks dir:      .vite-hooks
core.hooksPath: (unset)
Dispatcher:     missing (.vite-hooks/_)
Project hooks:  pre-commit
```

## `vp hooks enable`

install dispatcher

```
Git hook dispatcher installed at .vite-hooks/_
```

## `vp hooks status`

preference enabled after enable

```
Preference:     enabled
Hooks dir:      .vite-hooks
core.hooksPath: .vite-hooks/_ (Vite+ dispatcher)
Dispatcher:     installed (.vite-hooks/_)
Project hooks:  pre-commit
```

## `git config --local core.hooksPath`

should be .vite-hooks/_

```
.vite-hooks/_
```

## `vp hooks disable`

tear down and persist preference

```
Git hooks disabled: recorded disable preference (local git config); unset core.hooksPath (was ".vite-hooks/_"); removed .vite-hooks/_. Project-owned hooks under .vite-hooks/ and staged config were left unchanged. Run `vp hooks enable` to re-enable.
```

## `vp hooks status`

preference disabled (local)

```
Preference:     disabled (local)
Hooks dir:      .vite-hooks
core.hooksPath: (unset)
Dispatcher:     missing (.vite-hooks/_)
Project hooks:  pre-commit
```

## `vpt stat-file .vite-hooks/_/pre-commit --assert missing`

dispatcher removed

```
.vite-hooks/_/pre-commit: missing
```

## `vpt print-file .vite-hooks/pre-commit`

project-owned hook left unchanged

```
vp staged
```

## `npm_lifecycle_event=prepare vp config --no-agent`

prepare-like config should skip reinstall

```
skip install (hooks disabled; run `vp hooks enable` to re-enable)
```

## `vpt stat-file .vite-hooks/_/pre-commit --assert missing`

still missing after vp config

```
.vite-hooks/_/pre-commit: missing
```

## `vp hooks enable`

re-enable after disable

```
Git hook dispatcher installed at .vite-hooks/_
```

## `vp hooks status`

preference enabled again

```
Preference:     enabled
Hooks dir:      .vite-hooks
core.hooksPath: .vite-hooks/_ (Vite+ dispatcher)
Dispatcher:     installed (.vite-hooks/_)
Project hooks:  pre-commit
```

## `git config --local core.hooksPath`

dispatcher restored

```
.vite-hooks/_
```
