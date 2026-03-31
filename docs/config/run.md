# Run Config

You can configure Vite Task under the `run` field in `vite.config.ts`. Check out [`vp run`](/guide/run) to learn more about running scripts and tasks with Vite+.

```ts
import { defineConfig } from 'vite-plus';

export default defineConfig({
  run: {
    enablePrePostScripts: true,
    cache: {
      /* ... */
    },
    tasks: {
      /* ... */
    },
  },
});
```

## `run.enablePrePostScripts`

- **Type:** `boolean`
- **Default:** `true`

Whether to automatically run `preX`/`postX` package.json scripts as lifecycle hooks when script `X` is executed.

When enabled (the default), running a script like `test` will automatically run `pretest` before it and `posttest` after it, if they exist in `package.json`.

```ts
export default defineConfig({
  run: {
    enablePrePostScripts: false, // Disable pre/post lifecycle hooks
  },
});
```

::: warning
This option can only be set in the workspace root's `vite.config.ts`. Setting it in a package's config will result in an error.
:::

## `run.cache`

- **Type:** `boolean | { scripts?: boolean, tasks?: boolean }`
- **Default:** `{ scripts: false, tasks: true }`

Controls whether task results are cached and replayed on subsequent runs.

```ts
export default defineConfig({
  run: {
    cache: {
      scripts: true, // Cache package.json scripts (default: false)
      tasks: true, // Cache task definitions (default: true)
    },
  },
});
```

`cache: true` enables both task and script caching, `cache: false` disables both.

## `run.tasks`

- **Type:** `Record<string, TaskConfig>`

Defines tasks that can be run with `vp run <task>`.

### `command`

- **Type:** `string`

Defines the shell command to run for the task.

```ts
tasks: {
  build: {
    command: 'vp build',
  },
}
```

Each task defined in `vite.config.ts` must include its own `command`. You cannot define a task in both `vite.config.ts` and `package.json` with the same task name.

Commands joined with `&&` are automatically split into independently cached sub-tasks. See [Compound Commands](/guide/run#compound-commands).

### `dependsOn`

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

See [Task Dependencies](/guide/run#task-dependencies) for details on how explicit and topological dependencies interact.

### `cache`

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

### `env`

- **Type:** `string[]`
- **Default:** `[]`

Environment variables included in the cache fingerprint. When any listed variable's value changes, the cache is invalidated.

```ts
tasks: {
  build: {
    command: 'vp build',
    env: ['NODE_ENV'],
  },
}
```

Wildcard patterns are supported: `VITE_*` matches all variables starting with `VITE_`.

```bash
$ NODE_ENV=development vp run build    # first run
$ NODE_ENV=production vp run build     # cache miss: variable changed
```

### `untrackedEnv`

- **Type:** `string[]`
- **Default:** see below

Environment variables passed to the task process but **not** included in the cache fingerprint. Changing these values won't invalidate the cache.

```ts
tasks: {
  build: {
    command: 'vp build',
    untrackedEnv: ['CI', 'GITHUB_ACTIONS'],
  },
}
```

A set of common environment variables are automatically passed through to all tasks:

- **System:** `HOME`, `USER`, `PATH`, `SHELL`, `LANG`, `TZ`
- **Node.js:** `NODE_OPTIONS`, `COREPACK_HOME`, `PNPM_HOME`
- **CI/CD:** `CI`, `VERCEL_*`, `NEXT_*`
- **Terminal:** `TERM`, `COLORTERM`, `FORCE_COLOR`, `NO_COLOR`

### `input`

- **Type:** `Array<string | { auto: boolean } | { pattern: string, base: "workspace" | "package" }>`
- **Default:** `[{ auto: true }]` (auto-inferred)

Vite Task automatically detects which files are used by a command (see [Automatic File Tracking](/guide/cache#automatic-file-tracking)). The `input` option can be used to explicitly include or exclude certain files.

**Exclude files** from automatic tracking:

```ts
tasks: {
  build: {
    command: 'vp build',
    // Use `{ auto: true }` to use automatic fingerprinting (default).
    input: [{ auto: true }, '!**/*.tsbuildinfo', '!dist/**'],
  },
}
```

**Specify explicit files** only without automatic tracking:

```ts
tasks: {
  build: {
    command: 'vp build',
    input: ['src/**/*.ts', 'vite.config.ts'],
  },
}
```

**Resolve patterns relative to the workspace root** using the object form:

```ts
tasks: {
  build: {
    command: 'vp build',
    input: [
      { auto: true },
      { pattern: 'shared-config/**', base: 'workspace' },
    ],
  },
}
```

The `base` field is required and controls how the glob pattern is resolved:
- `"package"`: relative to the package directory
- `"workspace"`: relative to the workspace root

**Disable file tracking** entirely and cache only on command/env changes:

```ts
tasks: {
  greet: {
    command: 'node greet.mjs',
    input: [],
  },
}
```

::: tip
String glob patterns are resolved relative to the package directory by default. Use the object form with `base: "workspace"` to resolve relative to the workspace root.
:::

### `cwd`

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
