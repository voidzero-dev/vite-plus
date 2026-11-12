# Type-aware linting with tsrs-lint

The timing and memory results below were measured locally on **2026-11-13** with
the latest published [PR #3352 preview](https://github.com/voidzero-dev/vite-plus/actions/runs/43194215156),
`0.0.0-commit.25a7c006af2db834bfc66f9689b02de1060f580f`.
Both controls use the same published CLI and Oxlint `1.93.0`: the Go control uses
`oxlint-tsgolint@7.0.2006`, and the default preview uses `tsrs@0.25.0-ts7.1.0-beta`.
The subsequent source commit `68eb84008` changes only reports.

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
| Vite+   | Type-aware rules                 | 1.518 [1.489–1.654] | 1.371 [1.255–1.399] |      9.7% |
| Vite+   | Rules and TypeScript diagnostics | 1.861 [1.820–1.928] | 1.410 [1.316–1.504] |     24.2% |
| Vite+   | Full `vp check`                  | 2.112 [2.102–2.243] | 1.674 [1.646–1.795] |     20.8% |
| vinext  | Type-aware rules                 | 2.677 [2.616–2.791] | 1.572 [1.533–1.656] |     41.3% |
| vinext  | Rules and TypeScript diagnostics | 3.364 [3.296–3.454] | 1.695 [1.635–1.728] |     49.6% |
| vinext  | Full `vp check`                  | 4.680 [4.563–4.731] | 2.178 [2.095–2.184] |     53.4% |

## Peak memory

Each cell shows the median of **5 separate runs** and its interquartile range.
One warm-up per backend is excluded. Memory is the sampled peak of the simultaneous
sum of RSS for the CLI and its descendants, including Node, Oxlint, and the native
backend. The sampler itself is excluded. One GiB is 1,073,741,824 bytes.

| Project | Workload                         |            Go (GiB) |          tsrs (GiB) | Reduction |
| ------- | -------------------------------- | ------------------: | ------------------: | --------: |
| Vite+   | Type-aware rules                 | 1.103 [1.047–1.153] | 0.633 [0.601–0.647] |     42.6% |
| Vite+   | Rules and TypeScript diagnostics | 1.276 [1.258–1.462] | 0.651 [0.644–0.670] |     49.0% |
| Vite+   | Full `vp check`                  | 1.325 [1.269–1.332] | 0.706 [0.687–0.715] |     46.7% |
| vinext  | Type-aware rules                 | 3.427 [3.407–3.632] | 1.852 [1.836–1.889] |     46.0% |
| vinext  | Rules and TypeScript diagnostics | 3.694 [3.662–3.904] | 1.967 [1.956–1.977] |     46.8% |
| vinext  | Full `vp check`                  | 5.014 [4.756–5.391] | 2.102 [2.090–2.102] |     58.1% |

The sampler sleeps for 20 ms; the median observed interval is 38.8 ms
(the median of the measured runs' sampling intervals). Shared pages can count more
than once, and sampling can miss short peaks. These are process-tree RSS values,
not total allocations or macOS physical-footprint measurements.

## Inputs and method

| Input                                    | Pin / condition                                                                                                           |
| ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| Machine                                  | Apple M4 Max, 16 CPU cores, 48 GiB RAM, macOS `27.0.1`, ARM64                                                             |
| Vite+ source                             | [`68eb84008`](https://github.com/voidzero-dev/vite-plus/tree/68eb84008fd27a72a5e9426ad64636432385b0e1), Node.js `24.22.2` |
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
| Vite+   |          387 |   174 |                           675 |
| vinext  |        1,842 |   151 |                         4,755 |

## Interpretation and reproducibility

This is a local desktop measurement with warm filesystem caches, not a cold-cache
or isolated-server result. CPU use measured for 0.5 seconds before each timed run
has a median of 12.4% across the machine (range 7.2%–23.9%).
The CPU frequency is not fixed. Compare the paired backends within each row;
the interquartile ranges describe sample spread, not confidence intervals.

The vinext input contains both the Sentry fixture exclusion from its draft PR
and the upstream [`prefer-shared-utils` optimization](https://github.com/cloudflare/vinext/commit/5a6cc15f39ca7e93bb1892efdcf2d3d01aa1b562).
Its original plugin rules remain enabled. These source inputs, the hardware,
thread counts, and tool versions differ from the earlier Codespaces measurements.
Changes in absolute times or reduction percentages cannot be attributed solely
to the tsrs version update.

All measured samples are retained; there is no outlier filtering. Reductions use
`100 * (1 - tsrs median / Go median)`. Raw samples, commands, memory traces,
dependency receipts, and the local runner remain outside the repository.

## Historical installed disk footprint

The following disk measurements retain the **2026-11-06 Linux x64** comparison
of published `1.5.0` and the `0.22.0` preview. They are not measurements of the
current macOS preview. The [original report](https://github.com/voidzero-dev/vite-plus/blob/68eb84008fd27a72a5e9426ad64636432385b0e1/bench/tsrs-lint.md#installed-disk-footprint)
contains the installation method, package inventory scope, and lockfile hashes.

| Scope                                         | Go (MiB) | tsrs (MiB) | Increase |
| --------------------------------------------- | -------: | ---------: | -------: |
| Complete `node_modules`, on disk              |   132.93 |     150.87 |    13.5% |
| Complete `node_modules`, regular-file lengths |   123.39 |     139.27 |    12.9% |
| Backend packages, on disk                     |    21.63 |      39.69 |    83.5% |

One MiB is 1,048,576 bytes. The total installation comparison includes other
release-to-preview dependency changes; the backend row isolates the backend
packages. These disk figures do not describe runtime memory use.
