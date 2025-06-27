# vp

- Global CLI for Vite+
- For now the package and binary are dubbed `vp`
- Only one command: `vp new`
- Everything else is delegated to [vite-plus][1] for local tasks

## Development

- The global executable is `vp`, use `vpg` for development
- The local executable is `vite-plus`, use `vpl` for development

Example workflow:

1. Make `vp` (and `vpg`) available globally:

```sh
cd packages/global
pnpm link
```

1. From `vite-plus` package, link `multiplexer` package and use `vite-plus` in
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

Use `vp new` anywhere, or run this directly inside this repo:

```sh
cd packages/global/templates/minimal
pnpm link ../../../cli/
```

5. Run tasks

Now the following commands all do the same thing:

```sh
vp task build lint
pnpm vite-plus task build lint
pnpm run all
```

## Commands

### new

Run questionnaire to copy a template to current or sub directory:

```sh
vp new
```

Or `vpg new` to directly use TS source code.

### task

`vp task [name]` or `vite-plus task [name]` runs script with the same `name`
from `package.json` across the monorepo (topologically sorted). Multiple `name`
arguments supported.

## Verdaccio

Install [Verdaccio][2] for local actual package installs ([pkg.pr.new][3]
publishes only from CI and e.g. `npm link` doesn't always cut it).

[1]: ../cli
[2]: ./verdaccio.md
[3]: https://github.com/stackblitz-labs/pkg.pr.new
