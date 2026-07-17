# command_config_prepare_auto_hooks

## `git init`


## `vp config`

should install hooks automatically without prompting

```
```

## `git config --local core.hooksPath`

should be .vite-hooks/_

```
.vite-hooks/_
```

## `vpt print-file .vite-hooks/pre-commit`

should have vp staged

```
vp staged
```

## `vpt print-file vite.config.ts`

should have staged config

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    "*": "vp check --fix"
  },

});
```

## `vp config`

run again to ensure idempotent

```
```

## `vpt print-file .vite-hooks/pre-commit`

should remain unchanged

```
vp staged
```

## `vpt print-file vite.config.ts`

should remain unchanged

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    "*": "vp check --fix"
  },

});
```
