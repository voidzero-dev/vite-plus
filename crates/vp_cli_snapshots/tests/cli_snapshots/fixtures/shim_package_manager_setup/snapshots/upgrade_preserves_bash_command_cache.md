# upgrade_preserves_bash_command_cache

## `node verify.mjs cache`

The upgrade handoff runs with piped output, between direct calls in the same Bash process.

```
system pnpm and pnpx: cached paths survive setup and repeated direct calls.
managed pnpm and pnpx: cached paths survive setup and repeated direct calls.
```
