# vite-plus Performance Analysis

Performance measurements from E2E tests (Ubuntu, GitHub Actions runner).

**Test projects**: 9 ecosystem-ci projects (single-package and multi-package monorepos)
**Node.js**: 22-24 (managed by vite-plus js_runtime)
**Trace sources**:

- Run #22556278251 — baseline traces (2 runs per project, cache disabled)
- Run [#22558467033](https://github.com/voidzero-dev/vite-plus/actions/runs/22558467033) — cache-enabled traces (3 runs per project: first, cache hit, cache miss)

## Architecture Overview

A `vp run` command invocation traverses these layers:

```
User runs `vp run lint:check`
  |
  +- [Phase 1] Global CLI (Rust binary `vp`)                ~3-9ms
  |     +- argv0 processing                                  ~40us
  |     +- Node.js runtime resolution                        ~1.3ms
  |     +- Module resolution (oxc_resolver)                  ~170us
  |     +- Delegates to local CLI via exec(node bin.js)
  |
  +- [Phase 2] Node.js startup + NAPI loading                ~3.7ms
  |     +- bin.ts entry -> import NAPI binding -> call run()
  |
  +- [Phase 3] Rust core via NAPI (vite-task session)
  |     +- Session init                                      ~60-80us
  |     +- plan_from_cli_run_resolved
  |     |     +- plan_query
  |     |           +- load_task_graph
  |     |           |     +- load_package_graph               ~2-5ms
  |     |           |     +- load_user_config_file x N        ~170ms-1.3s (BOTTLENECK)
  |     |           +- handle_command (JS callback)           ~0.02-1.5ms
  |     +- execute_graph
  |           +- load_from_path (cache state)                 ~0.7-14ms
  |           +- execute_expanded_graph
  |                 +- execute_leaf -> execute_spawn
  |                       +- try_hit (cache lookup)           0-50ms
  |                       +- [hit]  validate + replay stdout
  |                       +- [miss] spawn_with_tracking       actual command runs
  |                       +- [miss] create_post_run_fingerprint + update
  |
  +- [Phase 4] Child process execution                       varies
```

## Execution Cache Performance

With `cacheScripts: true`, vite-task caches command outputs keyed by a spawn fingerprint (cwd + program + args + env) and validated by a post-run fingerprint (xxHash3_64 of all files accessed during execution, tracked by fspy).

### Cache Hit Savings (Per-Command)

When cache hits occur, the saved time comes from skipping `spawn_with_tracking` (the actual command execution) and `create_post_run_fingerprint` (post-run file hashing):

| Project                   | Command                        | Miss (ms) | Hit (ms) | Saved (ms) | Saved %   |
| ------------------------- | ------------------------------ | --------- | -------- | ---------- | --------- |
| dify                      | build (next build)             | 170,673   | 670      | 170,003    | **99.6%** |
| vitepress                 | tests-e2e#test                 | 26,696    | 250      | 26,446     | **99.1%** |
| vitepress                 | tests-init#test                | 11,430    | 290      | 11,140     | **97.5%** |
| vue-mini                  | test -- --coverage             | 6,357     | 217      | 6,140      | **96.6%** |
| dify                      | test (3 files)                 | 6,524     | 349      | 6,175      | **94.7%** |
| oxlint-plugin-complexity  | lint                           | 4,165     | 232      | 3,933      | **94.4%** |
| frm-stack                 | @yourcompany/api#test          | 14,760    | 895      | 13,865     | **93.9%** |
| oxlint-plugin-complexity  | build                          | 3,529     | 219      | 3,310      | **93.8%** |
| rollipop                  | -r typecheck (4 tasks)         | 8,581     | 697      | 7,884      | **91.9%** |
| vite-vue-vercel           | test                           | 2,744     | 326      | 2,418      | **88.1%** |
| oxlint-plugin-complexity  | test:run                       | 1,377     | 212      | 1,165      | **84.6%** |
| tanstack-start-helloworld | build                          | 8,844     | 1,383    | 7,461      | **84.4%** |
| oxlint-plugin-complexity  | format:check                   | 1,355     | 214      | 1,141      | **84.2%** |
| frm-stack                 | @yourcompany/backend-core#test | 5,571     | 894      | 4,677      | **83.9%** |
| oxlint-plugin-complexity  | format                         | 1,419     | 239      | 1,180      | **83.2%** |
| rollipop                  | @rollipop/core#test            | 2,878     | 671      | 2,208      | **76.7%** |
| vite-vue-vercel           | build                          | 842       | 328      | 514        | **61.0%** |
| rollipop                  | @rollipop/common#test          | 1,307     | 663      | 644        | **49.3%** |
| rollipop                  | format                         | 1,257     | 657      | 600        | **47.7%** |
| frm-stack                 | typecheck                      | 1,448     | 918      | 530        | **36.6%** |

### Cache Operation Overhead

#### On Cache Hit

| Operation                       | Time        | Description                                                             |
| ------------------------------- | ----------- | ----------------------------------------------------------------------- |
| `try_hit`                       | 0.0–50ms    | Look up spawn fingerprint in SQLite, then validate post-run fingerprint |
| `validate_post_run_fingerprint` | 1–40ms      | Re-hash all tracked input files to check if they changed                |
| **Total cache overhead**        | **10–50ms** | Negligible compared to saved execution time                             |

Cache hit total time is dominated by config loading (177–1,316ms depending on project), not cache operations.

#### On Cache Miss (with write-back)

| Operation                     | Time          | Description                                               |
| ----------------------------- | ------------- | --------------------------------------------------------- |
| `try_hit`                     | 0.0–0.1ms     | Quick lookup, returns `NotFound` or `FingerprintMismatch` |
| `spawn_with_tracking`         | 200–170,000ms | Execute the actual command with fspy file tracking        |
| `create_post_run_fingerprint` | 2–1,637ms     | Hash all files accessed during execution                  |
| `update`                      | 1–200ms       | Write fingerprint and outputs to SQLite cache             |

### Execution Timeline (Cache Hit vs Miss)

#### Cache Hit Flow

```
┌──────────────────┐  ┌─────────┐  ┌──────────────────────────────┐  ┌─────────┐
│ load_user_config │→│ try_hit │→│ validate_post_run_fingerprint │→│ replay  │
│   177–1316ms     │  │  <1ms   │  │          1–40ms              │  │ stdout  │
└──────────────────┘  └─────────┘  └──────────────────────────────┘  └─────────┘
Total: 200–1400ms (config loading dominates)
```

#### Cache Miss Flow

```
┌──────────────────┐  ┌─────────┐  ┌─────────────────────┐  ┌────────────────────────────┐  ┌────────┐
│ load_user_config │→│ try_hit │→│ spawn_with_tracking  │→│ create_post_run_fingerprint │→│ update │
│   177–1316ms     │  │  <1ms   │  │   200–170,000ms     │  │        2–1637ms             │  │ 1–200ms│
└──────────────────┘  └─────────┘  └─────────────────────┘  └────────────────────────────┘  └────────┘
Total: 400–172,000ms (spawn dominates)
```

### Cache Miss Root Causes

From CI log analysis, cache misses on the "cache hit" run fall into these categories:

| Miss Reason                                | Count | Explanation                                                                                   |
| ------------------------------------------ | ----- | --------------------------------------------------------------------------------------------- |
| `content of input 'package.json' changed`  | 60    | Expected — from the intentional cache invalidation step                                       |
| `content of input '' changed`              | 9     | Bug — fspy tracks an empty path (working directory listing) which changes between runs        |
| `content of input 'dist/...' changed`      | ~10   | Expected — build outputs change between runs (e.g., vitepress `build:client` changes `dist/`) |
| `content of input 'tsconfig.json' changed` | 3     | Side effect of prior commands modifying project config                                        |

The `content of input '' changed` issue affects vue-mini's `prettier`, `eslint`, and `tsc` commands — fspy records the working directory itself as a read, and its directory listing changes between runs because the first command creates or modifies files. This is the main reason vue-mini and rollipop show low cache hit rates.

## Cross-Project Comparison

NAPI overhead measured from trace files (Ubuntu, all invocations):

| Project                   | Packages | Config loading  | Overhead        | n   |
| ------------------------- | -------- | --------------- | --------------- | --- |
| vue-mini                  | 1        | **170-218ms**   | **173-223ms**   | 8   |
| oxlint-plugin-complexity  | 1-2      | **177-249ms**   | **184-258ms**   | 10  |
| vitepress                 | 4        | **175-202ms**   | **182-327ms**   | 12  |
| vite-vue-vercel           | 1        | **320-328ms**   | **326-338ms**   | 4   |
| rollipop                  | 6        | **635-658ms**   | **643-670ms**   | 14  |
| frm-stack                 | 10-11    | **959-993ms**   | **968-1002ms**  | 10  |
| tanstack-start-helloworld | 1        | **1305-1320ms** | **1308-1337ms** | 4   |
| vibe-dashboard            | --       | --              | --              | 0   |
| dify                      | --       | --              | --              | 0   |

vibe-dashboard and dify only produced global CLI traces (no NAPI traces captured). See Known Issues.

Config loading accounts for **95-99%** of total NAPI overhead in every project.

### Config Loading Patterns

The first `load_user_config_file` call pays a fixed JS module initialization cost (~150-170ms). Projects with heavy Vite plugins pay more:

| Project                   | First config    | Largest config       | Subsequent configs |
| ------------------------- | --------------- | -------------------- | ------------------ |
| vue-mini                  | 164-177ms       | same                 | 2-3ms              |
| oxlint-plugin-complexity  | 177-249ms       | same                 | N/A (single)       |
| vitepress                 | 158-201ms       | same                 | 5-7ms              |
| vite-vue-vercel           | 320-328ms       | same                 | N/A (single)       |
| rollipop                  | 150-165ms       | 146-168ms (#3)       | 100-155ms each     |
| frm-stack                 | 165-173ms       | **750-786ms** (#4-5) | 3-12ms             |
| tanstack-start-helloworld | **1305-1320ms** | same                 | N/A (single)       |

Key observations:

- **tanstack-start-helloworld** has the slowest single config load (1.3s) despite being a single-package project. Entirely due to heavy TanStack/Vinxi plugin dependencies.
- **frm-stack** has one "monster" config at ~750-786ms (a specific workspace package with heavy plugins), accounting for ~77% of total config loading.
- **rollipop** is unusual: subsequent config loads remain expensive (100-155ms) rather than dropping to 2-12ms, suggesting each package imports distinct heavy dependencies.
- Simple projects (vue-mini, vitepress) have a consistent ~165ms first-config cost, representing the baseline JS module initialization overhead.

## Phase 1: Global CLI (Rust binary)

Measured via Chrome tracing from the `vp` binary process.

### Cross-Project Global CLI Overhead

| Project                   | Range      | n   |
| ------------------------- | ---------- | --- |
| vite-vue-vercel           | 3.4-6.9ms  | 10  |
| rollipop                  | 3.7-4.7ms  | 14  |
| tanstack-start-helloworld | 3.7-6.2ms  | 4   |
| vitepress                 | 3.3-3.9ms  | 12  |
| vibe-dashboard            | 4.1-6.7ms  | 6   |
| vue-mini                  | 5.5-6.1ms  | 8   |
| oxlint-plugin-complexity  | 3.1-8.8ms  | 10  |
| dify                      | 4.3-13.6ms | 6   |
| frm-stack                 | 3.4-7.4ms  | 10  |

Global CLI overhead is consistently **3-9ms** across all projects, with rare outliers up to 14ms. This is the Rust binary resolving Node.js version, finding the local vite-plus install via oxc_resolver, and delegating via exec.

### Breakdown (vibe-dashboard, 6 invocations)

| Stage                    | Time from start | Duration   |
| ------------------------ | --------------- | ---------- |
| argv0 processing         | 37-57us         | ~40us      |
| Runtime resolution start | 482-684us       | ~500us     |
| Node.js version selected | 714-1042us      | ~300us     |
| Node.js version resolved | 1237-1593us     | ~50us      |
| Node.js cache confirmed  | 1302-1627us     | ~50us      |
| **oxc_resolver start**   | **3058-7896us** | --         |
| oxc_resolver complete    | 3230-8072us     | **~170us** |
| Delegation to Node.js    | 3275-8160us     | ~40us      |

## Phase 2: Node.js Startup + NAPI Loading

Measured from NAPI-side Chrome traces.

The NAPI `run()` function is first called at **~3.7ms** from Node.js process start:

| Event                   | Time (us) | Notes                              |
| ----------------------- | --------- | ---------------------------------- |
| NAPI `run()` entered    | ~3,700    | First trace event from NAPI module |
| `napi_run: start`       | ~3,950    | After ThreadsafeFunction setup     |
| `cli::main` span begins | ~4,100    | CLI argument processing starts     |

Node.js startup + ES module loading + NAPI binding initialization takes **~3.7ms**.

## Phase 3: Rust Core via NAPI (vite-task)

### Detailed Timeline (frm-stack `vp run lint:check`, first run)

From Chrome trace, all times in us from process start:

```
  ~3,700   NAPI run() entered
  ~3,950   napi_run: start
   4,462   cli::main begins
           execute_vite_task_command begins
   4,462     session::init                                    --  80us
   4,552     plan_from_cli_run_resolved begins
               plan_query begins
                 load_task_graph begins
   4,569           load_package_graph                         --  4.3ms
   8,878           load_user_config_file x10                  -- 983ms total
                     #1: 165ms (cold JS init)
                     #2: 12ms
                     #3: 4ms
                     #4: 776ms (monster config)
                     #5-#10: 3-5ms each
 992,988        handle_command                                --  0.04ms
 993,336     execute_graph begins
 993,385       load_from_path (cache state)                   --  7.4ms
1,000,873     execute_expanded_graph begins
1,001,667       execute_spawn begins
                  try_hit → spawn_with_tracking               -- command runs here
```

**Total overhead before task execution: ~1001ms**, of which **983ms (98%) is vite.config.ts loading**.

### frm-stack Per-Command Breakdown (10 traces, all values in ms)

| Command                          | Run | PkgGr | 1st Cfg | Total Cfg | Cfgs | Overhead | CacheLoad | hdl_cmd |
| -------------------------------- | --- | ----- | ------- | --------- | ---- | -------- | --------- | ------- |
| `lint:check`                     | 1st | 4.3   | 165     | 983       | 10   | 1002     | 7.4       | 0.04    |
| `format:check`                   | 1st | 4.1   | 172     | 964       | 10   | 972      | 0.8       | 0.00    |
| `typecheck`                      | 1st | 4.4   | 169     | 964       | 10   | 971      | 0.8       | 0.06    |
| `@yourcompany/api#test`          | 1st | 4.8   | 173     | 986       | 11   | 996      | 0.8       | 1.53    |
| `@yourcompany/backend-core#test` | 1st | 4.8   | 173     | 990       | 11   | 1001     | 1.3       | 1.42    |
| `lint:check`                     | 2nd | 4.7   | 169     | 990       | 11   | 1001     | 0.8       | 0.03    |
| `format:check`                   | 2nd | 4.3   | 167     | 961       | 11   | 969      | 0.8       | 0.08    |
| `typecheck`                      | 2nd | 4.5   | 165     | 993       | 11   | 1000     | 0.8       | 0.00    |
| `@yourcompany/api#test`          | 2nd | 4.7   | 166     | 959       | 11   | 969      | 1.4       | 1.51    |
| `@yourcompany/backend-core#test` | 2nd | 4.9   | 168     | 980       | 11   | 990      | 1.1       | 1.41    |

### frm-stack Aggregate Statistics

| Metric                               | Average | Range       | n   |
| ------------------------------------ | ------- | ----------- | --- |
| load_package_graph                   | 4.5ms   | 4.1-4.9ms   | 10  |
| Total config loading per command     | 977ms   | 959-993ms   | 10  |
| First config load                    | 169ms   | 165-173ms   | 10  |
| "Monster" config load (~config #4/5) | 763ms   | 750-786ms   | 10  |
| Other config loads                   | ~4ms    | 3-12ms      | ~90 |
| Total NAPI overhead                  | 987ms   | 968-1002ms  | 10  |
| Cache state load (load_from_path)    | 1.5ms   | 0.8-7.4ms   | 10  |
| handle_command (non-test)            | 0.03ms  | 0.00-0.08ms | 6   |
| handle_command (test w/ js_resolver) | 1.46ms  | 1.41-1.53ms | 4   |

### First Run vs Second Run (frm-stack averages)

| Metric               | First Run | Second Run | Delta         |
| -------------------- | --------- | ---------- | ------------- |
| Total NAPI overhead  | 988ms     | 985ms      | -3ms (-0.3%)  |
| load_package_graph   | 4.5ms     | 4.6ms      | +0.1ms        |
| Total config loading | 977ms     | 977ms      | ~0ms          |
| First config load    | 170ms     | 167ms      | -3ms          |
| Monster config       | 763ms     | 763ms      | ~0ms          |
| Cache state load     | 2.2ms     | 1.0ms      | -1.2ms (-55%) |

Config loading is **not cached** between invocations -- every `vp run` command re-resolves all Vite configs from JavaScript. There is no measurable difference between first and second runs.

### Callback Timing (`handle_command` + `js_resolver`)

After the task graph is loaded, vite-task calls back into JavaScript to resolve the tool binary:

```
 996,446   handle_command begins
 996,710     resolve begins
               js_resolver begins (test command)
 997,880       js_resolver ends                              -- 1.17ms
 998,040     resolve ends
 998,126   handle_command ends                               -- 1.53ms
```

The `js_resolver` callback (which locates the test runner binary via JavaScript) takes **~1.1ms**. Non-test commands (lint, fmt, typecheck) skip this callback and resolve directly, taking only ~0.03ms.

### rollipop: Multi-Spawn Execution

Some commands spawn multiple child processes sequentially (topological order from `dependsOn`):

```
rollipop `vp run -r build` (first run):
  ~668us    execute_expanded_graph begins
  ~678us      execute_leaf #1: spawn_inherited (1898ms)    -- @rollipop/common#build
  2,576us     execute_leaf #2: spawn_inherited (2668ms)    -- @rollipop/core#build
  5,244us     execute_leaf #3: spawn_inherited (2138ms)    -- @rollipop/rollipop#build
  7,382us     execute_leaf #4: spawn_inherited (1859ms)    -- @rollipop/dev-server#build
  Total spawn time: 8563ms (sequential due to dependsOn)
```

### vitepress: Build Pipeline

The `vp run build` command spawns 3 sequential phases:

```
vitepress `vp run build` (first run):
  ~185us    execute_expanded_graph begins
  ~185us      spawn_inherited #1: pnpm build:prepare        -- 466ms
  ~651us      spawn_inherited #2: pnpm build:client          -- 8362ms
  9,013us     spawn_inherited #3: pnpm build:node            -- 10312ms
  Total: 19.1s (sequential pipeline)
```

## Phase 4: Child Process Execution

Wall-clock timestamps from CI output logs. The `process uptime` value shows Node.js startup time (consistent ~33-55ms across all projects).

### Process Uptime (Node.js startup)

| Project                   | Range       |
| ------------------------- | ----------- |
| vibe-dashboard            | 35.0-35.1ms |
| rollipop                  | 32.4-37.8ms |
| frm-stack                 | 34.2-56.2ms |
| vue-mini                  | 38.6-54.8ms |
| vitepress                 | 32.4-35.9ms |
| tanstack-start-helloworld | 33.2-33.9ms |
| oxlint-plugin-complexity  | 33.0-47.2ms |
| vite-vue-vercel           | 32.1-33.2ms |
| dify                      | 33.8-40.1ms |

Node.js startup is consistently **32-55ms** across all projects.

## Key Findings

### 1. Cache hits save 50–99% of execution time

When cache hits occur, they are highly effective. The remaining time is almost entirely config loading (`load_user_config_file`), which must run every time regardless of cache status.

### 2. Config loading is the dominant bottleneck

Config loading accounts for **95-99%** of NAPI overhead and sets the floor for cache hit response time:

- Small projects (vue-mini, oxlint): ~180ms
- Medium projects (rollipop, vitepress): ~230–640ms
- Large projects (frm-stack): ~850ms
- Complex projects (tanstack-start, dify): ~1,300ms

Config loading is not cached between `vp` invocations — every command re-resolves all configs from JavaScript.

### 3. Cache fingerprinting overhead is negligible

`create_post_run_fingerprint` (2–60ms per task for most projects) and `validate_post_run_fingerprint` (1–40ms) add minimal overhead. The exception is dify where fingerprinting takes 170–1,637ms due to the large number of files tracked.

### 4. Within-run deduplication works

vitepress runs `VITE_TEST_BUILD=1 vp run tests-e2e#test` which is identical to the prior `vp run tests-e2e#test`. The second invocation is always a cache hit (even on the first run), saving ~26s each time.

### 5. Empty-path fingerprinting reduces cache hit rate

Commands whose child processes read the working directory (path `''`) get a volatile directory-listing fingerprint that changes between runs. This affects `prettier`, `eslint`, and `tsc` in vue-mini and `lint` in rollipop, dropping their overall cache hit speedup to 1.2–1.5x.

## Summary of Bottlenecks

| Bottleneck                    | Time                       | % of overhead |
| ----------------------------- | -------------------------- | ------------- |
| vite.config.ts loading (cold) | **170ms-1.3s** per command | **95-99%**    |
| load_package_graph            | **2-5ms**                  | <1%           |
| Cache state load              | **0.7-14ms**               | <1%           |
| Cache operations (hit)        | **10-50ms**                | <5%           |
| handle_command (js_resolver)  | **~1.5ms**                 | <0.2%         |
| Session init                  | **~70us**                  | <0.01%        |
| Node.js + NAPI startup        | **~3.7ms**                 | <0.4%         |
| Global CLI overhead           | **3-9ms**                  | <0.5%         |
| oxc_resolver                  | **~170us**                 | <0.02%        |

Config loading breakdown across projects:

- Simple configs (vue-mini, vitepress): ~170ms baseline, nearly all from first-config JS initialization
- Heavy single configs (tanstack-start-helloworld): up to 1.3s for a single config with heavy plugins
- Large monorepos (frm-stack, 10 packages): ~977ms total, dominated by one "monster" config (~763ms)
- Distinct-dependency monorepos (rollipop, 6 packages): ~644ms, each package importing different heavy dependencies (100-155ms each)

## Known Issues

### vibe-dashboard and dify produce no NAPI traces

These projects produce only global CLI traces. The NAPI-side tracing likely doesn't flush properly because:

- `vp fmt` and `vp test` (Synthesizable commands) may exit before `shutdownTracing()` is called
- The `shutdownTracing()` fix (commit `72b23304`) may not cover all exit paths for these command types

### Empty-path fingerprinting causes spurious cache misses

fspy tracks the working directory itself (path `''`) as a file read. The directory listing fingerprint changes between runs when prior commands create or modify files, causing `PostRunFingerprintMismatch`. This affects 9 commands across vue-mini and rollipop (`prettier`, `eslint`, `tsc`, `lint`).

### Trace files break formatter (fixed)

When `VITE_LOG_OUTPUT=chrome-json` is set, trace files were written to the project working directory. Formatters pick up these files and fail with parse errors.

**Fix**: Set `VITE_LOG_OUTPUT_DIR` to write trace files to a dedicated directory outside the workspace.

## Tracing Instrumentation

The following spans are instrumented at `debug` level in vite-task:

| Span                            | Location                         | Purpose                                      |
| ------------------------------- | -------------------------------- | -------------------------------------------- |
| `try_hit`                       | `session/cache/mod.rs`           | Cache lookup with spawn fingerprint matching |
| `validate_post_run_fingerprint` | `session/execute/fingerprint.rs` | Re-hash tracked files to validate cache      |
| `create_post_run_fingerprint`   | `session/execute/fingerprint.rs` | Hash all fspy-tracked files after execution  |
| `update`                        | `session/cache/mod.rs`           | Write cache entry to SQLite                  |
| `spawn_with_tracking`           | `session/execute/spawn.rs`       | Execute command with fspy file tracking      |
| `load_from_path`                | `session/cache/mod.rs`           | Open/create SQLite cache database            |
| `execute_spawn`                 | `session/execute/mod.rs`         | Full cache-aware execution lifecycle         |

Enabled via: `VITE_LOG=debug VITE_LOG_OUTPUT=chrome-json VITE_LOG_OUTPUT_DIR=<path>`

## Methodology

- **Tracing**: Rust `tracing` crate with `tracing-chrome` subscriber (Chrome DevTools JSON format)
- **Environment variables**: `VITE_LOG=debug`, `VITE_LOG_OUTPUT=chrome-json`, `VITE_LOG_OUTPUT_DIR=<dir>`
- **CI environment**: GitHub Actions ubuntu-latest runner
- **Measurement PRs**:
  - vite-task: https://github.com/voidzero-dev/vite-task/pull/178
  - vite-plus: https://github.com/voidzero-dev/vite-plus/pull/663
- **E2E tests**:
  - Run #22556278251 — 2 runs per project, cache disabled. Baseline overhead measurements.
  - Run #22558467033 — 3 runs per project (first, cache hit, cache miss). Cache performance measurements.
- **Analysis tools**:
  - `analyze2.py` — Parses Chrome trace JSON files, classifies cache behavior, extracts per-span timings
  - Trace artifacts: `run2-artifacts/trace-{project}-ubuntu-latest/`
  - Full CI log: `run2-full.log`
