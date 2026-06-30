# GitHub Actions Cache

::: warning Experimental
Reusing Vite Task cache through GitHub Actions cache is experimental. Validate the workflow in your project before you depend on it for CI time savings.
:::

Vite Task stores task results in `node_modules/.vite/task-cache` at the workspace root. You can restore that directory in a later GitHub Actions run, then let Vite Task decide whether each task matches the current command, environment, inputs, and outputs.

GitHub Actions cache and Vite Task make separate decisions:

1. `actions/cache` restores and saves the cache directory based on the key in your workflow.
2. Vite Task uses the restored cache directory and replays only the tasks whose fingerprints still match.

This cache is separate from dependency caching. Keep using [`setup-vp` cache support](/guide/ci) for package installs, then restore the Vite Task cache after dependencies are installed.

## Before You Start

Use this workflow when all of these are true:

- The command runs through [`vp run`](/guide/run).
- The task hits on a second local run.
- The task has stable input and output tracking for CI.
- The workflow installs dependencies before restoring `node_modules/.vite/task-cache`.

Fix local misses first. GitHub Actions cache can move Vite Task's local cache directory between runs, but it cannot make an unstable task cacheable.

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

Keep tracking config in `vite.config.ts`. See [Automatic Tracking](/guide/automatic-tracking) and [`run.tasks`](/config/run#tasks) for `input`, `output`, `env`, and `untrackedEnv`.

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
          key: vite-task-${{ runner.os }}-${{ runner.arch }}-${{ hashFiles('**/pnpm-lock.yaml', '**/package-lock.json', '**/yarn.lock', '**/bun.lock', '**/bun.lockb') }}-${{ github.run_id }}-${{ github.run_attempt }}
          restore-keys: |
            vite-task-${{ runner.os }}-${{ runner.arch }}-${{ hashFiles('**/pnpm-lock.yaml', '**/package-lock.json', '**/yarn.lock', '**/bun.lock', '**/bun.lockb') }}-

      - run: vp run lint
      - run: vp run build

      - name: Save Vite Task cache
        if: success()
        uses: actions/cache/save@v6
        with:
          path: node_modules/.vite/task-cache
          key: ${{ steps.vite-task-cache.outputs.cache-primary-key }}
```

The primary key includes `github.run_id` and `github.run_attempt` so each successful run can save a new immutable cache entry. The restore prefix includes the lockfile hash, so a dependency change starts from a cold Vite Task cache instead of restoring entries from a different dependency graph.

Leave source files out of the GitHub Actions key. Vite Task fingerprints task inputs. If a source change updates the Actions key, GitHub can skip useful restores before Vite Task decides which tasks still hit.

## 3. Verify In The Logs

On the first run, the restore step should say that no cache was found, and the save step should create one. Pull requests from forks may be restore-only because GitHub can give the cache token read-only access. In that case, the save step warns and exits successfully without writing a cache entry.

On a later run, look for both layers:

```text
Cache restored from key: vite-task-Linux-X64-...
$ vp build ◉ cache hit, replaying
vp run: cache hit, 1.10s saved.
```

If GitHub restores a cache but Vite Task prints a cache miss, the Actions cache transport worked, but the task fingerprint changed.

## 4. Workspaces

Cache reuse works with workspace task targets. Define cacheable tasks in the package that owns the target, then run the same `vp run` command in CI:

```bash
vp run -t @my/app#build
```

For workspace builds, keep automatic tracking and add the workspace lockfile as a workspace-relative input:

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  run: {
    tasks: {
      build: {
        command: 'vp build',
        dependsOn: [{ task: 'build', from: 'dependencies' }],
        input: [{ auto: true }, { pattern: 'pnpm-lock.yaml', base: 'workspace' }, '!dist/**'],
        output: ['dist/**'],
      },
    },
  },
});
```

The object-form `dependsOn` runs `build` in direct workspace dependency packages that define that task. Each dependency can pull in more tasks through its own `dependsOn` config.

When the task cache is restored, Vite Task can replay hits for the target package and its workspace dependencies. In a transitive `core -> util -> app` experiment, restoring only `node_modules/.vite/task-cache` in a fresh checkout produced `3/3` cache hits and restored each package's `dist/**` output.

## Keep Tracking Stable Across CI Runs

Vite Task's [automatic tracking](/guide/automatic-tracking) records file reads, missing-file probes, and directory listings. In CI, a command can read files that describe the dependency install or tool state rather than source behavior. Those reads can change between runs and cause misses after a successful GitHub cache restore.

Treat these cases by root cause:

- **Dependency install metadata.** Package managers and build tools can read install metadata to resolve packages, discover workspace layout, or configure file-system access. Track dependency identity with committed package manifests and lockfiles. For `vp build`, keep automatic tracking and add the lockfile as an input. Use explicit `input` globs only for commands with a small input set you can name. If that command writes files, review `output` in the same task config.
- **Tool-owned incremental state.** Tools such as TypeScript can write incremental state into a directory they also scan. A fresh checkout may lack that file, so the next run can miss because the directory listing changed. Move that state into a generated cache directory or use explicit task inputs. For TypeScript, set `--tsBuildInfoFile` to a generated cache location instead of writing `*.tsbuildinfo` next to source files.
- **Task outputs.** If a task output also appears in the input fingerprint, deleting the output causes a miss instead of an output restore. Keep generated files out of `input`, then use `output` to choose which files Vite Task restores.
- **Absolute arguments.** Vite Task stores command arguments in the fingerprint. If a command receives a different absolute checkout path between runs, it can miss with `args changed`. GitHub-hosted runners check out a repository under a stable path unless you override the checkout path. On self-hosted runners, keep the working directory stable or avoid absolute arguments.

## Choose A Cache Key

Use a rolling primary key plus a restore prefix:

```yaml [.github/workflows/ci.yml]
key: vite-task-${{ runner.os }}-${{ runner.arch }}-${{ hashFiles('**/pnpm-lock.yaml') }}-${{ github.run_id }}-${{ github.run_attempt }}
restore-keys: |
  vite-task-${{ runner.os }}-${{ runner.arch }}-${{ hashFiles('**/pnpm-lock.yaml') }}-
```

The exact key misses on each new run because the key contains `github.run_id` and `github.run_attempt`. GitHub then searches the restore prefix and restores the newest matching cache. Vite Task checks restored entries before replaying a task.

Include:

- `runner.os` and `runner.arch`, because outputs and native tools can be platform-specific.
- Your lockfile hash, so GitHub restores caches from the same dependency graph.
- A per-run value such as `github.run_id` and `github.run_attempt`, because GitHub cache entries are immutable.

You can add a broader restore prefix only if the task inputs include the package manifests and lockfiles that define dependency identity. The broader prefix can save a download on some projects, but it can also restore a cache that Vite Task rejects task by task.

## Limitations And Workarounds

- **Cache entries are immutable.** Use a unique primary key per run and restore prefixes. GitHub will not update an existing cache entry in place.
- **Fork pull requests may be restore-only.** GitHub can allow forked pull request runs to restore existing caches without saving new entries. Populate shared caches from trusted branch or default-branch runs.
- **GitHub evicts caches.** GitHub removes caches that have not been accessed for over 7 days. It also evicts least-recently-used entries when repository cache storage exceeds its limit, which defaults to 10 GB. If this causes churn, narrow what you cache, delete stale entries, or increase the repository cache limit in GitHub settings.
- **Cache scope is branch-aware.** Workflow runs can restore caches from the current branch and the default branch. Pull request merge-ref caches have limited scope and help re-runs of the same pull request. See [GitHub's dependency caching reference](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching) for the full branch and tag rules.
- **Secrets can leak through cache contents.** Vite Task caches terminal output and configured output files. Keep secrets out of task logs and generated files that you cache.
- **Output tracking can be too broad or too narrow.** By default, Vite Task restores files that the task writes. Use `output` globs such as `['dist/**']` when you want to narrow restoration. Use `{ auto: true }` with extra globs when you want automatic write tracking plus extra files.
- **Failed tasks are not cached.** Vite Task saves successful task results. Fix failing lint, test, or build commands before expecting cache reuse.
- **Toolchain changes can start cold.** Vite Task uses schema-versioned cache directories. If a restored cache was written by an incompatible version, Vite Task ignores those entries and reruns the task.

::: tip
Restore after dependency install because the default cache path lives under `node_modules`. If your workflow must restore before install, set a stable absolute `VITE_CACHE_PATH` outside `node_modules` for all `vp run` steps and cache that directory instead.
:::
