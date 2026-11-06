# Type-aware linting with tsrs-lint

This comparison uses the default `tsrs@0.22.0-ts7.1.0-beta` integration and an explicit
`oxlint-tsgolint@7.0.2005` override. Both use Oxlint `1.91.0` and the same built CLI.

The separate [ecosystem correctness comparison](./tsrs-lint-ecosystem.md) found
compiler regressions and library differences in representative forks. The performance
results below do not establish diagnostic equivalence.

## Elapsed time

Each value is the median of ten fresh CLI processes after one warm-up per backend.
The runner alternates the order between pairs. Timing runs do not sample memory.

| Project  | Workload                         | Go (s) | tsrs (s) | Reduction |
| -------- | -------------------------------- | -----: | -------: | --------: |
| Vite+    | Type-aware rules                 |  3.712 |    2.212 |     40.4% |
| Vite+    | Rules and TypeScript diagnostics |  4.612 |    2.382 |     48.4% |
| Vite+    | Full `vp check`                  |  5.842 |    3.554 |     39.2% |
| vinext   | Type-aware rules                 | 52.028 |   45.363 |     12.8% |
| vinext † | Rules and TypeScript diagnostics | 54.192 |   44.794 |     17.3% |
| vinext   | Full `vp check`                  | 62.325 |   50.355 |     19.2% |

† The clean timing repetition used an EPYC 7763 after a Codespace restart.
All other timing rows used an EPYC 9V74. Both backends within each row used
the same CPU. Compare backend values within a row; the CPU change prevents
a strict comparison of absolute times across workloads.

## Peak memory

Each value is the median of five separate runs after one warm-up per backend.
Memory is the sampled peak of the simultaneous sum of RSS for the CLI and its
descendants, including Node, Oxlint, and the native backend. One GiB is 1,073,741,824 bytes.

| Project | Workload                         | Go (GiB) | tsrs (GiB) | Reduction |
| ------- | -------------------------------- | -------: | ---------: | --------: |
| Vite+   | Type-aware rules                 |    1.175 |      0.930 |     20.9% |
| Vite+   | Rules and TypeScript diagnostics |    1.345 |      0.984 |     26.8% |
| Vite+   | Full `vp check`                  |    1.438 |      1.017 |     29.3% |
| vinext  | Type-aware rules                 |    4.382 |      3.030 |     30.8% |
| vinext  | Rules and TypeScript diagnostics |    4.903 |      3.166 |     35.4% |
| vinext  | Full `vp check`                  |    5.413 |      3.292 |     39.2% |

The sampler sleeps for 20 ms. The median observed interval across runs was
21.8 ms. Shared pages can count more than once,
and sampling can miss short peaks. These values are process-tree RSS, not total
allocation or container memory.

## Installed disk footprint

Clean production installations on Linux x64 with glibc compare published `1.5.0`
with preview `0.0.0-commit.4fcf99edbe42fe3305dcd41dc884a4c4b7f5fd45`.
The preview binary-size comment measures individual executables and native bindings;
it does not measure the installed dependency tree.

| Scope                            | Go (MiB) | tsrs (MiB) | Increase (MiB) | Increase |
| -------------------------------- | -------: | ---------: | -------------: | -------: |
| Complete `node_modules`, on disk |   132.93 |     150.87 |          17.94 |    13.5% |
| Complete `node_modules`, files   |   123.39 |     139.27 |          15.88 |    12.9% |
| Backend packages, on disk        |    21.63 |      39.69 |          18.06 |    83.5% |

One MiB is 1,048,576 bytes. On-disk values use `du -s -B1 node_modules` and
include directory allocation. File sizes sum regular-file lengths without following
symlinks or counting an inode twice. The filesystem uses 4 KiB blocks.
Stores, download caches, Node.js, pnpm, and the global `vp` executable are excluded.

The job used Node.js `24.22.2`, pnpm `10.34.7`, and each version's published
`vp-install-lock.yaml`. It reproduced the global installer's manifest and frozen
installation flags. Separate stores and `--config.package-import-method=copy`
prevent shared hard links from affecting the standalone footprint. Each installation
contained 80 packages. Version output and type-aware linting with TypeScript
diagnostics passed for both installations. The preview contained no `oxlint-tsgolint`.

The backend totals include `oxlint-tsgolint` and `@oxlint-tsgolint/linux-x64`, or
`tsrs` and `@tsrs/linux-x64-gnu`. The Rust distribution includes a larger executable,
TypeScript library declarations, JavaScript APIs, and source maps. Its package
footprint increases despite the runtime and memory improvements. The total
installation comparison also includes other release-to-preview changes;
the backend row separates the backend packages from those changes.

The measurement ran on 2026-11-06 in Codespaces job `job-2a30c8ec3fb84524`.
Scripts, raw package inventories, lockfiles, and logs remain outside the repository.
The lockfile SHA256 values are
`eb06955c3e76589da771286d3bfc6e6ea940c1a09ac64f597d2c72eb94403ad1` for `1.5.0`
and `cf8fe673142ace0abaa362552bff78281f3b8f00e01bc13fa0eb93c5b1344683` for the preview.

## Conditions and interpretation

The jobs used one GitHub Codespace with four vCPUs, 16 GB advertised RAM, and an
AMD EPYC 9V74 processor for the main run. The focused timing repetition used an
AMD EPYC 7763 after the instance restarted. Usable RAM was 15.62 GiB.
The container used Ubuntu 22.04 on Linux x64 with glibc. Vite+ used Node.js
`24.22.2`; vinext used Node.js `26.13.0`.

The CLI JavaScript came from `22b07bd150aa57b3e04cc52e0b81d43294ffe982`.
Its native binding, core, and prompts artifacts came from the published Vite+
`1.5.0` packages. The measured Vite+ source also includes the RFC patch at
`da260f86fce1d43810c62e4e6aad011066c88cb9`. Vinext is pinned to
`249088af307c0a852fa6dc59c08fc6ec80e1620e`, from the requested upgrade branch.
This is a comparison of backend routing in the built CLI, not a full native release build.

Lint uses four threads. Vinext lint sets
`OXLINT_TSGOLINT_CHECKER_SCHEDULING=sorted`; Vite+ lint uses default scheduling.
Full checks use four threads and default backend scheduling. The rules-only
configuration disables TypeScript diagnostics while retaining the project's
original rules and plugins. Setup, builds, validation, and memory sampling are
excluded from the elapsed-time table.

Both backends passed all 12 validation commands and every benchmark invocation.
Vite+ checked 385 source files with 174 rules and formatted 632 files. Vinext
checked 1,828 source files with 151 rules and formatted 4,673 files. These clean
corpora do not establish complete diagnostic or compiler parity.

Vinext retains its original JS plugin rules, including
`vinext-utils/prefer-shared-utils`. The backend switch leaves that shared work
in place. This limits the elapsed-time improvement even though the memory
reduction is larger. The measurements do not include that rule's separate optimization.

## Run selection

The main job completed 204 invocations, including warm-ups.
A browser connection used to diagnose an SSH
outage started editor extension activity during vinext's type-checking timing
set. That entire timing set is excluded. The table instead uses the 22 records
from a clean timing repetition.
The repetition uses the same retained build and dependencies, with their hashes
checked before execution. All memory values come from the main job.

Warm-up runs do not contribute to medians. Each reduction is calculated as
`100 * (1 - rust / go)` from the corresponding backend medians.
