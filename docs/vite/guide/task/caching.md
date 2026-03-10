# Caching

## How It Works {#how-caching-works}

When a task runs successfully (exit code 0), its output is saved. On the next run, the task runner checks if anything changed:

1. **Command and arguments** — did the command itself change?
2. **Environment variables** — did any [fingerprinted env vars](/config/task#envs) change?
3. **Input files** — did any file that the command reads change?

If everything matches, the cached output is replayed instantly — the command never actually runs.

When a cache miss occurs, the task runner tells you exactly why:

```
$ vp lint ✗ cache miss: 'src/utils.ts' modified, executing
$ vp build ✗ cache miss: envs changed, executing
$ vp test ✗ cache miss: args changed, executing
```

## Automatic File Tracking {#automatic-file-tracking}

By default, the task runner uses a file system spy to intercept file reads during execution. When `vp build` runs, it detects which files the command opens — your `.ts` source files, `vite.config.ts`, `package.json`, etc. — and records their content hashes. On the next run, it re-checks those hashes to determine if anything changed.

This means caching works out of the box for most commands without any configuration. The spy also tracks:

- **File non-existence** — if a command probes for a file that doesn't exist (e.g., `utils.ts` during module resolution), creating that file later correctly invalidates the cache.
- **Directory listings** — if a command scans a directory (e.g., a test runner looking for `*.test.ts`), adding or removing files in that directory invalidates the cache.

When you need more control over which files are tracked, use the [`inputs`](/config/task#inputs) option.

## Cache Sharing {#cache-sharing}

The cache is content-based. If two different tasks run the exact same command, they share a single cache entry. For example:

```ts
tasks: {
  check: {
    command: 'vp lint && vp build',
  },
  release: {
    command: 'vp lint && deploy-script',
  },
}
```

The `vp lint` sub-command is identical in both tasks, so running `check` first means the `lint` step in `release` is an instant cache hit.

## Cache Storage {#cache-storage}

Cache data is stored in a SQLite database at `node_modules/.vite/task-cache/cache.db`. The database supports concurrent access — multiple `vp` processes can read the cache simultaneously.

## Clearing the Cache {#clearing-the-cache}

```bash
vp cache clean
```

This deletes the entire cache directory. Tasks will run fresh on the next invocation.
