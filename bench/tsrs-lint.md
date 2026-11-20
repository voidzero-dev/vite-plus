# Type-aware linting with tsrs-lint

The timing and memory results below were measured locally on **2026-11-16** with
the latest published [PR #3352 preview](https://github.com/voidzero-dev/vite-plus/actions/runs/43875765725),
`0.0.0-commit.3345a7bf89b1f8c5e3a3afa6d852446151256835`.
Both controls use the same published CLI and Oxlint `1.93.0`: the Go control uses
`oxlint-tsgolint@7.0.2007`, and the default preview uses `tsrs@7.1.0-beta.tsrs2`.

All 12 validation commands and 204 benchmark invocations pass. The backends have
matching checked-file counts and no diagnostics on these two inputs. The separate
[full ecosystem comparison](./tsrs-lint-ecosystem.md) documents compatibility
differences on other projects; these performance results do not establish compiler parity.

## Elapsed time

Each cell shows the median of **10 runs**, followed by the interquartile range
(25th–75th percentile) in brackets. One warm-up per backend is excluded.
Each run starts a fresh CLI process. Backend order alternates between pairs.
Timing runs do not sample memory, and all benchmark commands run sequentially.

| Project | Workload                         |              Go (s) |            tsrs (s) | Reduction |
| ------- | -------------------------------- | ------------------: | ------------------: | --------: |
| Vite+   | Type-aware rules                 | 1.510 [1.440–1.577] | 1.233 [1.192–1.290] |     18.3% |
| Vite+   | Rules and TypeScript diagnostics | 1.885 [1.831–1.938] | 1.378 [1.345–1.414] |     26.9% |
| Vite+   | Full `vp check`                  | 2.087 [2.061–2.121] | 1.695 [1.611–1.731] |     18.8% |
| vinext  | Type-aware rules                 | 2.727 [2.703–2.768] | 1.664 [1.638–1.718] |     39.0% |
| vinext  | Rules and TypeScript diagnostics | 3.412 [3.322–3.499] | 1.728 [1.672–1.767] |     49.4% |
| vinext  | Full `vp check`                  | 4.728 [4.694–4.771] | 2.201 [2.173–2.256] |     53.4% |

## Peak memory

Each cell shows the median of **5 separate runs** and its interquartile range.
One warm-up per backend is excluded. Memory is the sampled peak of the simultaneous
sum of RSS for the CLI and its descendants, including Node, Oxlint, and the native
backend. The sampler itself is excluded. One GiB is 1,073,741,824 bytes.

| Project | Workload                         |            Go (GiB) |          tsrs (GiB) | Reduction |
| ------- | -------------------------------- | ------------------: | ------------------: | --------: |
| Vite+   | Type-aware rules                 | 1.209 [1.193–1.214] | 0.576 [0.566–0.593] |     52.4% |
| Vite+   | Rules and TypeScript diagnostics | 1.277 [1.260–1.279] | 0.648 [0.648–0.650] |     49.2% |
| Vite+   | Full `vp check`                  | 1.457 [1.334–1.462] | 0.702 [0.693–0.708] |     51.8% |
| vinext  | Type-aware rules                 | 3.611 [3.547–3.652] | 1.732 [1.730–1.736] |     52.0% |
| vinext  | Rules and TypeScript diagnostics | 3.718 [3.637–4.261] | 1.815 [1.814–1.844] |     51.2% |
| vinext  | Full `vp check`                  | 5.085 [4.606–5.434] | 1.972 [1.955–1.993] |     61.2% |

The sampler sleeps for 20 ms; the median observed interval is 37.8 ms
(the median of the measured runs' sampling intervals). Shared pages can count more
than once, and sampling can miss short peaks. These are process-tree RSS values,
not total allocations or macOS physical-footprint measurements.

## Inputs and method

| Input                                    | Pin / condition                                                                                                           |
| ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Machine                                  | Apple M4 Max, 16 CPU cores, 48 GiB RAM, macOS `27.0.1`, ARM64                                                             |
| Vite+ source                             | [`3345a7bf8`](https://github.com/voidzero-dev/vite-plus/tree/3345a7bf89b1f8c5e3a3afa6d852446151256835), Node.js `24.22.3` |
| vinext source                            | [`7f1feb134`](https://github.com/fengmk2/vinext/tree/7f1feb134255f9e816e1b03760b97c10148eed3c), Node.js `26.13.0`         |
| Lint threads                             | `4` for both lint workloads                                                                                               |
| Full check threads                       | Default `16`, confirmed in format and lint output                                                                         |
| Repetitions per project/workload/backend | 10 timing runs, 5 memory runs, and one warm-up before each set                                                            |

The three workloads are:

```sh
vp lint --type-aware --threads 4 --format json -c <rules-only-config>
vp lint --type-aware --type-check --threads 4 --format json
vp check
```

The temporary rules-only JSON contains the project's `lint` section with
`options.typeCheck` set to `false`; all rules, plugins, and ignore patterns
are retained. Vinext lint uses sorted checker scheduling under both controls.
Full checks and Vite+ lint use default checker scheduling.

Both controls use the same source and installed dependencies. Vite+ retains its
own workspace dependency links; the measured CLI, Oxlint, and native backends
come from the published preview. Vinext links the preview CLI and core into its
workspace and pnpm peer importers. These temporary links are restored afterwards.
All commands use isolated temporary `VP_HOME` directories and disable the Node
compile cache. Setup, configuration extraction, and output validation are excluded
from the timed intervals; CLI startup and command execution are included.

Native executable paths are verified in every validation and memory run. Package
versions, executable hashes, source commits, and lockfile hashes are recorded.
Tracked project sources and lockfiles are unchanged after measurement.

| Project | Linted files | Rules | Formatted files in `vp check` |
| ------- | -----------: | ----: | ----------------------------: |
| Vite+   |          392 |   174 |                           682 |
| vinext  |        1,842 |   151 |                         4,755 |

## Interpretation and reproducibility

This is a local desktop measurement with warm filesystem caches, not a cold-cache
or isolated-server result. CPU use measured for 0.5 seconds before each timed run
has a median of 12.1% across the machine (range 8.0%–23.9%).
The CPU frequency is not fixed. Compare the paired backends within each row;
the interquartile ranges describe sample spread, not confidence intervals.

The vinext input contains both the Sentry fixture exclusion from its draft PR
and the upstream [`prefer-shared-utils` optimization](https://github.com/cloudflare/vinext/commit/5a6cc15f39ca7e93bb1892efdcf2d3d01aa1b562).
Its original plugin rules remain enabled. Both controls use these same inputs.

All measured samples are retained; there is no outlier filtering. Reductions use
`100 * (1 - tsrs median / Go median)`. Raw samples, commands, memory traces,
dependency receipts, and the local runner remain outside the repository.

## Installed disk footprint

The local **2026-11-16** comparison measures clean standalone production
installations on macOS ARM64 and APFS with 4 KiB allocation blocks.
The Go baseline is published `vite-plus@1.5.0`. The tsrs installation uses
the same `0.0.0-commit.3345a7bf89b1f8c5e3a3afa6d852446151256835` preview as the timing and memory runs.

| Scope                                         | Go release (MiB) | tsrs preview (MiB) | Change (MiB) | Change |
| --------------------------------------------- | ---------------: | -----------------: | -----------: | -----: |
| Complete `node_modules`, allocated on disk    |           130.09 |             138.81 |        +8.71 |  +6.7% |
| Complete `node_modules`, regular-file lengths |           123.01 |             129.75 |        +6.74 |  +5.5% |
| Backend packages, allocated on disk           |            21.22 |              28.97 |        +7.75 | +36.5% |

One MiB is 1,048,576 bytes. Allocated sizes come from `du -sk node_modules`,
cross-checked against filesystem block counts. File lengths count each regular
file inode once. Symlinks are not followed. Allocated blocks do not measure the
change in free space across the APFS volume.

The baseline backend is `oxlint-tsgolint@7.0.2005` plus its Darwin ARM64 package.
The preview backend is `tsrs@7.1.0-beta.tsrs2` plus its Darwin ARM64 package.
The preview production tree contains no `oxlint-tsgolint`; the root benchmark
development dependency is excluded.

Backend packages account for **+7.75 MiB** of the **+8.71 MiB** total change.
Other dependencies and installation metadata account for **+0.97 MiB**.
This release-to-preview comparison includes the dependency changes below.
Timing and memory instead isolate the backend within one preview.

| Dependency | Go release | tsrs preview |
| ---------- | ---------- | ------------ |
| `oxlint`   | `1.91.0`   | `1.93.0`     |
| `oxfmt`    | `0.76.0`   | `0.78.0`     |

### Installation and validation

Both installs use Node.js `24.22.2`, pnpm `10.34.9`, and their published
`vp-install-lock.yaml`, extracted from the corresponding platform archive after
integrity verification. The wrapper manifest declares only `vite-plus`.
Installation uses `--ignore-workspace --frozen-lockfile`, the installer peer
settings, and `--ignore-scripts`. Each install has its own store and cache.
`package-import-method=copy` prevents hard links to the store.

The baseline contains **81 packages** and the preview **81 packages**.
The total includes pnpm metadata, bin shims, and the installation receipt.
Stores, download caches, Node.js, pnpm, and the global executable outside
`node_modules` are excluded.

Both installations report the expected versions, accept the valid type-check
probe, and reject the invalid probe with `TS2322`. Process traces confirm their
default native backends. Smoke checks leave tree sizes and lockfiles unchanged.
Temporary installations and isolated `VP_HOME` directories are removed after
measurement; inventories and logs remain outside the repository.

The installer lockfile SHA256 values are:

- Go release: `eb06955c3e76589da771286d3bfc6e6ea940c1a09ac64f597d2c72eb94403ad1`.
- tsrs preview: `d7a0b007f74fa7130b3414597bd69ec7780332641ebf3f9dbbf3633c5480f3c7`.
