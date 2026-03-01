# vite-plus Performance Analysis

Performance measurements from E2E tests (Ubuntu, GitHub Actions runner).

**Test projects**: vibe-dashboard (single-package), frm-stack (multi-package monorepo)
**Node.js**: 24.14.0 (managed by vite-plus js_runtime)
**Commands tested**: `vp fmt`, `vp test`, `vp run build`, `vp run lint:check`, `vp run @yourcompany/api#test`

## Architecture Overview

A `vp` command invocation traverses multiple layers:

```
User runs `vp run lint:check`
  │
  ├─ [1] Global CLI (Rust binary `vp`)                    ~3-8ms
  │     ├─ argv0 processing                                ~40μs
  │     ├─ Node.js runtime resolution                       ~1.3ms
  │     ├─ Module resolution (oxc_resolver)                  ~170μs
  │     └─ Delegates to local CLI via exec(node bin.js)
  │
  ├─ [2] Node.js startup + NAPI loading                    ~3.7ms
  │     └─ bin.ts entry → import NAPI binding → call run()
  │
  ├─ [3] Rust core via NAPI (vite-task session)
  │     ├─ Session init                                     ~60μs
  │     ├─ load_package_graph (workspace discovery)          ~4ms
  │     ├─ load_user_config_file × N (JS callbacks)          ~160ms first, ~3-12ms subsequent
  │     ├─ handle_command + resolve (JS callbacks)            ~1.3ms
  │     └─ Task execution (spawns child processes)
  │
  └─ [4] Task spawns (child processes)
        ├─ Spawn 1: pnpm install / dependsOn                ~0.95-1.05s
        └─ Spawn 2: actual command                           varies (1-6s)
```

## Phase 1: Global CLI (Rust binary)

Measured via Chrome tracing from the `vp` binary process.
Timestamps are relative to process start (microseconds).

### Breakdown (6 invocations, vibe-dashboard, Ubuntu)

| Stage | Time from start | Duration |
|---|---|---|
| argv0 processing | 37-57μs | ~40μs |
| Runtime resolution start | 482-684μs | ~500μs |
| Node.js version selected | 714-1042μs | ~300μs |
| LTS alias resolved | 723-1075μs | ~10μs |
| Version index cache check | 1181-1541μs | ~400μs |
| Node.js version resolved | 1237-1593μs | ~50μs |
| Node.js cache confirmed | 1302-1627μs | ~50μs |
| **oxc_resolver start** | **3058-7896μs** | — |
| oxc_resolver complete | 3230-8072μs | **~170μs** |
| Delegation to Node.js | 3275-8160μs | ~40μs |

### Key Observations

- **Total global CLI overhead**: 3.3ms - 8.2ms per invocation
- **oxc_resolver** is extremely fast (~170μs), resolving `vite-plus/package.json` via node_modules
- **Dominant variable cost**: Gap between "Node cached" and "oxc_resolver start" (1.7-6.6ms). This includes CLI argument parsing, command dispatch, and resolver initialization
- **Node.js runtime resolution** consistently uses cached version index and cached Node.js binary (~1.3ms)

## Phase 2: Node.js Startup + NAPI Loading

Measured from NAPI-side Chrome traces (frm-stack project).

The NAPI `run()` function is first called at **~3.7ms** from Node.js process start:

| Event | Time (μs) | Notes |
|---|---|---|
| NAPI `run()` entered | 3,682 | First trace event from NAPI module |
| `napi_run: start` | 3,950 | After ThreadsafeFunction setup |
| `cli::main` span begins | 4,116 | CLI argument processing starts |

This means **Node.js startup + ES module loading + NAPI binding initialization takes ~3.7ms**.

## Phase 3: Rust Core via NAPI (vite-task)

### NAPI-side Detailed Breakdown (frm-stack `vp run lint:check`)

From Chrome trace, all times in μs from process start:

```
  3,682   NAPI run() entered
  3,950   napi_run: start
  4,116   cli::main begins
  4,742   execute_vite_task_command begins
  4,865     session::init begins
  4,907       init_with begins
  4,923       init_with ends                              ──  16μs
  4,924     session::init ends                            ──  59μs
  4,925     session::main begins
  4,931       plan_from_cli_run_resolved begins
  4,935         plan_query begins
  4,941           load_task_graph begins
  4,943             task_graph::load begins
  4,944               load_package_graph begins           ━━ 3.8ms
  8,764               load_package_graph ends
  8,779           load_user_config_file #1 begins         ━━ 164ms (first vite.config.ts load)
173,248           load_user_config_file #1 ends
173,265           load_user_config_file #2 begins         ━━ 12ms
185,212           load_user_config_file #2 ends
185,221           load_user_config_file #3 begins         ━━ 3.4ms
188,666           load_user_config_file #3 ends
188,675           load_user_config_file #4 begins         ━━ 741ms (cold import of workspace package config)
929,476           load_user_config_file #4 ends
  ...     (subsequent loads: ~3-5ms each)
```

### Critical Finding: vite.config.ts Loading is the Bottleneck

The **`load_user_config_file`** callback (which calls back into JavaScript to load `vite.config.ts` for each workspace package) dominates the task graph loading time:

| Config Load | Duration | Notes |
|---|---|---|
| First package | **164ms** | Cold import: requires JS module resolution + transpilation |
| Second package | **12ms** | Warm: shared dependencies already cached |
| Third package | **3.4ms** | Warm: nearly all deps cached |
| Fourth package (different deps) | **741ms** | Cold: imports new heavy dependencies |
| Subsequent packages | **3-5ms** each | All warm |

**For frm-stack (10 packages), total config loading: ~930ms** — this is the single largest cost.

### Callback Timing (`handle_command` + `resolve`)

After the task graph is loaded, vite-task calls back into JavaScript to resolve the tool binary:

```
937,757   handle_command begins
937,868     resolve begins
937,873       js_resolver begins (test command)
939,126       js_resolver ends                            ── 1.25ms
939,187     resolve ends
939,189   handle_command ends                             ── 1.43ms
```

The `js_resolver` callback (which locates the test runner binary via JavaScript) takes **~1.25ms**.

### Session Init Timing Comparison

| Stage | frm-stack (10 packages) | Notes |
|---|---|---|
| Session init | ~60μs | Minimal setup |
| load_package_graph | ~4ms | Workspace discovery |
| load_user_config_file (all) | **~930ms** | JS callbacks, dominant cost |
| handle_command + resolve | ~1.4ms | Tool binary resolution |
| **Total before task execution** | **~936ms** | |

## Phase 4: Task Execution (vibe-dashboard)

### Spawn Timing (First Run — Cold)

| Command | Spawn 1 (setup) | Spawn 2 (execution) | Total |
|---|---|---|---|
| `vp fmt` | 1.05s (977 reads, 50 writes) | 1.00s (163 reads, 1 write) | ~2.1s |
| `vp test` | 0.96s (977 reads, 50 writes) | 5.71s (4699 reads, 26 writes) | ~6.7s |
| `vp run build` | 0.95s (977 reads, 50 writes) | 1.61s (3753 reads, 17 writes) | ~2.6s |

### Spawn Timing (Second Run — Cache Available)

| Command | Spawn 1 (setup) | Spawn 2 (execution) | Total | Delta |
|---|---|---|---|---|
| `vp fmt` | 0.95s (977 reads, 50 writes) | 0.97s (167 reads, 3 writes) | ~1.9s | -0.2s |
| `vp test` | 0.95s (977 reads, 50 writes) | 4.17s (1930 reads, 4 writes) | ~5.1s | **-1.6s** |
| `vp run build` | 0.96s (977 reads, 50 writes) | **cache hit (replayed)** | ~1.0s | **-1.6s** |

### Key Observations

- **Spawn 1 is constant** (~0.95-1.05s, 977 path_reads, 50 path_writes) regardless of command or cache state. This is the workspace/task-graph loading + pnpm resolution overhead.
- **`vp run build` cache hit**: On second run, the build was fully replayed from cache, saving 1.19s. The 977-read spawn 1 still executes.
- **`vp test` improvement**: Second run read 1930 paths (vs 4699), suggesting OS filesystem caching reduced disk I/O.

## Phase 5: Task Cache Effectiveness

vite-task implements a file-system-aware task cache at `node_modules/.vite/task-cache`.

| Command | First Run | Cache Run | Cache Hit? | Savings |
|---|---|---|---|---|
| `vp fmt` | 2.1s | 1.9s | No | — |
| `vp test` | 6.7s | 5.1s | No | -1.6s (OS cache) |
| `vp run build` | 2.6s | 1.0s | **Yes** | **-1.6s** (1.19s from task cache) |

**Only `vp run build` was cache-eligible.** Formatting and test commands are not cached (side effects / non-deterministic outputs).

## End-to-End Timeline: Full Command Lifecycle

Combining all phases for a single `vp run lint:check` invocation (frm-stack):

```
T+0.00ms    Global CLI starts (Rust binary)
T+0.04ms    argv0 processed
T+0.50ms    Runtime resolution begins
T+1.30ms    Node.js version resolved (cached)
T+3.30ms    oxc_resolver finds local vite-plus              ── ~170μs
T+3.35ms    exec(node, [dist/bin.js, "run", "lint:check"])   ── process replaced
─── Node.js process starts ───
T+3.70ms    NAPI run() called (Node.js startup overhead)
T+4.00ms    napi_run: start
T+4.12ms    cli::main begins
T+4.74ms    execute_vite_task_command begins
T+4.94ms    load_package_graph begins
T+8.76ms    load_package_graph ends                          ── 3.8ms
T+8.78ms    load_user_config_file #1 begins (JS callback)
T+173ms     load_user_config_file #1 ends                    ── 164ms ★ bottleneck
  ...       (more config loads)
T+937ms     handle_command begins
T+939ms     handle_command ends (js_resolver: 1.25ms)
T+940ms     Task execution starts (child process spawn)
  ...       (actual command runs)
```

**Total overhead before task execution: ~940ms**, of which **~930ms (99%) is vite.config.ts loading**.

## Wall-Clock Timelines (vibe-dashboard, Ubuntu)

### First Run

```
19:16:44.039  vp fmt    — pnpm download starts
19:16:44.170  vp fmt    — cache dir created
19:16:45.158  vp fmt    — spawn 1 finished (setup)
19:16:46.028  vp fmt    — spawn 2 finished (biome)           Total: ~2.0s
19:16:46.082  vp test   — pnpm resolution starts
19:16:46.084  vp test   — cache dir created
19:16:47.057  vp test   — spawn 1 finished (setup)
19:16:52.750  vp test   — spawn 2 finished (vitest)          Total: ~6.7s
19:16:52.846  vp run build — cache dir created
19:16:53.793  vp run build — spawn 1 finished (setup)
19:16:55.398  vp run build — spawn 2 finished (vite build)   Total: ~2.6s
```

**Total first run: ~11.4s** (3 commands sequential)

### Cache Run

```
19:16:56.446  vp fmt    — cache dir created
19:16:57.399  vp fmt    — spawn 1 finished
19:16:58.368  vp fmt    — spawn 2 finished                   Total: ~1.9s
19:16:58.441  vp test   — cache dir created
19:16:59.390  vp test   — spawn 1 finished
19:17:03.556  vp test   — spawn 2 finished                   Total: ~5.1s
19:17:03.641  vp run build — cache dir created
19:17:04.596  vp run build — spawn 1 finished
19:17:05.040  vp run build — cache replayed                  Total: ~1.4s
```

**Total cache run: ~8.6s** (-24% from first run)

## Summary of Bottlenecks

| Bottleneck | Time | % of overhead | Optimization opportunity |
|---|---|---|---|
| vite.config.ts loading (cold) | **164-741ms** per package | **99%** | Cache config results, lazy loading, parallel loading |
| Spawn 1 (pnpm/setup) | **~1s** | — | Persistent process, avoid re-resolving |
| load_package_graph | **~4ms** | <1% | Already fast |
| Session init | **~60μs** | <0.01% | Already fast |
| Global CLI overhead | **~5ms** | <0.5% | Already fast |
| Node.js + NAPI startup | **~3.7ms** | <0.4% | Already fast |
| oxc_resolver | **~170μs** | <0.02% | Already fast |
| js_resolver callback | **~1.25ms** | <0.1% | Already fast |

**The single most impactful optimization would be caching or parallelizing `load_user_config_file` calls.** The first cold load takes 164ms, and when new heavy dependencies are encountered, loads can take 741ms. For a 10-package monorepo, this accumulates to ~930ms of config loading before any task runs.

## Inter-Process Communication

vite-task uses Unix shared memory (`/dev/shm`) for parent-child process communication during task execution:
- Creates persistent mapping at `/shmem_<hash>`
- Maps memory into address space for fast IPC
- Cleaned up after spawn completion

## Known Issues

### Windows: Trace files break formatter
When `VITE_LOG_OUTPUT=chrome-json` is set, trace files (`trace-*.json`) are written to the project working directory. On Windows, `vp fmt` picks up these files and fails with "Unterminated string constant" because the trace files contain very long PATH strings.

**Recommendation**: Add `trace-*.json` to formatter ignore patterns, or write trace files to a dedicated directory outside the workspace.

### NAPI trace files empty for some projects
The Chrome tracing `FlushGuard` stored in a static `OnceLock` is never dropped when `process.exit()` is called. Fixed by adding `shutdownTracing()` NAPI function called before exit (commit `72b23304`). Some projects (frm-stack) produce traces because their exit path differs.

## Methodology

- **Tracing**: Rust `tracing` crate with `tracing-chrome` subscriber (Chrome DevTools JSON format)
- **Environment variables**: `VITE_LOG=debug`, `VITE_LOG_OUTPUT=chrome-json`
- **CI environment**: GitHub Actions ubuntu-latest runner
- **Measurement PRs**:
  - vite-task: https://github.com/voidzero-dev/vite-task/pull/178
  - vite-plus: https://github.com/voidzero-dev/vite-plus/pull/663
- **Trace sources**: Global CLI traces (6 files, vibe-dashboard), NAPI traces (20 files, frm-stack)
