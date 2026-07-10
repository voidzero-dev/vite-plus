# fmt_no_config_message

## `vp fmt`

should show 'vp fmt --init' instead of 'oxfmt --init'

```
Finished in <duration> on 3 files using <n> threads.
No config found, using defaults. Please add a config file or try `vp fmt --init` if needed.
```

## `vp fmt --init`

```
Added 'fmt' to 'vite.config.ts'.
```

## `vpt print-file vite.config.ts`

should have fmt config

```
export default {
  fmt: {
    ignorePatterns: [],
  },
};
```

## `vp fmt`

should no longer show 'No config found' message

```
Finished in <duration> on 3 files using <n> threads.
```
