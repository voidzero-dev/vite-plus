# Type-aware linting with tsrs-lint

On the Vite+ workspace at `a32a8cec2` with the benchmark script added, the experimental backend saved 0.51 seconds per lint run with full type checking. Both backends analyzed 385 files with 174 rules, reported zero diagnostics, and exited successfully on every run.

| `vp lint` workload                          | Bundled tsgolint | tsrs-lint | Wall time saved |
| ------------------------------------------- | ---------------: | --------: | --------------: |
| Type-aware rules                            |          1.089 s |   0.822 s |           24.5% |
| Type-aware rules and TypeScript diagnostics |          1.340 s |   0.830 s |           38.1% |

For the full `vp check` command, the median fell from 1.852 s to 1.335 s (27.9%). This command also checked formatting on 632 files and used its default 16 threads.

Measured on an Apple M4 Max with macOS 27.0.1, Node.js 24.22.2, Vite+ 1.5.0, Oxlint 1.91.0, oxlint-tsgolint 7.0.2005, and tsrs 0.21.0-ts7.1.0-beta. The benchmark used the published Vite+ 1.5.0 CLI artifacts and the workspace's locked dependencies. Each sample started a fresh local CLI process and included config loading, JS plugins, the backend launcher, and diagnostic reporting. The `vp lint` runs used four Oxlint threads. Both backends used their default settings.

The table shows the median of ten runs per backend after one warm-up each. The runner reverses the backend order between pairs and compares the exit code, file and rule counts, and sorted JSON diagnostics for every run. In a separate failing fixture, both backends reported `typescript/no-floating-promises` and `TS2322` at the same locations through `vp lint` and `vp check`.

## Reproduce

Start in a built Vite+ checkout. Install the experimental backend under the ignored `tmp` directory to keep it separate from workspace dependencies.

```sh
npm install --prefix ./tmp/tsrs-lint --no-save --package-lock=false tsrs@0.21.0-ts7.1.0-beta
export OXLINT_TSGOLINT_PATH="$PWD/tmp/tsrs-lint/node_modules/.bin/tsrs-lint"
node bench/tsrs-lint.ts --type-check
```

For the rules-only row, temporarily set `lint.options.typeCheck` to `false` in the root `vite.config.ts`, run `node bench/tsrs-lint.ts` without `--type-check`, then restore the setting. The benchmark always enables `--type-aware` and prints the individual samples in seconds.
