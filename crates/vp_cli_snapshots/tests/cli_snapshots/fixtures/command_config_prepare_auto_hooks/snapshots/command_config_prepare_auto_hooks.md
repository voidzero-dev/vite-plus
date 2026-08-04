# command_config_prepare_auto_hooks

## `git init`


## `vp config`

should install the dispatcher automatically without prompting

```
```

## `git config --local core.hooksPath`

should be .vite-hooks/_

```
.vite-hooks/_
```

## `vpt stat-file .vite-hooks/_/pre-commit --assert file`

generated dispatcher shim should exist

```
.vite-hooks/_/pre-commit: file
```

## `vpt stat-file .vite-hooks/pre-commit --assert missing`

project hook should not be created

```
.vite-hooks/pre-commit: missing
```

## `vpt stat-file vite.config.ts --assert missing`

vite config should not be created

```
vite.config.ts: missing
```

## `vp config`

run again to ensure idempotent

```
```

## `vpt stat-file .vite-hooks/pre-commit --assert missing`

project hook should still be absent

```
.vite-hooks/pre-commit: missing
```

## `vpt stat-file vite.config.ts --assert missing`

vite config should still be absent

```
vite.config.ts: missing
```
