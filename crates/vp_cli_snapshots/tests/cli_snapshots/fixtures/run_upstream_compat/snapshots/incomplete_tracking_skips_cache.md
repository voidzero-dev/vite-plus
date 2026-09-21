# incomplete_tracking_skips_cache

Limit tracking to 64 KiB of records after the descriptor table. The task must finish successfully and skip caching on both runs.

## `vp run -v tracking`

```
VITE+ - The Unified Toolchain for the Web

$ node many-files.mjs
Task completed after 20000 file accesses

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    Vite+ Task Runner • Execution Summary
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Statistics:   1 tasks • 0 cache hits • 1 cache misses
Performance:  0% cache hit rate

Task Details:
────────────────────────────────────────────────
  [1] run-upstream-compat#tracking: $ node many-files.mjs ✓
      → Not cached: this task used more files than automatic tracking can record. Configure `input` and `output` manually to enable caching.
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

## `vp run -v tracking`

```
VITE+ - The Unified Toolchain for the Web

$ node many-files.mjs
Task completed after 20000 file accesses

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    Vite+ Task Runner • Execution Summary
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Statistics:   1 tasks • 0 cache hits • 1 cache misses
Performance:  0% cache hit rate

Task Details:
────────────────────────────────────────────────
  [1] run-upstream-compat#tracking: $ node many-files.mjs ✓
      → Not cached: this task used more files than automatic tracking can record. Configure `input` and `output` manually to enable caching.
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```
