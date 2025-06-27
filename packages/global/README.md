# Vite+

- Global `vite` CLI for Vite+
- `vite new`
- Everything else is delegated to [vite-plus][1] for local tasks

## Overview

```
$ vite --help

vite new               Scaffold new project
vite build [dir]       Run vite build (default in: ".")
vite optimize [dir]    Run vite optimize
vite preview [dir]     Run vite preview
vite dev [dir]         Run vite dev
vite lint [dir]        Run oxlint
vite lib [dir]         Run tsdown
vite test [dir]        Run vitest
vite bench [dir]       Run vitest bench
vite docs [dir]        Run vitepress
vite task [name]       Run [name] script of package.json#scripts in each workspace
```

## Development

- The global executable is `vite`, use `vite-dev` for development
- The local executable is `vite-plus`, use `vpl` for development

Example workflow:

1. Make `vite` (and `vite-dev`) available globally:

```sh
cd packages/global
pnpm link
pnpm dev
```

2. From `vite-plus` package, link `multiplexer` package and use `vite-plus` in
   any project's `package.json`:

```sh
cd packages/cli
pnpm link ../multiplexer/
pnpm dev
```

3. Build multiplexer

```sh
cd packages/multiplexer
pnpm dev
```

4. Install in project

Use `vite new` anywhere, or run this directly inside this repo:

```sh
cd packages/global/templates/minimal
pnpm link ../../../cli/
```

Outside this repo do `pnpm link to/vite-plus/packages/cli/`

5. Run tasks

Now the following commands all do the same thing:

```sh
vite lint
pnpm vite-plus lint
```

Or use the task runner for

```sh
vite task build lint
pnpm vite-plus task build lint
pnpm run all
```

## Commands

### lint, build, dev, preview, test, etc.

Execute our own toolchain in current directory

### new

Run questionnaire to copy a template to current or sub directory:

```sh
vite new
```

Or `vite-dev new` to directly use TS source code.

### task

`vite task [name]` or `vite-plus task [name]` runs script with the same `name`
from `package.json` across the monorepo (topologically sorted). Multiple `name`
arguments supported.

## Verdaccio

Install [Verdaccio][2] for local actual package installs ([pkg.pr.new][3]
publishes only from CI and e.g. `npm link` doesn't always cut it).

[1]: ../cli
[2]: ./verdaccio.md
[3]: https://github.com/stackblitz-labs/pkg.pr.new
