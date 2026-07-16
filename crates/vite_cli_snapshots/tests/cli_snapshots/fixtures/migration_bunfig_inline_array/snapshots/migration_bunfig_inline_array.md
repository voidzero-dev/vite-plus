# migration_bunfig_inline_array

## `vp migrate --no-interactive`

migration preserves inline arrays in an existing bunfig.toml

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  bun <version>
• 2 config updates applied
```

## `vpt print-file bunfig.toml`

check Bun configuration is unchanged

```
[install]
minimumReleaseAge = 259200
minimumReleaseAgeExcludes = ["@zerobyte/*", "vite-plus", "@voidzero-dev/vite-plus-core", "@voidzero-dev/vite-plus-darwin-arm64", "@voidzero-dev/vite-plus-darwin-x64", "@voidzero-dev/vite-plus-linux-arm64-gnu", "@voidzero-dev/vite-plus-linux-arm64-musl", "@voidzero-dev/vite-plus-linux-x64-gnu", "@voidzero-dev/vite-plus-linux-x64-musl", "@voidzero-dev/vite-plus-win32-arm64-msvc", "@voidzero-dev/vite-plus-win32-x64-msvc", "vitest", "@vitest/browser", "@vitest/browser-playwright", "@vitest/browser-preview", "@vitest/browser-webdriverio", "@vitest/coverage-istanbul", "@vitest/coverage-v8", "@vitest/expect", "@vitest/mocker", "@vitest/pretty-format", "@vitest/runner", "@vitest/snapshot", "@vitest/spy", "@vitest/ui", "@vitest/utils", "@vitest/web-worker", "@vitest/ws-client"]
```
