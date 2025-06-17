# Vite+ example repo

This example has `apps/spa` and `apps/next`, both depend on `packages/logger`.

## Build

The "canonical" way using e.g. pnpm:

```sh
pnpm -F @repo/logger run build
pnpm -F @repo/next run build
pnpm -F @repo/spa run build
```

Btw, pnpm has `pnpm --recursive run build` which does topological sorting (
assuming it uses
[@pnpm/deps.graph-sequencer](https://github.com/pnpm/pnpm/tree/main/deps/graph-sequencer)).

With Vite+ (global CLI):

```sh
vp task build
```

- A task `build` is defined in `vite-task.json`
- `^build` syntax (Turborepo and Nx use this) → create dep graph from workspaces
- `apps/next` and `apps/spa` have `packages/logger` listed in `dependencies`
- take `build` from their `package.json#scripts`
- plan tasks
- run tasks
  1. `tsdown`
  2. `next build`
  3. `vite build`

Could also run through package manager:

```sh
pnpm run build
```

As [package.json#scripts](./package.json) has `"build": "vite-plus task build"`.

## Config

[Task Design → Task Configuration](https://linear.app/voidzero/document/vite-task-design-doc-d6f7384ab696#heading-task-configuration-651cfdec)

Example [vite-task.json](./vite-task.json):

```json
{
  "tasks": {
    "build": {
      "dependsOn": ["^build"],
      "cache": true
    },
    "dev": {
      "dependsOn": ["@repo/logger#build"],
      "cache": false,
      "longRunning": true
    }
  }
}
```

Borrowed from Turborepo, I think overall it's good. Yet there are things that we
can probably optimize, e.g.:

- Default `dependsOn` to prefix with `^` (e.g. `"^build"`) and use
  `package.json` dependencies to create graph
- Default to `cache: true` for `build` tasks
- Default to `cache: false` for `dev` tasks
- Defaults for output folders (e.g. `dist/**` for build tasks)

This should work well especially if we know that our own tools are being used.
We can also read from our own tooling's config e.g. to use non-default output
directory.

Here's a good example of how a configuration could be reduced significantly were
such defaults being applied:
https://github.com/motiondivision/motion/blob/main/turbo.json (perhaps even
zero-config).

## Execution

Simplified task execution graph:

```json
{
  "tasks": [
    [
      {
        "command": "tsdown",
        "args": [],
        "cwd": "packages/logger",
        "cachable": true
      }
    ],
    [
      {
        "command": "next",
        "args": ["build"],
        "cwd": "apps/next",
        "cachable": true
      },
      {
        "command": "vite",
        "args": ["build"],
        "cwd": "apps/spa",
        "cachable": true
      }
    ]
  ]
}
```

- Running `next` from `apps/next` and it will find its own `next.config.ts`
- Running `vite build` will read/merge `viteplus.config.ts` from both parent +
  child workspace and take `build` config

## dev

If we would pre-build `packages/logger` and then watch the apps:

```json
{
  "tasks": [
    [
      {
        "command": "tsdown",
        "args": [],
        "cwd": "packages/logger",
        "cachable": true
      }
    ],
    [
      {
        "command": "next",
        "args": ["dev"],
        "cwd": "apps/next",
        "cachable": false
      },
      {
        "command": "vite",
        "args": ["dev"],
        "cwd": "apps/spa",
        "cachable": false
      }
    ]
  ]
}
```
