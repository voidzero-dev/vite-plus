# Run Config

You can configure Vite Task under the `run` field in `vite.config.ts`. Check out [`vp run`](/guide/run) to learn more about running scripts and tasks with Vite+.

```ts [vite.config.ts]
import { defineConfig } from 'vite-plus';

export default defineConfig({
  run: {
    enablePrePostScripts: true,
    cache: {/* ... */},
    tasks: {/* ... */},
  },
});
```

## `run.enablePrePostScripts`

- **Type:** `boolean`
- **Default:** `true`

Whether to automatically run `preX`/`postX` package.json scripts as lifecycle hooks when script `X` is executed.

When enabled (the default), running a script like `test` will automatically run `pretest` before it and `posttest` after it, if they exist in `package.json`.

```ts [vite.config.ts]
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

`cache: true` enables both task and script caching, `cache: false` disables both.

## `run.tasks`

- **Type:** `Record<string, Task | string | string[]>`

Defines tasks that can be run with `vp run <task>`.

As a shorthand, a task value can be a command string or an array of command strings directly:

```ts [vite.config.ts]
tasks: {
  build: 'vp build',
  check: ['vp lint', 'vp build'],
}
```

These are equivalent to setting only `command` on a task config:

```ts [vite.config.ts]
tasks: {
  build: { command: 'vp build' },
  check: { command: ['vp lint', 'vp build'] },
}
```

Use the object form when a task needs other fields like `cache`, `dependsOn`, `env`, or `input`.

### `command`

- **Type:** `string | string[]`

Defines the shell command to run for the task. The string is interpreted by a shell, so spaces, quoting, `&&`, pipes, and redirects all work as written on the command line.

```ts [vite.config.ts]
tasks: {
  build: {
    command: 'vp build',
  },
}
```

An array runs each entry as its own command, sequentially, equivalent to joining them with `&&`. It is **not** a way to split one command into argv tokens — `['vp', 'build']` would try to run a command called `vp` with no arguments, then a command called `build`, instead of `vp build`.

```ts [vite.config.ts]
tasks: {
  check: {
    // Two commands, run in order
    command: ['vp lint', 'vp build'],
  },
  bad: {
    // Wrong: this is NOT `vp build`; it is `vp` followed by `build`
    command: ['vp', 'build'],
  },
}
```

Each task defined in `vite.config.ts` must include its own `command`. You cannot define a task in both `vite.config.ts` and `package.json` with the same task name.

Commands joined with `&&` (or supplied as an array) are automatically split into independently cached sub-tasks. See [Compound Commands](/guide/run#compound-commands).

### `dependsOn`

- **Type:** `Array<string | { task: string, from: DependsOnFrom }>`
- **Default:** `[]`

`from` accepts the dependency types `"dependencies"`, `"devDependencies"`, `"peerDependencies"`, or an array of those values, such as `["dependencies", "devDependencies"]`.

Tasks that must complete successfully before this one starts.

```ts [vite.config.ts]
tasks: {
  deploy: {
    command: 'deploy-script --prod',
    dependsOn: ['build', 'test'],
  },
}
```

Dependencies can reference tasks in other packages using the `package#task` format:

```ts [vite.config.ts]
dependsOn: ['@my/core#build', '@my/utils#lint'];
```

Use the object form `{ task: string, from: DependsOnFrom }` to reference tasks from all dependencies:

```ts [vite.config.ts]
tasks: {
  test: {
    command: 'vp test',
    dependsOn: [{ task: 'build', from: ['dependencies', 'devDependencies'] }],
  },
}
```

For the example above, Vite Task reads the declaring package's direct `dependencies` and `devDependencies`, and runs the `build` task in each dependency that defines one. Packages without `build` are skipped.

See [Task Dependencies](/guide/run#task-dependencies) for details on how explicit and topological dependencies interact.

### `cache`

- **Type:** `boolean`
- **Default:** `true`

Whether to cache this task's output. Set to `false` for tasks that should never be cached, like dev servers:

```ts [vite.config.ts]
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

```ts [vite.config.ts]
tasks: {
  build: {
    command: 'node build.mjs',
    env: ['NODE_ENV'],
  },
}
```

Wildcard patterns and `!` exclusion patterns are supported: `VITE_*` matches all variables starting with `VITE_`, and `!VITE_SECRET` excludes the `VITE_SECRET` variable from the match.

For `vp build`, Vite reports Vite environment variables through [automatic tracking](/guide/automatic-data-tracking#cooperative-tracking). Do not add `VITE_*` or `NODE_ENV` here for a standard Vite build unless your project has extra build behavior Vite cannot report.

```bash
$ NODE_ENV=development vp run build    # first run
$ NODE_ENV=production vp run build     # cache miss: env 'NODE_ENV' changed
```

### `untrackedEnv`

- **Type:** `string[]`
- **Default:** see below

Environment variables passed to the task process but **not** included in the cache fingerprint. Changing these values won't invalidate the cache.

```ts [vite.config.ts]
tasks: {
  build: {
    command: 'node build.mjs',
    untrackedEnv: ['CI', 'GITHUB_ACTIONS'],
  },
}
```

`untrackedEnv` accepts the same wildcard and `!` exclusion patterns as [`env`](#env).

Do not put a variable in `untrackedEnv` if its value changes the task result. If a cache-reporting tool covers the variable through [automatic tracking](/guide/automatic-data-tracking#cooperative-tracking), leave it out of both `env` and `untrackedEnv`.

Vite Task passes a set of common environment variables to all tasks:

- **System:** `HOME`, `USER`, `PATH`, `SHELL`, `LANG`, `TZ`
- **Node.js:** `NODE_OPTIONS`, `COREPACK_HOME`, `PNPM_HOME`
- **CI/CD:** `CI`, `VERCEL_*`, `NEXT_*`
- **Terminal:** Color variables (`FORCE_COLOR`, `NO_COLOR`, `COLORTERM`, `TERM`, `TERM_PROGRAM`) aren't passed to tasks unless you list them under `env` (the value gets fingerprinted, so changing it invalidates the cache) or `untrackedEnv` (passed without fingerprinting). If `FORCE_COLOR` isn't in either list, the child gets `FORCE_COLOR=1` so cached logs stay colored. Colors get stripped on display when the terminal can't render them.

### `input`

- **Type:** `Array<string | { auto: boolean } | { pattern: string, base: "workspace" | "package" }>`
- **Default:** `[{ auto: true }]` (auto-inferred)

Vite Task automatically detects which files a command uses. See [Automatic Data Tracking](/guide/automatic-data-tracking) for the details and when to add manual config.

**Exclude files** from automatic tracking:

```ts [vite.config.ts]
tasks: {
  build: {
    command: 'vp build',
    // Use `{ auto: true }` to use automatic fingerprinting (default).
    input: [{ auto: true }, '!**/*.tsbuildinfo', '!dist/**'],
  },
}
```

**Specify explicit files** only without automatic tracking:

```ts [vite.config.ts]
tasks: {
  build: {
    command: 'vp build',
    input: ['src/**/*.ts', 'vite.config.ts'],
  },
}
```

**Resolve patterns relative to the workspace root** using the object form:

```ts [vite.config.ts]
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

```ts [vite.config.ts]
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

### `output`

- **Type:** `Array<string | { auto: boolean } | { pattern: string, base: "workspace" | "package" }>`
- **Default:** automatic write tracking

Vite Task automatically archives files generated by a successful task run and restores them on a cache hit.

If you omit `output`, Vite Task uses automatic write tracking to choose those files. Add explicit `output` entries when you need to override which files are restored.

```ts [vite.config.ts]
tasks: {
  build: {
    command: 'node build.mjs',
    output: ['dist/**', '!dist/cache/**'],
  },
}
```

Use `{ auto: true }` to keep automatic write tracking while adding explicit output globs.

This is useful when a task writes files that should not be restored from the cache. For example, exclude TypeScript `.tsbuildinfo` files:

```ts [vite.config.ts]
tasks: {
  typecheck: {
    command: 'tsc --build',
    output: [{ auto: true }, '!*.tsbuildinfo'],
  },
}
```

If a task writes outside its own package, use the object form with `base: "workspace"`:

```ts [vite.config.ts]
tasks: {
  build: {
    command: 'node build.mjs',
    output: [
      'dist/**',
      { pattern: 'shared-artifacts/**', base: 'workspace' },
    ],
  },
}
```

Set `output: []` to disable output restoration for a cached task:

```ts [vite.config.ts]
tasks: {
  report: {
    command: 'node scripts/report.mjs',
    output: [],
  },
}
```

Unlike `cache: false`, `output: []` still lets Vite Task fingerprint the task. On a cache hit, Vite Task skips the command and replays its terminal output. Use this for local caches when the task's output files are already there and do not need to be restored.

### `cwd`

- **Type:** `string`
- **Default:** package root

Working directory for the task, relative to the package root.

```ts [vite.config.ts]
tasks: {
  'test-e2e': {
    command: 'vp test',
    cwd: 'tests/e2e',
  },
}
```
