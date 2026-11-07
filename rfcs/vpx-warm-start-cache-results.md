# vpx Warm Start Cache Prototype Results

- Status: Measured prototype; caching remains opt-in
- Date: 2026-10-27
- Design: [Warm-start cache RFC](./vpx-warm-start-cache.md)

The October 27 benchmark tables describe the original prototype. After the rebase onto `0bea91491`, a [November 4 profile](#warm-transform-cache-hits-after-the-rebase) measured the two 300-module graphs on the updated loader and compiler. A [follow-up experiment](#packing-small-records-by-source-directory) compared packed storage with individual files and caching disabled.

## Outcome

The first version does not reach the 10× end-to-end warm-start target. Large single-source fixtures benefit from the combined Oxc and Node caches, but the 300-module graphs become slower despite successful artifact hits. Keep the prototype disabled by default and profile the per-module cache overhead before considering rollout.

When `vpx` selects the built prototype package, `VPX_CACHE=1` enables both new layers. `VPX_CACHE=transform` and `VPX_CACHE=compile` isolate them; `VPX_DISABLE_CACHE=1` disables the prototype. The installed global launcher must resolve that local package for these controls to apply. The existing local launcher's normal Node compile-cache behavior is preserved in every comparison.

## Main results on Node.js 24.22.1

All five existing fixtures are retained: one small TypeScript file (`hello`), 300-module ESM and CommonJS graphs (`esm-app`, `cjs-app`), and roughly 1.8 MB single-source CommonJS and ESM cases (`large-cjs`, `large-esm`). The [shared fixture generator](../bench/vpx-fixtures.ts) also serves the historical benchmark.

Times below are launch-to-ready milliseconds, shown as **p50 / p95**. Speedup is baseline p50 divided by combined-cache p50; less than 1× means a regression. The interval is a paired-block bootstrap 95% confidence interval for that ratio.

### Global vpx

| Fixture     | Baseline ms   | Combined ms   | Speedup | 95% interval |
| ----------- | ------------- | ------------- | ------- | ------------ |
| `hello`     | 35.07 / 38.66 | 35.86 / 39.81 | 0.98×   | 0.96–1.00×   |
| `esm-app`   | 73.70 / 82.29 | 85.87 / 98.70 | 0.86×   | 0.83–0.87×   |
| `cjs-app`   | 73.44 / 82.26 | 84.69 / 98.78 | 0.87×   | 0.84–0.88×   |
| `large-cjs` | 79.39 / 84.77 | 48.16 / 53.42 | 1.65×   | 1.61–1.67×   |
| `large-esm` | 72.34 / 80.40 | 46.65 / 51.46 | 1.55×   | 1.51–1.58×   |

### Local vpx and direct loader

Local rows include the project-local Node launcher. Direct-loader rows omit the global/local launcher and are diagnostic, not user-facing acceptance results.

| Fixture / launcher   | Baseline ms    | Combined ms     | Speedup | 95% interval |
| -------------------- | -------------- | --------------- | ------- | ------------ |
| `hello` / local      | 56.85 / 62.52  | 56.72 / 62.43   | 1.00×   | 0.98–1.03×   |
| `esm-app` / local    | 94.03 / 104.60 | 105.87 / 121.58 | 0.89×   | 0.87–0.91×   |
| `cjs-app` / local    | 93.90 / 103.22 | 106.32 / 129.55 | 0.88×   | 0.86–0.89×   |
| `large-cjs` / local  | 99.27 / 110.05 | 69.38 / 76.06   | 1.43×   | 1.40–1.47×   |
| `large-esm` / local  | 93.17 / 101.33 | 68.81 / 76.47   | 1.35×   | 1.32–1.38×   |
| `hello` / direct     | 29.35 / 32.66  | 30.11 / 32.92   | 0.97×   | 0.95–1.00×   |
| `esm-app` / direct   | 66.87 / 78.95  | 80.95 / 96.90   | 0.83×   | 0.81–0.85×   |
| `cjs-app` / direct   | 67.67 / 78.04  | 79.94 / 95.36   | 0.85×   | 0.83–0.86×   |
| `large-cjs` / direct | 73.82 / 79.09  | 42.88 / 47.08   | 1.72×   | 1.68–1.75×   |
| `large-esm` / direct | 66.57 / 71.68  | 41.87 / 46.59   | 1.59×   | 1.57–1.62×   |

## Cache layers and regressions

This table isolates the two cache layers on the global launcher. Cells are warm readiness p50 milliseconds.

| Fixture     | Baseline | Disabled | Transform | Compile | Combined |
| ----------- | -------- | -------- | --------- | ------- | -------- |
| `hello`     | 35.07    | 35.98    | 35.66     | 35.63   | 35.86    |
| `esm-app`   | 73.70    | 72.66    | 81.31     | 77.60   | 85.87    |
| `cjs-app`   | 73.44    | 73.61    | 82.02     | 77.39   | 84.69    |
| `large-cjs` | 79.39    | 80.26    | 61.78     | 66.66   | 48.16    |
| `large-esm` | 72.34    | 73.62    | 60.14     | 60.41   | 46.65    |

The graph regression is already present with transform caching alone; enabling only Node's compile cache also regresses these fixtures. The large-source cases benefit from each layer and improve further when both are enabled. These ablations identify which policy adds cost, not the time spent in any individual filesystem or compiler phase.

With the prototype disabled, warm p50 changes range from -1.04 to +2.07 ms across all Node.js 24 rows (-1.4% to +2.7%). Warm p95 changes range from -3.4% to +3.9%. Small positive deltas should be investigated rather than described as zero overhead; these point estimates do not establish statistical significance.

### Cold artifact costs

Cold readiness p50 on the global launcher, in milliseconds:

| Fixture     | Baseline | Transform | Compile | Combined | Combined change    |
| ----------- | -------- | --------- | ------- | -------- | ------------------ |
| `hello`     | 38.34    | 39.83     | 40.55   | 39.77    | +1.43 ms / +3.7%   |
| `esm-app`   | 81.44    | 147.14    | 92.65   | 153.12   | +71.68 ms / +88.0% |
| `cjs-app`   | 80.14    | 153.24    | 88.37   | 155.07   | +74.93 ms / +93.5% |
| `large-cjs` | 87.87    | 95.36     | 85.96   | 98.39    | +10.53 ms / +12.0% |
| `large-esm` | 77.59    | 86.54     | 83.99   | 91.21    | +13.61 ms / +17.5% |

Across all three launchers, combined-cache cold graph medians increase by 63.4–95.3%. The cold penalty is already visible in transform-only mode. Cold runs have only five samples per row, so borderline threshold crossings need confirmation; the large graph regressions are not borderline.

Cold launch-to-exit medians also increase: `esm-app` 83.85 → 199.45 ms; `cjs-app` 83.26 → 198.88 ms. These global-launcher results include work after readiness, which requires separate profiling.

| RFC gate                          | Observed result                                                                                    |
| --------------------------------- | -------------------------------------------------------------------------------------------------- |
| 10× end-to-end warm startup       | Not met by any measured global or local row.                                                       |
| Warm p95 regression budget of 5%  | The `hello` rows change by -0.1% to +3.0%; graph rows regress by 16.2–25.5% and exceed the budget. |
| Cold p50 regression budget of 10% | 11 of 15 Node.js 24 combined-cache point estimates exceed it, including every graph row.           |
| Default-on rollout                | Blocked by the performance regressions; retain explicit opt-in.                                    |

## Supplemental runtime results

The same builds and global launcher were measured on Node.js 22.18.0, which uses off-thread ESM hooks, and Node.js 26.12.0. Only baseline and combined policies were compared. Times remain warm readiness p50 / p95 milliseconds.

### Node.js 22.18.0

| Fixture     | Baseline ms     | Combined ms    | Speedup | 95% interval |
| ----------- | --------------- | -------------- | ------- | ------------ |
| `hello`     | 47.77 / 51.17   | 48.40 / 54.06  | 0.99×   | 0.98–0.99×   |
| `esm-app`   | 90.00 / 101.37  | 98.12 / 107.57 | 0.92×   | 0.90–0.93×   |
| `cjs-app`   | 89.50 / 95.82   | 97.08 / 105.71 | 0.92×   | 0.91–0.94×   |
| `large-cjs` | 113.02 / 120.55 | 81.59 / 87.35  | 1.39×   | 1.37–1.40×   |
| `large-esm` | 97.76 / 101.74  | 72.09 / 75.96  | 1.36×   | 1.33–1.36×   |

### Node.js 26.12.0

| Fixture     | Baseline ms   | Combined ms   | Speedup | 95% interval |
| ----------- | ------------- | ------------- | ------- | ------------ |
| `hello`     | 34.63 / 36.21 | 35.06 / 36.93 | 0.99×   | 0.97–1.00×   |
| `esm-app`   | 69.56 / 74.08 | 80.30 / 88.06 | 0.87×   | 0.85–0.89×   |
| `cjs-app`   | 70.14 / 77.05 | 80.77 / 87.07 | 0.87×   | 0.86–0.89×   |
| `large-cjs` | 77.36 / 82.83 | 46.81 / 51.54 | 1.65×   | 1.63–1.68×   |
| `large-esm` | 69.85 / 73.13 | 45.90 / 48.04 | 1.52×   | 1.50–1.54×   |

## Cache diagnostics and validation

Separate post-measurement diagnostic launches on Node.js 24.22.1 and 26.12.0 report 304 native hits for each graph, four for `hello` and `large-esm`, and five for `large-cjs`, with zero native misses, writes, or errors in the transform-enabled policies. These are cache operations, not module counts. The diagnostic launches are excluded from the timing summaries.

The native counters are thread-local. On Node.js 22.18.0 the main-thread probe cannot observe off-thread ESM counters: it reports zero for most fixtures and one hit for `large-cjs`. These values do not prove missing ESM caching. No Node/V8 compile-cache hit counter was collected; activation and ablation timings are not proof of individual V8 hits.

Validation completed: Rust check/test/lint; six artifact-store tests and six upstream cache tests; 2,807 TypeScript tests with one skipped; 54 CLI snapshots; `vp check`; and task-cache source/config invalidation smoke checks. The benchmark validates readiness, stdout, stderr, exit status, a fresh execution side effect, actual Node executable/version, and selected loader on every cold, priming, warm, and diagnostic launch.

The main run contains 4,425 successful launches; the supplemental run contains 1,180. Neither report contains a failed sample.

## Measurement method and artifact identity

- Machine: Apple M4 Max, 16 logical CPUs, 48 GiB memory; macOS arm64, Darwin 27.0.0. The benchmark parent ran Node.js 22.23.4.
- Baseline: Vite+ source `a9cc41dedf612b2a991da2968cb4107b46bccf38`, built with the same optimized release configuration and dependency pins as the prototype. The candidate includes this task's cache changes. Both package versions are `1.4.0`; hashes distinguish the actual builds. The generated `binding/index.cjs`, local launcher, and bin shim are byte-identical across the builds.
- Each row uses five cold artifact runs, three completed priming runs, and 50 warm runs. Each launch starts a fresh process. Policies are interleaved in seeded randomized blocks (`--seed 1`), with separate cache roots and temporary directories.
- Cold clears persistent artifacts and temporary compile caches, not OS filesystem caches. Dependency installation and fixture generation are outside the timings.
- The global launcher is the same installed executable for both contenders. Its isolated config uses `nodeShimMode: "system_first"`, with the requested runtime first on `PATH`; each run verifies the actual executable and selected project loader. Managed runtime selection is not measured here.
- Readiness follows fixture imports and initialization. A small JavaScript probe under `node_modules` avoids adding an Oxc transform, observes readiness, and writes a unique execution token after the marker. Exit time includes that write and cache shutdown work. These instrumented readiness numbers are not directly comparable with the earlier uninstrumented `spawnSync` launch-to-exit table.
- Confidence intervals use 2,000 deterministic bootstrap resamples (`random.Random(20261027)`). Each resample draws 50 iteration indices with replacement and applies the same indices to baseline and combined samples. The interval is the 2.5th–97.5th percentile of the ratio of their medians. It describes sampling uncertainty for this machine/run, not cross-machine or application performance.

The raw reports are local ignored artifacts, not checked-in files: `tmp/vpx-cache-v1-node24.json` and `tmp/vpx-cache-v1-node22-node26.json`. They retain chronological samples, all p50/p95 summaries, runtime identities, per-file package hashes, exact arguments, and diagnostics.

| Artifact                        | SHA256                                                             |
| ------------------------------- | ------------------------------------------------------------------ |
| Node.js 24.22.1                 | `6f32069480ec14d52b6d7b1f3c7124cfbbc46203abf5ddf5c65b127435f3f329` |
| Node.js 22.18.0                 | `9187ad22c98cea5b635a79db52fa32ab3f6aa9d41e3abf5da71437cfef1ca9de` |
| Node.js 26.12.0                 | `12ccc7c7d8b9b33ac0740fc7c2590d2aba14ccfa0a40f3d3f7a6c4c60c87f07c` |
| Shared global launcher          | `af04cd44d02ee80f170f47b37ab387c27db2a0528c73d697e851c8c09b8edc01` |
| Baseline native binding         | `39e27f8031acbcc597cd4076ceac27628603edb995b640d949a71c0dbf57ecd4` |
| Candidate native binding        | `a314b801365a0ac82c976b0a66696478ff580ee40a51e176f57b4438412e0c03` |
| Shared generated binding loader | `ac942895862b5986387e7bb2300ed8c6f5756cfcce1a4e8c2b2b3b3c6f622e6e` |

The global command path is `/Users/fengmk2/.vite-plus/bin/vpx`, resolving to `/Users/fengmk2/.vite-plus/1.3.0+force.70583.1792569205250812000/bin/vp`. Runtime executable paths are `/Users/fengmk2/.vite-plus/js_runtime/node/<version>/bin/node` for the exact versions above. Package roots are `/Users/fengmk2/git/worktrees/vp/vpx-cache-baseline/packages/cli` and `/Users/fengmk2/.codex/worktrees/d925/vp/packages/cli`.

## Warm transform-cache hits after the rebase

On November 4, both 300-module graphs spent about 77% of native cache-hit time accessing artifact files. The table shows median milliseconds per global `vpx` launch, summed over 304 hits. Each warm cache-enabled run recorded zero misses, writes, and errors.

| Work inside a native hit                | ESM (ms) | CommonJS (ms) |
| --------------------------------------- | -------: | ------------: |
| Prepare key inputs, including options   |     0.65 |          0.66 |
| Hash key inputs with BLAKE3             |     0.88 |          0.88 |
| Check directory and file metadata       |     1.71 |          1.71 |
| Allocate buffer, open, read, close      |     5.21 |          5.36 |
| Verify payload checksum with BLAKE3     |     0.38 |          0.38 |
| Complete native lookup, including above |     8.98 |          9.10 |

Formatting the compiler options took about 0.63 ms per launch. The two BLAKE3 operations together took about 1.26 ms. The filesystem timings include warm OS cache access. The complete lookup also includes path construction, record decoding, and timer overhead. It starts at cache-request creation and ends when the store returns a hit.

The uninstrumented release took 71.80 ms with caching disabled and 78.82 ms with transform caching on ESM (+7.02 ms). CommonJS rose from 72.06 to 78.53 ms (+6.47 ms). These startup differences include the transform work that hits skip. With timers enabled, startup medians were 78.56 and 78.37 ms, respectively. Bypassing the global launcher yielded the same 77% filesystem share on both graphs.

The experiment used commit `19d7d80cb`, Node.js `24.22.2`, an Apple M4 Max, and the repository's optimized release profile for both bindings. Each fixture, launcher, and mode ran four priming processes and 50 measured processes in interleaved blocks. Node's compile cache was disabled. Temporary probes accumulated per-thread wall times and reported them after readiness. The local samples and probe script are in `tmp/vpx-profile-19d7d80cb/`.

## Packing small records by source directory

The [packed store](../packages/cli/binding/src/oxc_node/cache/packed.rs) loads related records together and indexes their full content keys in memory. Both fixtures put all 300 leaf modules in one source directory. Separate diagnostic launches traced file API calls under the transform-cache root on macOS. The counts below were identical for ESM and CommonJS across two warm processes each.

| Per warm process     | Individual files | Packed |
| -------------------- | ---------------: | -----: |
| File metadata checks |              612 |      6 |
| Open                 |              304 |      2 |
| Read, including EOF  |              610 |      6 |
| Close                |              304 |      2 |

All 304 lookups hit in both layouts. The packed processes read about 496–498 kB of artifact data each, within 0.3% of the individual-file totals.

The table shows warm global `vpx` launch-to-ready p50 in milliseconds.

| Fixture  | Run | Cache disabled | Individual files | Packed | Packed minus disabled |
| -------- | --: | -------------: | ---------------: | -----: | --------------------: |
| ESM      |   1 |          80.33 |            89.93 |  80.01 |                 −0.32 |
| ESM      |   2 |          73.84 |            81.74 |  72.71 |                 −1.13 |
| CommonJS |   1 |          82.01 |            91.49 |  81.23 |                 −0.78 |
| CommonJS |   2 |          74.70 |            81.12 |  73.83 |                 −0.87 |

Each run used a fresh cache root, four priming processes, and 120 independently launched processes per mode. Each randomized block exercised all four modes, including cache-off controls for both binaries. The table compares packed storage with its own binary's cache-off control. Both release builds used Node.js `24.22.2` on the same M4 Max, with Node's compile cache disabled.

Packing saved 7.3–10.3 ms versus individual files. Its advantage over cache-off was smaller. The 95% bootstrap intervals for each run's difference in p50 include zero. Comparing launches within the same block reduces the effect of changes in machine load. Across 240 blocks, the median paired saving was 1.02 ms for each graph (95% intervals of 0.72–1.36 ms for ESM and 0.56–1.49 ms for CommonJS). These intervals resample whole blocks within each run.

Cold global p50 for the packed store was 101.71 ms on ESM and 96.54 ms on CommonJS. Cache-off took 81.00 and 81.46 ms, respectively, over five cold launches per mode. Warm p95 was faster with packed storage in run 1 and slower in run 2 for both graphs.

The local timing samples are in `tmp/vpx-packed-bench/run1.json` and `tmp/vpx-packed-bench/repeat.json`. The file-operation traces are in `tmp/vpx-packed-io/`. The individual-file and packed native binding SHA256 digests start with `fadc32118c99` and `a4a69354de33`, respectively.

## Next measurements

Node compile-cache work on the 300-module fixtures still needs a separate profile. For cold starts, measure directory checks, publication, and per-record writes. Transform-only timings show that this path already contains most of the observed cold graph penalty.

Measure packed lookups on projects that import from many directories or have accumulated revisions. Other experiments include reusing normalized option fingerprints and skipping transforms that cost less to recompute than to retrieve.

This is one macOS machine and synthetic workloads only. Real scripts, watch-restart timing, Windows/Linux runtime validation, and cache garbage collection remain outstanding. The results do not justify narrowing the 10× objective or enabling the prototype by default.

## Reproduction

Build baseline and candidate packages with matching optimized release settings, following the repository's build and mbx instructions. Preserve each package's `dist/` and `binding/` files. Run the harness from the candidate checkout with real absolute executable/package paths:

```sh
node bench/vpx-cache.ts \
  --baseline-package /absolute/baseline/packages/cli \
  --candidate-package /absolute/candidate/packages/cli \
  --node /absolute/node24/bin/node \
  --vpx /absolute/bin/vpx \
  --runs 50 --warmup 3 --cold-runs 5 --seed 1 \
  --diagnostics --json tmp/vpx-cache-v1-node24.json

node bench/vpx-cache.ts \
  --baseline-package /absolute/baseline/packages/cli \
  --candidate-package /absolute/candidate/packages/cli \
  --node /absolute/node22/bin/node --node /absolute/node26/bin/node \
  --vpx /absolute/bin/vpx --launcher global \
  --mode baseline --mode combined \
  --runs 50 --warmup 3 --cold-runs 5 --seed 1 \
  --diagnostics --json tmp/vpx-cache-v1-node22-node26.json
```

## Complete timing matrices

Every cell below is **p50 / p95 in milliseconds**. Cold and warm readiness and exit results are retained for every measured mode and launcher. Priming and diagnostic launches are excluded.

<details>
<summary>Node.js 24.22.1 — all five modes and three launchers</summary>

| Fixture     | Launcher | Mode      | Cold ready      | Cold exit       | Warm ready      | Warm exit       |
| ----------- | -------- | --------- | --------------- | --------------- | --------------- | --------------- |
| `hello`     | global   | baseline  | 38.34 / 63.90   | 40.41 / 65.94   | 35.07 / 38.66   | 36.92 / 40.88   |
| `hello`     | global   | disabled  | 37.02 / 38.42   | 39.45 / 40.83   | 35.98 / 39.97   | 38.14 / 43.08   |
| `hello`     | global   | transform | 39.83 / 40.43   | 41.99 / 42.75   | 35.66 / 39.55   | 37.99 / 41.91   |
| `hello`     | global   | compile   | 40.55 / 41.01   | 43.97 / 44.82   | 35.63 / 39.72   | 37.75 / 42.01   |
| `hello`     | global   | combined  | 39.77 / 40.48   | 42.90 / 43.60   | 35.86 / 39.81   | 38.08 / 41.97   |
| `hello`     | local    | baseline  | 60.33 / 63.24   | 62.54 / 65.29   | 56.85 / 62.52   | 58.76 / 64.72   |
| `hello`     | local    | disabled  | 62.13 / 64.58   | 64.13 / 66.64   | 57.22 / 61.41   | 59.28 / 63.78   |
| `hello`     | local    | transform | 65.24 / 66.87   | 67.88 / 69.23   | 56.46 / 61.87   | 58.73 / 64.06   |
| `hello`     | local    | compile   | 63.88 / 65.50   | 67.05 / 68.51   | 57.66 / 62.71   | 59.93 / 65.45   |
| `hello`     | local    | combined  | 65.71 / 66.34   | 68.80 / 69.74   | 56.72 / 62.43   | 58.86 / 64.74   |
| `hello`     | direct   | baseline  | 31.40 / 33.01   | 33.37 / 35.01   | 29.35 / 32.66   | 31.17 / 34.73   |
| `hello`     | direct   | disabled  | 32.77 / 35.58   | 35.31 / 38.03   | 30.13 / 32.90   | 32.35 / 35.01   |
| `hello`     | direct   | transform | 33.76 / 38.25   | 36.36 / 40.66   | 30.24 / 33.04   | 32.38 / 35.35   |
| `hello`     | direct   | compile   | 32.73 / 32.96   | 36.02 / 36.28   | 30.33 / 33.27   | 32.65 / 35.53   |
| `hello`     | direct   | combined  | 34.37 / 37.09   | 37.58 / 40.24   | 30.11 / 32.92   | 32.21 / 35.20   |
| `esm-app`   | global   | baseline  | 81.44 / 83.23   | 83.85 / 86.09   | 73.70 / 82.29   | 76.08 / 84.81   |
| `esm-app`   | global   | disabled  | 82.42 / 84.71   | 85.32 / 87.47   | 72.66 / 84.75   | 75.13 / 87.11   |
| `esm-app`   | global   | transform | 147.14 / 155.07 | 150.06 / 158.10 | 81.31 / 95.92   | 83.77 / 98.27   |
| `esm-app`   | global   | compile   | 92.65 / 101.95  | 140.04 / 189.35 | 77.60 / 87.53   | 79.99 / 90.09   |
| `esm-app`   | global   | combined  | 153.12 / 204.70 | 199.45 / 263.45 | 85.87 / 98.70   | 88.33 / 101.18  |
| `esm-app`   | local    | baseline  | 108.11 / 114.51 | 110.76 / 116.94 | 94.03 / 104.60  | 96.42 / 107.08  |
| `esm-app`   | local    | disabled  | 107.10 / 108.52 | 110.66 / 110.98 | 94.47 / 107.51  | 96.75 / 109.90  |
| `esm-app`   | local    | transform | 175.66 / 198.47 | 177.95 / 200.99 | 102.40 / 120.93 | 104.82 / 123.66 |
| `esm-app`   | local    | compile   | 113.20 / 115.52 | 157.09 / 168.49 | 98.08 / 116.32  | 100.54 / 118.80 |
| `esm-app`   | local    | combined  | 176.60 / 202.63 | 220.24 / 247.33 | 105.87 / 121.58 | 108.21 / 124.44 |
| `esm-app`   | direct   | baseline  | 77.76 / 79.86   | 80.19 / 82.70   | 66.87 / 78.95   | 69.54 / 81.97   |
| `esm-app`   | direct   | disabled  | 76.61 / 84.00   | 79.29 / 86.59   | 67.83 / 76.26   | 70.26 / 81.02   |
| `esm-app`   | direct   | transform | 141.04 / 149.62 | 143.43 / 152.35 | 75.91 / 100.37  | 78.34 / 102.98  |
| `esm-app`   | direct   | compile   | 81.61 / 88.29   | 127.22 / 186.45 | 71.87 / 84.85   | 74.41 / 87.75   |
| `esm-app`   | direct   | combined  | 151.86 / 178.83 | 195.52 / 229.13 | 80.95 / 96.90   | 83.45 / 101.38  |
| `cjs-app`   | global   | baseline  | 80.14 / 82.86   | 83.26 / 85.78   | 73.44 / 82.26   | 76.22 / 84.90   |
| `cjs-app`   | global   | disabled  | 80.56 / 90.26   | 83.41 / 93.01   | 73.61 / 85.47   | 76.15 / 88.20   |
| `cjs-app`   | global   | transform | 153.24 / 179.91 | 155.84 / 182.80 | 82.02 / 95.34   | 84.63 / 97.96   |
| `cjs-app`   | global   | compile   | 88.37 / 94.98   | 131.78 / 140.64 | 77.39 / 86.88   | 80.18 / 89.56   |
| `cjs-app`   | global   | combined  | 155.07 / 163.83 | 198.88 / 209.38 | 84.69 / 98.78   | 87.28 / 103.04  |
| `cjs-app`   | local    | baseline  | 110.12 / 115.86 | 112.55 / 118.82 | 93.90 / 103.22  | 96.33 / 105.83  |
| `cjs-app`   | local    | disabled  | 105.89 / 113.53 | 108.50 / 117.46 | 95.97 / 106.90  | 98.59 / 109.56  |
| `cjs-app`   | local    | transform | 178.93 / 203.28 | 181.60 / 207.05 | 101.16 / 113.30 | 103.73 / 115.68 |
| `cjs-app`   | local    | compile   | 109.36 / 120.46 | 158.40 / 171.65 | 100.40 / 116.87 | 103.01 / 119.67 |
| `cjs-app`   | local    | combined  | 181.31 / 190.24 | 224.29 / 237.22 | 106.32 / 129.55 | 108.85 / 132.21 |
| `cjs-app`   | direct   | baseline  | 76.51 / 81.99   | 79.09 / 84.96   | 67.67 / 78.04   | 70.05 / 80.56   |
| `cjs-app`   | direct   | disabled  | 78.43 / 105.82  | 81.28 / 109.06  | 67.94 / 79.70   | 70.53 / 82.49   |
| `cjs-app`   | direct   | transform | 141.01 / 149.55 | 143.60 / 152.28 | 75.31 / 89.51   | 77.90 / 92.12   |
| `cjs-app`   | direct   | compile   | 82.03 / 87.68   | 133.58 / 138.24 | 72.20 / 85.66   | 74.98 / 88.34   |
| `cjs-app`   | direct   | combined  | 147.50 / 155.03 | 191.29 / 199.79 | 79.94 / 95.36   | 82.52 / 98.03   |
| `large-cjs` | global   | baseline  | 87.87 / 89.92   | 91.07 / 95.96   | 79.39 / 84.77   | 82.14 / 88.05   |
| `large-cjs` | global   | disabled  | 87.18 / 90.30   | 90.71 / 93.75   | 80.26 / 86.21   | 82.89 / 88.98   |
| `large-cjs` | global   | transform | 95.36 / 97.77   | 98.58 / 101.30  | 61.78 / 65.40   | 64.55 / 68.76   |
| `large-cjs` | global   | compile   | 85.96 / 86.66   | 91.41 / 93.81   | 66.66 / 72.53   | 69.42 / 75.54   |
| `large-cjs` | global   | combined  | 98.39 / 99.74   | 103.35 / 104.47 | 48.16 / 53.42   | 50.46 / 56.10   |
| `large-cjs` | local    | baseline  | 109.57 / 112.09 | 114.23 / 116.90 | 99.27 / 110.05  | 101.93 / 113.58 |
| `large-cjs` | local    | disabled  | 110.34 / 112.40 | 113.66 / 115.65 | 100.97 / 108.16 | 103.88 / 111.05 |
| `large-cjs` | local    | transform | 118.58 / 169.88 | 121.72 / 173.52 | 82.03 / 91.49   | 84.51 / 94.62   |
| `large-cjs` | local    | compile   | 110.63 / 112.97 | 115.33 / 120.05 | 87.43 / 97.41   | 90.01 / 100.27  |
| `large-cjs` | local    | combined  | 118.25 / 127.89 | 123.41 / 132.66 | 69.38 / 76.06   | 71.78 / 78.68   |
| `large-cjs` | direct   | baseline  | 79.05 / 79.44   | 82.12 / 82.77   | 73.82 / 79.09   | 76.66 / 82.87   |
| `large-cjs` | direct   | disabled  | 81.39 / 83.88   | 84.38 / 87.51   | 74.40 / 79.79   | 77.16 / 82.77   |
| `large-cjs` | direct   | transform | 87.38 / 89.29   | 90.45 / 93.04   | 56.13 / 61.76   | 59.22 / 64.52   |
| `large-cjs` | direct   | compile   | 82.85 / 83.25   | 87.92 / 88.40   | 60.72 / 65.87   | 63.52 / 69.03   |
| `large-cjs` | direct   | combined  | 89.12 / 91.21   | 94.28 / 96.91   | 42.88 / 47.08   | 45.27 / 49.66   |
| `large-esm` | global   | baseline  | 77.59 / 80.09   | 80.61 / 83.28   | 72.34 / 80.40   | 75.14 / 83.06   |
| `large-esm` | global   | disabled  | 77.18 / 77.80   | 80.49 / 81.23   | 73.62 / 78.59   | 76.33 / 82.10   |
| `large-esm` | global   | transform | 86.54 / 87.14   | 89.67 / 90.57   | 60.14 / 65.02   | 62.57 / 67.68   |
| `large-esm` | global   | compile   | 83.99 / 85.32   | 88.81 / 90.45   | 60.41 / 67.33   | 63.11 / 70.01   |
| `large-esm` | global   | combined  | 91.21 / 97.33   | 98.56 / 102.89  | 46.65 / 51.46   | 49.01 / 54.08   |
| `large-esm` | local    | baseline  | 103.03 / 105.86 | 106.21 / 108.66 | 93.17 / 101.33  | 95.92 / 104.08  |
| `large-esm` | local    | disabled  | 103.29 / 105.90 | 106.86 / 108.91 | 93.29 / 102.13  | 95.91 / 106.03  |
| `large-esm` | local    | transform | 111.87 / 114.20 | 114.91 / 117.28 | 80.89 / 90.70   | 83.41 / 93.76   |
| `large-esm` | local    | compile   | 109.86 / 111.85 | 114.95 / 116.44 | 81.51 / 89.82   | 84.21 / 92.96   |
| `large-esm` | local    | combined  | 116.40 / 133.95 | 121.73 / 140.71 | 68.81 / 76.47   | 71.22 / 79.08   |
| `large-esm` | direct   | baseline  | 71.83 / 73.43   | 75.21 / 76.24   | 66.57 / 71.68   | 69.17 / 75.22   |
| `large-esm` | direct   | disabled  | 71.68 / 73.15   | 75.01 / 76.23   | 67.27 / 72.72   | 70.09 / 75.70   |
| `large-esm` | direct   | transform | 81.75 / 85.85   | 85.07 / 89.15   | 54.46 / 58.45   | 56.87 / 62.32   |
| `large-esm` | direct   | compile   | 79.54 / 82.52   | 87.09 / 89.62   | 54.78 / 59.74   | 57.37 / 62.89   |
| `large-esm` | direct   | combined  | 85.45 / 86.61   | 90.18 / 93.74   | 41.87 / 46.59   | 44.29 / 49.34   |

</details>

<details>
<summary>Node.js 22.18.0 and 26.12.0 — global baseline and combined cache</summary>

| Node.js | Fixture     | Mode     | Cold ready      | Cold exit       | Warm ready      | Warm exit       |
| ------- | ----------- | -------- | --------------- | --------------- | --------------- | --------------- |
| 22.18.0 | `hello`     | baseline | 50.02 / 52.94   | 52.86 / 56.35   | 47.77 / 51.17   | 50.40 / 54.09   |
| 22.18.0 | `hello`     | combined | 49.77 / 53.80   | 53.49 / 57.92   | 48.40 / 54.06   | 51.02 / 56.75   |
| 22.18.0 | `esm-app`   | baseline | 90.95 / 98.11   | 94.83 / 102.41  | 90.00 / 101.37  | 93.70 / 105.29  |
| 22.18.0 | `esm-app`   | combined | 153.30 / 154.67 | 197.78 / 201.45 | 98.12 / 107.57  | 101.64 / 110.85 |
| 22.18.0 | `cjs-app`   | baseline | 90.09 / 95.83   | 93.82 / 99.57   | 89.50 / 95.82   | 93.15 / 99.09   |
| 22.18.0 | `cjs-app`   | combined | 146.89 / 152.72 | 190.20 / 197.16 | 97.08 / 105.71  | 100.48 / 109.02 |
| 22.18.0 | `large-cjs` | baseline | 115.26 / 118.68 | 120.54 / 124.32 | 113.02 / 120.55 | 118.47 / 126.19 |
| 22.18.0 | `large-cjs` | combined | 121.31 / 123.21 | 128.46 / 130.54 | 81.59 / 87.35   | 86.32 / 92.52   |
| 22.18.0 | `large-esm` | baseline | 101.63 / 107.78 | 105.29 / 113.05 | 97.76 / 101.74  | 101.27 / 105.39 |
| 22.18.0 | `large-esm` | combined | 108.58 / 110.04 | 113.94 / 115.57 | 72.09 / 75.96   | 75.63 / 79.50   |
| 26.12.0 | `hello`     | baseline | 34.28 / 35.98   | 35.96 / 38.69   | 34.63 / 36.21   | 36.55 / 38.03   |
| 26.12.0 | `hello`     | combined | 37.56 / 38.64   | 40.13 / 41.27   | 35.06 / 36.93   | 36.88 / 39.03   |
| 26.12.0 | `esm-app`   | baseline | 74.80 / 77.16   | 77.05 / 79.50   | 69.56 / 74.08   | 72.03 / 76.51   |
| 26.12.0 | `esm-app`   | combined | 144.04 / 154.28 | 184.29 / 194.17 | 80.30 / 88.06   | 82.57 / 90.56   |
| 26.12.0 | `cjs-app`   | baseline | 71.67 / 77.53   | 74.11 / 80.02   | 70.14 / 77.05   | 72.46 / 79.27   |
| 26.12.0 | `cjs-app`   | combined | 137.89 / 146.02 | 178.54 / 186.20 | 80.77 / 87.07   | 82.96 / 89.22   |
| 26.12.0 | `large-cjs` | baseline | 77.64 / 83.38   | 80.21 / 86.84   | 77.36 / 82.83   | 80.08 / 85.50   |
| 26.12.0 | `large-cjs` | combined | 86.00 / 89.56   | 89.77 / 93.43   | 46.81 / 51.54   | 49.16 / 53.98   |
| 26.12.0 | `large-esm` | baseline | 69.24 / 74.46   | 71.92 / 77.07   | 69.85 / 73.13   | 72.49 / 75.85   |
| 26.12.0 | `large-esm` | combined | 82.36 / 85.87   | 86.39 / 92.17   | 45.90 / 48.04   | 48.35 / 50.55   |

</details>
