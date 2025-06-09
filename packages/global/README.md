# vp

- Global CLI for Vite+
- For now the package and binary are dubbed `vp`
- Only one command: `vp new`
- Everything else is delegated to [vite-plus][1] for local tasks

## Development

- The global executable is `vp`, use `vpg` for development
- The local executable is `vite-plus`, use `vpl` for development

The `vpg` and `vpl` binaries require Node.js to run `.ts` directly.
Make them available globally, e.g. using `npm link` or alias.

## Commands

### new

Copy files from template dir to current dir:

```sh
vp new
```

### task

Example commands with included dummy template:

```sh
vp task dev#packages/app
vp task build#packages/app
vp task test#packages/lib -- run
pnpm run vite-plus task test#packages/lib -- run  # same
vp task run#packages/lib -- script.ts
```

## Verdaccio

Install [Verdaccio][2] for local development with actual package installs
([pkg.pr.new][3] publishes only from CI and e.g. `npm link` doesn't always cut it).

[1]: ../cli
[2]: ./verdaccio.md
[3]: https://github.com/stackblitz-labs/pkg.pr.new
