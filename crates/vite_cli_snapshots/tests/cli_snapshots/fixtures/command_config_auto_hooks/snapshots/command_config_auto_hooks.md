# command_config_auto_hooks

## `git init`


## `vp config`

prepare should install the dispatcher without changing project hook policy

```
```

## `git config --local core.hooksPath`

should be .vite-hooks/_

```
.vite-hooks/_
```

## `vpt print-file .vite-hooks/pre-commit`

project-owned hook should remain unchanged

```
vp run lint
vp exec tsc --noEmit
```

## `vpt print-file vite.config.ts`

should remain unchanged

```
import { defineConfig } from 'vite-plus';

export default defineConfig({});
```
