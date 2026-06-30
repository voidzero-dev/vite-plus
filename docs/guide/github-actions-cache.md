# GitHub Actions Cache

::: warning Experimental
Reusing Vite Task cache through GitHub Actions cache is experimental. Validate the workflow in your project before you depend on it for CI time savings.
:::

Vite Task stores task results in `node_modules/.vite/task-cache` at the workspace root. You can restore that directory in a later GitHub Actions run, then let Vite Task decide whether each task matches the current command, environment, inputs, and outputs.

GitHub Actions cache and Vite Task make separate decisions:

1. `actions/cache` restores and saves the cache directory based on the key in your workflow.
2. Vite Task uses the restored cache directory and replays only the tasks whose fingerprints still match.

## Before You Start

Use this workflow when all of these are true:

- The command runs through [`vp run`](/guide/run).
- The task hits on a second local run.
- The task has stable input and output tracking for CI.
- The workflow installs dependencies before restoring `node_modules/.vite/task-cache`.

Fix local misses first. GitHub Actions cache can move Vite Task's local cache directory between runs, but it cannot make an unstable task cacheable.

## Measure Before Caching Across Runs

Measure before you add GitHub Actions cache for Vite Task. Compare workflow duration with and without the restore and save steps. Check both the GitHub cache step time and the `vp run` time.

You may not need to restore Vite Task cache across GitHub Actions runs in these cases:

- The task is already fast enough. Cache restore and save steps add overhead, so short tasks can finish faster without this workflow.
- The cache is expensive to move between runs. Vite Task can still save time when the same task runs more than once in one workflow run. Across workflow runs, GitHub must download and upload the cache. If that transfer time is greater than rerunning the task, skip this workflow.

## 1. Define Cacheable CI Tasks

Only commands run through `vp run` use Vite Task caching. A direct command such as `vp build` does not use the task cache. Define a task in `vite.config.ts` for each command you want to cache in CI:

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  run: {
    tasks: {
      build: 'vp build',
      lint: 'vp lint',
    },
  },
});
```

This guide assumes each task already hits locally. If a task misses, fix its tracking config in `vite.config.ts` before adding the GitHub Actions cache steps. See [Automatic Tracking](/guide/automatic-tracking) and [`run.tasks`](/config/run#tasks).

Run each task twice:

```bash
vp run build
vp run build # should print "cache hit"
vp run lint
vp run lint # should print "cache hit"
```

## 2. Restore The Cache After Install

Restore `node_modules/.vite/task-cache` after `vp install`, because package installation can recreate or modify `node_modules`.

```yaml [.github/workflows/ci.yml]
name: CI

on:
  pull_request:
  push:
    branches: [main]

permissions:
  contents: read

jobs:
  ci:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: voidzero-dev/setup-vp@v1
        with:
          node-version: '24'

      - run: vp install

      - name: Restore Vite Task cache
        id: vite-task-cache
        uses: actions/cache/restore@v6
        with:
          path: node_modules/.vite/task-cache
          key: vite-task-${{ runner.os }}-${{ runner.arch }}-${{ github.run_id }}-${{ github.run_attempt }}
          restore-keys: |
            vite-task-${{ runner.os }}-${{ runner.arch }}-

      - run: vp run lint
      - run: vp run build

      - name: Save Vite Task cache
        if: success()
        uses: actions/cache/save@v6
        with:
          path: node_modules/.vite/task-cache
          key: ${{ steps.vite-task-cache.outputs.cache-primary-key }}
```

The primary key includes `github.run_id` and `github.run_attempt` so each successful run can save a new immutable cache entry. The restore prefix lets GitHub restore the newest cache for the same operating system and architecture.

Leave task inputs, including source files and lockfiles, out of the GitHub Actions key. Vite Task fingerprints them. If they change the Actions key, GitHub can skip useful restores before Vite Task decides which tasks still hit.

For workspaces, restore the task cache from the workspace root. Then run the same workspace targets you use locally, such as `vp run -t @my/app#build`. The same cache covers tasks from workspace dependencies.

## 3. Verify In The Logs

On the first run, the restore step should say that no cache was found, and the save step should create one. Pull requests from forks may be restore-only because GitHub can give the cache token read-only access. In that case, the save step warns and exits successfully without writing a cache entry.

On a later run, look for both layers:

```text
Cache restored from key: vite-task-Linux-X64-...
$ vp build ◉ cache hit, replaying
vp run: cache hit, 1.10s saved.
```

If GitHub restores a cache but Vite Task prints a cache miss, the Actions cache transport worked, but the task fingerprint changed.

## Keep Task Tracking Stable

If GitHub restores a cache but `vp run` prints a cache miss, fix the task fingerprint before changing the Actions cache key. See [Automatic Tracking](/guide/automatic-tracking) and [`run.tasks`](/config/run#tasks).

## Choose A Cache Key

Use a rolling primary key plus a restore prefix:

```yaml [.github/workflows/ci.yml]
key: vite-task-${{ runner.os }}-${{ runner.arch }}-${{ github.run_id }}-${{ github.run_attempt }}
restore-keys: |
  vite-task-${{ runner.os }}-${{ runner.arch }}-
```

The exact key misses on each new run because the key contains `github.run_id` and `github.run_attempt`. GitHub then searches the restore prefix and restores the newest matching cache.

Include:

- `runner.os` and `runner.arch`, because outputs and native tools can be platform-specific.
- A per-run value such as `github.run_id` and `github.run_attempt`, because GitHub cache entries are immutable.

If a dependency file affects a task result, track it in the task fingerprint rather than the GitHub Actions key.

## Manage Cache Eviction And Scope

GitHub evicts caches based on its cache retention and repository storage rules. Cache scope is also branch-aware: workflow runs can restore caches from the current branch and the default branch, while pull request merge-ref caches have limited scope.

Vite Task can clean the whole task cache, but it does not currently evict individual task entries by age or size. As new task entries and output archives are saved, `node_modules/.vite/task-cache` can keep growing.

Use GitHub Actions cache as the eviction boundary:

- Keep the cached `path` limited to the Vite Task cache directory.
- Keep the restore prefix scoped to compatible runners, such as the same OS and architecture.
- Delete stale GitHub Actions cache entries, save caches from fewer workflows, or adjust the repository cache limit if cache size causes churn.

See [GitHub's cache reference](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching) for the current eviction and scope rules.
