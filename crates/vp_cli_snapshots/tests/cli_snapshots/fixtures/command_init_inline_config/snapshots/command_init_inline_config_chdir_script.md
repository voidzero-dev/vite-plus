# command_init_inline_config_chdir_script

An init command after vp -C in a task script merges its config into vite.config.ts like a direct call.

## `vp run fmt:init`

```
$ vp -C . fmt --init ⊘ cache disabled
Added 'fmt' to 'vite.config.ts'.
```

## `vpt print-file vite.config.ts`

```
import { defineConfig } from "vite-plus";

export default defineConfig({
  fmt: {
    ignorePatterns: [],
  },
});
```

## `vpt stat-file .oxfmtrc.json --assert-not file`

check .oxfmtrc.json is removed

```
.oxfmtrc.json: missing
```
