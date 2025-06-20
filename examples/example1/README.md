# Vite+ example repo

This example has `apps/spa` and `apps/next`, both depend on `packages/logger`.

## Config

- Workspaces are self-contained
- Tasks can run from root (run task in all workspaces) or from child workspace dir
- Any executable we start in child workspace `cwd` should find its config there
- Each workspace contains its own `scripts` in `package.json`
- Each workspace contains its own dependencies
- Each workspace contains its own configuration files for dev, build, test, lint, etc.
- Configuration extension should be explicit and depends on tooling (e.g. `oxlint` has `extends`)
- We can add `extends` to `vite.config.ts`:

```ts
import { defineConfig } from "vite-plus";
export default defineConfig({
  extends: "../../vite.config.ts"
});
```

## Example: `vite task build`

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
vite task build
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
        "cache": true
      }
    ],
    [
      {
        "command": "next",
        "args": ["build"],
        "cwd": "apps/next",
        "cache": true
      },
      {
        "command": "vite",
        "args": ["build"],
        "cwd": "apps/spa",
        "cache": true
      }
    ]
  ]
}
```

- Running `next` from `apps/next` and it will find its own `next.config.ts`
- Running `vite build` from child workspace will read `vite.config.ts` (which
  might `extend` from root config)

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
        "cache": true
      }
    ],
    [
      {
        "command": "next",
        "args": ["dev"],
        "cwd": "apps/next",
        "cache": false
      },
      {
        "command": "vite",
        "args": ["dev"],
        "cwd": "apps/spa",
        "cache": false
      }
    ]
  ]
}
```
