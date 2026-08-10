# lint_override_semantic

## `vp lint src`

extends rules are preserved when a file override adds Vue rules

**Exit code:** 1

```

  × eslint(no-console): Unexpected console statement.
   ╭─[src/example.js:1:1]
 1 │ console.log();
   · ───────────
   ╰────
  help: Delete this console statement.

  × vue(no-export-in-script-setup): <script setup>` cannot contain ES module exports.
   ╭─[src/example.vue:8:16]
 7 │
 8 │ export default {};
   ·                ──
 9 │ </script>
   ╰────

  × vue(no-export-in-script-setup): <script setup>` cannot contain ES module exports.
   ╭─[src/example.vue:8:8]
 7 │
 8 │ export default {};
   ·        ───────
 9 │ </script>
   ╰────

  × eslint(no-console): Unexpected console statement.
   ╭─[src/example.vue:6:1]
 5 │ <script lang="ts" setup>
 6 │ console.log();
   · ───────────
 7 │
   ╰────
  help: Delete this console statement.

Found 0 warnings and 4 errors.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```
