# coverage_providers_and_v4_file_set

Install the packed CLI and project-owned providers together; checkout symlinks cannot test native provider resolution.

## `npm install --ignore-scripts --no-audit --no-fund`


## `npm install --prefix legacy --ignore-scripts --no-audit --no-fund`


## `vpt cp -r src legacy`


## `node verify.mjs`

```
v8: matching provider, shared runner, covered/untested files, and exclusion passed
v8: aggregate glob threshold passes; perFile rejects the untested file
istanbul: matching provider, shared runner, covered/untested files, and exclusion passed
istanbul: aggregate glob threshold passes; perFile rejects the untested file
Vitest v4/v5 coverage file sets match: src/covered.js, src/untested.js
Istanbul provider uses the final @vitest/istanbul-lib-* graph
Mismatched provider rejected before tests run
```
