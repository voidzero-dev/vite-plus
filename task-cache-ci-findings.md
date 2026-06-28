# Findings: Reusing the Vite Task Cache Across CI Runs

Experiment report behind the docs added in `docs/guide/cache.md`
("Reusing the Cache Across CI Runs") and `docs/guide/ci.md`
("Caching Task Results Across Runs").

Every claim below was verified empirically — by reading the `vite-task` Rust
source, by local cache experiments, and by real GitHub Actions runs under the
**@wan9chi** account — not assumed. All experiments used a **main-branch
preview build** of Vite+ from pkg.pr.new (`vite-plus@1973`), except where noted.

## TL;DR

- The Vite Task cache (`node_modules/.vite/task-cache`) is content-addressed and
  **portable across machines and checkout paths**, so it can be persisted
  between CI runs with `actions/cache`.
- Cross-run reuse was proven on three repositories (a build, a plain `node`
  task, and a `tsc` type-check): a cold run saves the cache, a later run
  restores it via `restore-keys` and **replays the task instead of re-running**.
- Two non-obvious CI gotchas bust the cache silently; both are fixed by keeping
  tool-owned files out of the tracked tree. They are documented in the guide.

## How the cache works (from `vite-task` source)

- **Location:** `node_modules/.vite/task-cache/v<N>` at the workspace root.
  Schema version is currently **v17**
  (`crates/vite_task/src/session/cache/mod.rs`). One cache is a SQLite
  `cache.db` (tables `cache_entries`, `task_fingerprints`) plus `<uuid>.tar.zst`
  output archives. `VITE_CACHE_PATH` can relocate it.
- **Key (`SpawnFingerprint`)** = cwd (workspace-relative) + program fingerprint
  (relative if inside the workspace, else just the program name) + **args
  (verbatim)** + env fingerprints (only configured vars; values SHA-256'd).
  Globbed inputs are stored as `BTreeMap<RelativePath, xxHash3_64>` of file
  content.
- **Portability:** all paths are relativized to the workspace root; file content
  is hashed (xxHash3_64), not mtime'd; **no hostname / username / OS / absolute
  path** is mixed into the key. The serialization is endian-aware (`wincode`).
  => A cache produced on machine A at path X reuses on machine B at path Y, as
  long as source content and configured env are identical.
- **One caveat in the key:** if a task's **args** contain an absolute path, that
  path leaks into the key (see the `vp build` finding below).

## Local proof (portability + correctness)

Set up a project with the `vite-plus@1973` preview build and these tasks, then
copied it to **different absolute paths** to simulate independent CI checkouts.

| Test | Result |
| --- | --- |
| `node greet.mjs` task, cache made at path A, run at **path B** | `◉ cache hit, replaying`, `out.txt` restored from archive |
| Same, after editing `data.txt` | `○ cache miss: 'data.txt' modified, executing` (correct) |
| `vp build`, wipe cache + `dist`, **restore cache at same path** | `◉ cache hit, replaying`, `dist/` restored |
| `vp build`, restore cache at a **different** path | `○ cache miss: args changed` |

The last row's root cause (read from source): `vp build`'s args include the
**absolute path to the bundled Vite CLI**
(`node_modules/@voidzero-dev/vite-plus-core/cli.js`), inserted at
`packages/cli/binding/src/cli/resolver.rs:154-167`. It is **stable per checkout
path with no per-run randomness**, so `vp build` reuses across runs whenever the
checkout path is stable — the normal case on GitHub-hosted runners
(`/home/runner/work/<repo>/<repo>`). Plain `node`/script tasks have no such
limitation.

## CI proof (real GitHub Actions runs)

### Repo 1 — `wan9chi/vp-cache-ci-demo` (primary, raw pnpm setup)

`build` (= `vp build`) + `greet` (= `node greet.mjs`), workflow caches
`node_modules/.vite/task-cache` with
`key: vite-task-${{ runner.os }}-${{ github.sha }}` and
`restore-keys: vite-task-${{ runner.os }}-`.

- **Run 1 (cold):** `Cache not found for input keys: vite-task-Linux-…` →
  tasks execute → `Cache saved with key: vite-task-Linux-55a31d0…`.
- **Run 2 (different commit, README-only change):**
  `Cache restored from key: vite-task-Linux-55a31d0…` (partial, via
  `restore-keys`). `greet` → `◉ cache hit, replaying`, `31ms saved`.
  **`build` MISSED:** `○ cache miss: 'node_modules/.modules.yaml' modified`.
- **Run 3 (fix):** added `input: [{ auto: true }, '!node_modules/**']` to
  `build`. Build re-cached under the new key.
- **Run 4 (README-only change):** both **`build` and `greet` → `◉ cache hit,
  replaying`** (`351ms` and `31ms` saved). Clean cross-run, cross-commit reuse.

### Repo 2 — `wan9chi/vp-cache-vite-app` (real create-vite `react-ts`)

`vp run build` on a React 19 + TS app.

- Run 1 (cold, `28334550139`): build executes, `Cache saved with key:
  vite-task-Linux-fc23fdce…`.
- Run 2 (warm, `28334567415`): `Received 143483 … (100.0%)`,
  `Cache restored from key: vite-task-Linux-fc23fdce…`,
  `$ vp build ◉ cache hit, replaying`, `vp run: cache hit, 466ms saved.`
- Only `'!node_modules/**'` was needed. **Verdict: cross-run build reuse works.**

### Repo 3 — `wan9chi/vp-cache-tsc-lib` (non-Vite: `tsc --noEmit`)

- Run 1 (cold, `28334819821`): `tsc` executes, cache saved.
- Run 2 (warm, `28334840009`): `Cache restored from key:
  vite-task-Linux-1362a2b…`, `$ tsc --noEmit … ◉ cache hit, replaying`,
  `vp run: cache hit, 573ms saved.` **Verdict: cross-run reuse works for a
  non-Vite task.**

## The two CI gotchas (and fixes)

### 1. `node_modules/.modules.yaml` (content changes between runs)

pnpm rewrites this metadata file whenever the dependency cache is restored, so a
build that reads it misses on the next run
(`cache miss: 'node_modules/.modules.yaml' modified`). **Fix:** exclude
`node_modules` from the task's inputs — dependency identity is still tracked
through the committed lockfile:

```ts
build: { command: 'vp build', input: [{ auto: true }, '!node_modules/**'] }
```

### 2. `tsc`'s `.tsbuildinfo` (removal changes a scanned directory listing)

A root-level `tsconfig.tsbuildinfo` is written during the run but is gitignored,
so it is **present when the cache is saved and absent on the next fresh
checkout**. Because `tsc` scans the workspace-root directory, the listing
changes and the task misses with
`'tsconfig.tsbuildinfo' removed from workspace root`.

Critically, a file-glob exclusion **does not fix this**. I reproduced the
mechanism deterministically with a directory-scanning task:

| Task | Writes | Input exclusion | Re-run after deleting the file |
| --- | --- | --- | --- |
| `bi.mjs` (opens specific files only) | root `*.tsbuildinfo` | `!**/*.tsbuildinfo` | **HIT** |
| `biScan.mjs` (scans `.` like tsc) | root `d.tsbuildinfo` | `!**/*.tsbuildinfo` | **MISS** (`'d.tsbuildinfo' removed from workspace root`) |
| `biScan.mjs` (scans `.`) | `node_modules/.cache/e.tsbuildinfo` | `!node_modules/**` | **HIT** |

So an `input` glob suppresses a file's **content** tracking but not its presence
in a **scanned directory listing**. **Fix:** make the tool write the file into
an already-excluded directory:

```ts
typecheck: {
  command: 'tsc --noEmit --tsBuildInfoFile node_modules/.cache/tsc/typecheck.tsbuildinfo',
  input: [{ auto: true }, '!node_modules/**'],
}
```

This was confirmed in CI on Repo 3 (`573ms saved` on the warm run).

## Recommended workflow (essence of the guide)

```yaml
- uses: actions/checkout@v4
- uses: voidzero-dev/setup-vp@v1          # or pnpm/action-setup + setup-node
  with: { node-version: '24', cache: true } # cache: true = DEPENDENCIES only
- name: Cache Vite Task results            # the TASK cache is separate
  uses: actions/cache@v4
  with:
    path: node_modules/.vite/task-cache
    key: vite-task-${{ runner.os }}-${{ github.sha }}
    restore-keys: |
      vite-task-${{ runner.os }}-
- run: vp run build                        # must go through `vp run`
```

- `setup-vp`'s `cache: true` caches **dependencies**
  (`vite-plus-{os}-{arch}-{pm}-{lockfile-hash}`), **not** task results — verified
  in its `action.yml`. The task cache needs its own `actions/cache` step.
- Use a **rolling key** (`github.sha`) + a **prefix `restore-keys`**, because a
  GitHub cache entry is immutable: "if the provided key matches an existing
  cache, a new cache is not created."
- Only `vp run <task>` is cached; `vp build` on its own is not.
- Optionally fold `hashFiles('pnpm-lock.yaml')` into the key so a dependency or
  toolchain change starts from a clean cache.

## Limitations & workarounds (GitHub Actions cache)

- **10 GB** default budget per repo (raisable / pay-as-you-go since Nov 2025);
  over budget, entries are evicted **least-recently-used first**. The task cache
  dir is small, so this rarely binds.
- Caches **not accessed for 7 days are deleted**; a quiet repo rebuilds and
  repopulates.
- **Immutable keys** → handled by the rolling-key pattern.
- **Branch scope:** a run can restore caches from its **own branch** and the
  repo's **default branch**, not sibling branches. Run CI on the default branch
  so its cache becomes the shared baseline for PRs. Fork PRs have restricted,
  read-only cache access.

## Caveats / open issues

- **`setup-vp` end-to-end was not greened.** The guide leads with the `setup-vp`
  install layer (consistent with the existing CI guide) + the **proven**
  `actions/cache` step. I could not get a fully green `setup-vp` run because:
  (a) `vp install` enforces a supply-chain **minimum-release-age** policy
  (`ERR_PNPM_MINIMUM_RELEASE_AGE_VIOLATION`) that rejects the recently-published
  pkg.pr.new preview build (and `.npmrc minimum-release-age=0` does not override
  it); and (b) one scratch repo, `wan9chi/vp-cache-setupvp-demo`, hit a
  persistent GitHub-side **workflow startup failure** (0 jobs, valid YAML, valid
  `voidzero-dev/setup-vp@v1` ref). The `setup-vp` workflow YAML itself parses and
  runs (it executed on `vp-cache-ci-demo`, failing only at the release-age gate).
  The `actions/cache` step is identical to the proven raw setup.
- **pkg.pr.new lockfile integrity:** a lockfile generated during a flaky fetch can
  omit the `integrity` field on the preview tarball entries, which
  `--frozen-lockfile` rejects (`ERR_PNPM_MISSING_TARBALL_INTEGRITY`); regenerate
  until every direct entry has `integrity`.

## Artifacts

- Docs: `docs/guide/cache.md` (new "Reusing the Cache Across CI Runs" section),
  `docs/guide/ci.md` (new "Caching Task Results Across Runs" section).
- Experiment repos under @wan9chi: `vp-cache-ci-demo` (green, primary),
  `vp-cache-vite-app` (green, react-ts), `vp-cache-tsc-lib` (green, tsc),
  `vp-cache-setupvp-demo` (red — GitHub startup glitch).
