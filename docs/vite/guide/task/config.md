# Task Runner Config

The task runner is configured under the `run` field in your `vite.config.ts`:

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  run: {
    cache: {
      /* ... */
    },
    tasks: {
      /* ... */
    },
  },
});
```

## `run.cache` {#run-cache}

- **Type:** `boolean | { scripts?: boolean, tasks?: boolean }`
- **Default:** `{ scripts: false, tasks: true }`

Global cache settings. Controls whether task results are cached and replayed on subsequent runs.

```ts [vite.config.ts]
export default defineConfig({
  run: {
    cache: {
      scripts: true, // Cache package.json scripts (default: false)
      tasks: true, // Cache task definitions (default: true)
    },
  },
});
```

Shorthands: `cache: true` enables both, `cache: false` disables both.

Tasks defined in `vite.config.ts` are cached by default. Plain `package.json` scripts (without a matching task entry) are **not** cached by default. See [When Is Caching Enabled?](./caching#when-is-caching-enabled) for the full resolution order including CLI overrides.

## `run.tasks` {#run-tasks}

- **Type:** `Record<string, TaskConfig>`

Defines named tasks. Each key is a task name that can be run with `vp run <name>`.

```ts [vite.config.ts]
export default defineConfig({
  run: {
    tasks: {
      build: {
        command: 'vp build',
        dependsOn: ['lint'],
      },
      lint: {
        command: 'vp lint',
      },
    },
  },
});
```

If a task name matches a script in `package.json`, the script is used automatically when `command` is omitted. See [Getting Started](./getting-started#task-definitions) for details.

### `command` {#command}

- **Type:** `string`
- **Default:** matching `package.json` script

Shell command to run. If omitted, uses the script with the same name from `package.json`.

```ts
tasks: {
  build: {
    command: 'vp build',
  },
}
```

You cannot define a command in both `vite.config.ts` and `package.json` for the same task name — it's one or the other.

Commands joined with `&&` are automatically split into independently cached sub-tasks. See [Compound Commands](./running-tasks#compound-commands).

### `dependsOn` {#depends-on}

- **Type:** `string[]`
- **Default:** `[]`

Tasks that must complete successfully before this one starts.

```ts
tasks: {
  deploy: {
    command: 'deploy-script --prod',
    dependsOn: ['build', 'test'],
  },
}
```

Dependencies can reference tasks in other packages using the `package#task` format:

```ts
dependsOn: ['@my/core#build', '@my/utils#lint'];
```

See [Task Dependencies](./getting-started#task-dependencies) for details on how explicit and topological dependencies interact.

### `cache` {#cache}

- **Type:** `boolean`
- **Default:** `true`

Whether to cache this task's output. Set to `false` for tasks that should never be cached, like dev servers:

```ts
tasks: {
  dev: {
    command: 'vp dev',
    cache: false,
  },
}
```

### `envs` {#envs}

- **Type:** `string[]`
- **Default:** `[]`

Environment variables included in the cache fingerprint. When any listed variable's value changes, the cache is invalidated.

```ts
tasks: {
  build: {
    command: 'vp build',
    envs: ['NODE_ENV'],
  },
}
```

Wildcard patterns are supported: `VITE_*` matches all variables starting with `VITE_`.

```
$ NODE_ENV=development vp run build    # first run
$ NODE_ENV=production vp run build     # cache miss: envs changed
```

### `passThroughEnvs` {#pass-through-envs}

- **Type:** `string[]`
- **Default:** see below

Environment variables passed to the task process but **not** included in the cache fingerprint. Changing these values won't invalidate the cache.

```ts
tasks: {
  build: {
    command: 'vp build',
    passThroughEnvs: ['CI', 'GITHUB_ACTIONS'],
  },
}
```

A set of common environment variables are automatically passed through to all tasks:

- **System:** `HOME`, `USER`, `PATH`, `SHELL`, `LANG`, `TZ`
- **Node.js:** `NODE_OPTIONS`, `COREPACK_HOME`, `PNPM_HOME`
- **CI/CD:** `CI`, `VERCEL_*`, `NEXT_*`
- **Terminal:** `TERM`, `COLORTERM`, `FORCE_COLOR`, `NO_COLOR`

### `inputs` {#inputs}

- **Type:** `Array<string | { auto: boolean }>`
- **Default:** `[{ auto: true }]` (auto-inferred)

Files to track for cache invalidation. By default, the task runner automatically detects which files a command reads. See [Automatic File Tracking](./caching#automatic-file-tracking) for how this works.

**Exclude files** from automatic tracking:

```ts
tasks: {
  build: {
    command: 'vp build',
    inputs: [{ auto: true }, '!**/*.tsbuildinfo', '!dist/**'],
  },
}
```

**Specify explicit files** only (disables automatic tracking):

```ts
tasks: {
  build: {
    command: 'vp build',
    inputs: ['src/**/*.ts', 'vite.config.ts'],
  },
}
```

**Disable file tracking** entirely (cache only on command/env changes):

```ts
tasks: {
  greet: {
    command: 'node greet.mjs',
    inputs: [],
  },
}
```

::: tip
Glob patterns are resolved relative to the package directory, not the task's `cwd`.
:::

### `cwd` {#cwd}

- **Type:** `string`
- **Default:** package root

Working directory for the task, relative to the package root.

```ts
tasks: {
  'test-e2e': {
    command: 'vp test',
    cwd: 'tests/e2e',
  },
}
```
