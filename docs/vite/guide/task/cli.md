# CLI Reference

## `vp run` {#vp-run}

Run tasks defined in `vite.config.ts` or `package.json` scripts.

```bash
vp run [options] [task] [-- additional-args]
```

All options must come **before** the task name. If `task` is omitted, an interactive task selector is shown.

### Task Specifier {#task-specifier}

- `build` — runs the `build` task in the current package
- `@my/app#build` — runs the `build` task in a specific package

### Options {#options}

| Flag | Short | Description |
| --- | --- | --- |
| `--recursive` | `-r` | Run in all workspace packages, in topological order |
| `--transitive` | `-t` | Run in the current package and its transitive dependencies |
| `--workspace-root` | `-w` | Run in the workspace root package |
| `--filter <pattern>` | `-F` | Select packages by name, directory, or glob (repeatable) |
| `--cache` | — | Enable caching for all tasks and scripts |
| `--no-cache` | — | Disable caching entirely |
| `--ignore-depends-on` | — | Skip explicit `dependsOn` dependencies |
| `--verbose` | `-v` | Show detailed execution summary |
| `--last-details` | — | Display the summary from the last run |

### Additional Arguments {#additional-arguments}

Arguments after `--` are passed through to the task command:

```bash
vp run test -- --reporter verbose
```

### Filter Patterns {#filter-patterns}

| Pattern | Description |
| --- | --- |
| `@my/app` | Exact package name |
| `@my/*` | Glob matching |
| `./packages/app` | By directory |
| `{./packages/app}` | By directory (braced form) |
| `@my/app...` | Package and its dependencies |
| `...@my/core` | Package and its dependents |
| `@my/app^...` | Dependencies only (exclude package itself) |
| `...^@my/core` | Dependents only (exclude package itself) |
| `!@my/utils` | Exclude a package |

Multiple `--filter` flags are combined as a union. Exclusion filters (`!`) are applied after all inclusions.

## `vp cache clean` {#vp-cache-clean}

Delete all cached task results:

```bash
vp cache clean
```

Tasks will run fresh on the next invocation.

## Exit Codes {#exit-codes}

| Scenario | Exit code |
| --- | --- |
| All tasks succeed | `0` |
| Single task fails | The task's own exit code |
| Multiple tasks fail | `1` |
| Task not found (non-interactive) | `1` |
