# vp link

Create local development links through the selected package manager. Alias: `vp ln`.

```bash
vp link ../my-library
```

## Options

An optional package name or directory is passed to the manager.

There are no command-specific options.

## Compatibility

- **npm:** `npm link`.
- **pnpm:** `pnpm link`.
- **Yarn Classic:** `yarn link`.
- **Yarn Modern:** `yarn link`.
- **Bun:** `bun link`.

The meaning of a directory, package name, or omitted argument follows that manager's native linking workflow. In particular, Yarn 2 and later link projects differently from Yarn Classic's package registration workflow.

These links do not use Vite+'s [global package store](./global-packages) and do not appear in `vp list -g`. Use [unlink](./unlink) to remove native links.

## Native Options

For native options, see [npm link](https://docs.npmjs.com/cli/v11/commands/npm-link), [pnpm link](https://pnpm.io/cli/link), [Yarn link](https://yarnpkg.com/cli/link), [Yarn Classic link](https://classic.yarnpkg.com/en/docs/cli/link), and [Bun link](https://bun.com/docs/pm/cli/link). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
