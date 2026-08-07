# command_staged_broken_config

## `vpt write-file vite.config.ts 'export default {
  staged: {
    "*.ts": "vp check --fix",
  },
  // syntax error: missing closing brace
'`


## `vp staged`

should show actual config error, not 'No staged config found'

**Exit code:** 1

```
failed to load config from <workspace>/vite.config.ts
Failed to load vite.config: Build failed with 1 error:

[PARSE_ERROR] Unexpected token
   ╭─[ vite.config.ts:5:42 ]
   │
 5 │   // syntax error: missing closing brace
   │                                          │
   │                                          ╰─
───╯
```
