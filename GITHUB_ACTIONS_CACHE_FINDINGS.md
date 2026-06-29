# GitHub Actions Vite Task Cache Findings

Date: 2026-06-29

This file records the experiments behind `docs/guide/github-actions-cache.md`.

## Summary

- Vite Task cache lives at `node_modules/.vite/task-cache`.
- Local cache must work before GitHub Actions cache reuse can help.
- Copying only `node_modules/.vite/task-cache` into a fresh checkout can produce cache hits and restore configured outputs.
- Real GitHub Actions cache reuse worked in `wan9chi/react-vite-shadcn-ui` with `actions/cache/restore@v6` and `actions/cache/save@v6`.
- Automatic input tracking can include volatile install metadata under `node_modules`; explicit CI task `input` globs were the most reliable workaround.
- A monorepo transitive workspace target restored from the same cache directory and hit `3/3` tasks after a fresh checkout-style install.

## Vite+ Build Used

The checked-out repo was behind GitHub `main` during the experiment. The current `main` SHA was `bc5c747aff29cc16ee0598e937820f36d27f7702`, but direct pkg.pr.new installs for that SHA returned 404.

The newest available default-branch package found on the pkg.pr.new page was:

```text
https://pkg.pr.new/voidzero-dev/vite-plus@9bc520a
9bc520ac25d6233a65007fc04804d77aadde28d5
```

GitHub Actions installed that package successfully and reported:

```text
vp v0.2.1
vite v8.1.0
rolldown v1.1.2
vitest v4.1.9
oxfmt v0.56.0
oxlint v1.71.0
oxlint-tsgolint v0.23.0
tsdown v0.22.3
```

Local access to `pkg.pr.new` was flaky from this machine. Several direct `curl`, `npm`, and `pnpm` attempts failed with TLS `ECONNRESET`. One subagent worked around local DNS by resolving `pkg.pr.new` to Cloudflare Pages addresses; another used an already-available preview SHA from a labeled PR branch.

## Source Behavior

Repo/source inspection found:

- Default cache path: `workspaceRoot/node_modules/.vite/task-cache`.
- Current schema directory observed in experiments: `v17`.
- Cache contents include `cache.db`, `db_open.lock`, `last-summary.json`, and `.tar.zst` output archives.
- `run.cache` defaults to caching configured tasks and not package scripts.
- `vp run --cache` enables script caching for that invocation; `run.cache: true` enables scripts and tasks in config.
- Direct built-ins such as `vp build` are not cached unless run through `vp run`.

## Real GitHub Actions Experiment

Fork:

```text
https://github.com/wan9chi/react-vite-shadcn-ui
```

Workflow:

```text
.github/workflows/vite-task-cache.yml
```

Key successful runs:

- `28336567473`: restored previous cache and hit `ci-build`, `ci-lint`, and `stamp`.
- `28336646946`: verified `actions/cache/restore@v6` and `actions/cache/save@v6`; restored from a previous key and hit all configured tasks.

Final task config used explicit inputs:

```ts
tasks: {
  'ci-build': {
    command: 'vp build',
    input: [
      'components.json',
      'index.html',
      'package.json',
      'src/**',
      'tailwind.config.ts',
      'tsconfig*.json',
      'vite.config.ts',
    ],
    output: ['dist/**'],
  },
  'ci-lint': {
    command: 'vp lint',
    input: [
      'components.json',
      'eslint.config.js',
      'package.json',
      'src/**',
      'tailwind.config.ts',
      'tsconfig*.json',
      'vite.config.ts',
    ],
  },
  stamp: {
    command: 'node scripts/stamp.mjs',
    input: ['scripts/stamp.mjs'],
    output: ['dist/stamp.txt'],
  },
}
```

Observed successful log lines:

```text
Cache restored from key: vite-task-exp-Linux-X64-...
$ vp build ◉ cache hit, replaying
$ vp lint ◉ cache hit, replaying
$ node scripts/stamp.mjs ◉ cache hit, replaying
```

The `stamp` task restored `dist/stamp.txt`; the restored file retained the timestamp from the earlier cached run.

## Failed Or Partial Iterations

- Initial Vite template script `tsc -b && vite build` was not a good cache target. It modified tracked inputs and reported `not cached because they modified their inputs`.
- The original ESLint script failed on the chosen OSS template. Switching to `vp lint` made the task pass.
- A first workflow key hashed `package.json` and `pnpm-lock.yaml` while CI used `pnpm install --no-frozen-lockfile`; the lockfile changed during the job, so restore and save keys did not match. Normal projects should commit a stable lockfile and use frozen installs.
- Restoring an automatically tracked cache produced first-run misses when install metadata changed under `node_modules`, including `.modules.yaml`.
- Explicit `input` globs for CI tasks fixed the real Actions experiment. In another Vite template simulation, keeping automatic tracking and excluding `!node_modules/.modules.yaml`, `!node_modules/.pnpm-workspace-state-v1.json`, and `!node_modules/.vite/**` fixed same-path restore.

## Other Local Experiments

Non-Vite TypeScript project:

- `tsc --noEmit` hit locally and after simulated CI restore.
- `tsc -p tsconfig.json` restored `dist-types/**`.
- A Node report script restored `artifacts/**`.
- A package.json script hit when `run.cache.scripts: true` was enabled.
- Copying only `node_modules/.vite/task-cache` into a fresh project copy produced hits and output restoration.

Vite template project:

- `lint`, `test`, and `build` hit locally.
- `build` restored `dist` with `output: ['dist/**']`.
- Same absolute checkout path hit after reinstall and cache restore once volatile `node_modules` inputs were excluded.
- Different absolute checkout path missed: `lint` due `OXLINT_TSGOLINT_PATH`, and `test`/`build` due `args changed`.

Monorepo project:

- Installed `vite-plus` from `https://pkg.pr.new/voidzero-dev/vite-plus@9bc520a`.
- Workspace packages were `@cache-exp/core`, `@cache-exp/util`, and `@cache-exp/app`, with `util` depending on `core` and `app` depending on `util`.
- Package-level `vite.config.ts` task definitions were discovered by `vp run -t @cache-exp/app#build`; a root-only task definition was not discovered for that package target.
- Cold transitive build produced `0/3` hits; warm build produced `3/3` hits.
- Restoring only `node_modules/.vite/task-cache` into a fresh checkout-style copy after `pnpm install --frozen-lockfile` produced `3/3` hits and restored each package's `dist/index.txt`.
- Deleting `dist` caused misses until task inputs excluded `!dist` and `!dist/**` while keeping `output: ['dist/**']`.

## GitHub Actions Cache Notes

Verified against current GitHub docs:

- Cache matching searches the exact key first, then prefix matches and `restore-keys`.
- Cache scope includes branch/tag restrictions; default-branch caches are available to other branches.
- Pull request merge-ref caches are scoped narrowly.
- GitHub removes caches that have not been accessed for over 7 days.
- Repository cache storage defaults to 10 GB before eviction pressure.
- `actions/cache` v6 exists and worked in the fork experiment.
