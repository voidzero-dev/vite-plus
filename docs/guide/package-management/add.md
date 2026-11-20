# vp add

Add packages to the project and update its manifest and lockfile.

```bash
vp add react
vp add -D typescript
vp add --save-peer react
vp add --filter web react
```

## Options

Choose one dependency target at a time. Peer dependencies follow the native manager's behavior, including whether it also records a development dependency.

| Option                       | Description                                            | Notes                                                                          |
| ---------------------------- | ------------------------------------------------------ | ------------------------------------------------------------------------------ |
| `-P, --save-prod`            | Save to dependencies, the default target.              | Choose one dependency target.                                                  |
| `-D, --save-dev`             | Save to devDependencies.                               | Choose one dependency target.                                                  |
| `--save-peer`                | Request the manager's peer-dependency save behavior.   | Choose one dependency target.                                                  |
| `-O, --save-optional`        | Save to optionalDependencies.                          | Choose one dependency target.                                                  |
| `-E, --save-exact`           | Save an exact dependency version.                      |                                                                                |
| `--save-catalog-name <name>` | Save the dependency to a named catalog.                |                                                                                |
| `--save-catalog`             | Save the dependency to the default catalog.            |                                                                                |
| `--allow-build <names>`      | Allow the named packages to run postinstall scripts.   |                                                                                |
| `--ignore-scripts`           | Do not run lifecycle scripts.                          |                                                                                |
| `--no-optional`              | Do not install optionalDependencies.                   | Cannot be combined with --global.                                              |
| `--frozen-lockfile`          | Prevent lockfile changes.                              | Cannot be combined with --global. Overrides an earlier `--no-frozen-lockfile`. |
| `--no-frozen-lockfile`       | Explicitly allow lockfile updates.                     | Cannot be combined with --global. Overrides an earlier `--frozen-lockfile`.    |
| `--lockfile-only`            | Update the lockfile without a full install.            | Cannot be combined with --global.                                              |
| `--prefer-offline`           | Use cached packages when available.                    | Cannot be combined with --global.                                              |
| `--offline`                  | Only use packages already in cache.                    | Cannot be combined with --global.                                              |
| `-f, --force`                | Request a forced reinstall.                            | Cannot be combined with --global.                                              |
| `--no-lockfile`              | Skip reading or generating a lockfile.                 | Cannot be combined with --global.                                              |
| `--shamefully-hoist`         | Create a flat `node_modules` layout.                   | Cannot be combined with --global.                                              |
| `--silent`                   | Suppress Vite+ output and request native quiet output. | Cannot be combined with --global.                                              |
| `--filter <pattern>`         | Select workspaces by pattern.                          | Can be repeated.                                                               |
| `-w, --workspace-root`       | Add to workspace root.                                 |                                                                                |
| `--workspace`                | Require the dependency to exist in the workspace.      |                                                                                |
| `-g, --global`               | Install globally.                                      | Requires the global CLI. Uses the [global package store](./global-packages).   |
| `--node <version>`           | Node.js version to use for global installation.        | Requires --global.                                                             |
| `--concurrency <number>`     | Number of global package installs to run in parallel.  | Requires --global. Must be a positive integer.                                 |

## Compatibility

The commands below describe project operations. Global mode is described under [Options](#options).

- **npm:** `npm install`.
- **pnpm:** `pnpm add`.
- **Yarn Classic:** `yarn add`.
- **Yarn Modern:** `yarn add`.
- **Bun:** `bun add`.

| Option                       | npm             | pnpm             | Yarn Classic | Yarn Modern | Bun            | Notes                                              |
| ---------------------------- | --------------- | ---------------- | ------------ | ----------- | -------------- | -------------------------------------------------- |
| `-P, --save-prod`            | Yes             | Yes              | Yes          | Yes         | Yes            |                                                    |
| `-D, --save-dev`             | Yes             | Yes              | Yes          | Yes         | Yes            |                                                    |
| `--save-peer`                | Yes             | Yes              | Yes          | Yes         | Yes            |                                                    |
| `-O, --save-optional`        | Yes             | Yes              | Yes          | Yes         | Yes            |                                                    |
| `-E, --save-exact`           | Yes             | Yes              | Yes          | Yes         | Yes            |                                                    |
| `--save-catalog-name <name>` | No              | 10.12.0 or later | No           | No          | 1.4 or later   |                                                    |
| `--save-catalog`             | No              | 10.12.0 or later | No           | No          | 1.4 or later   |                                                    |
| `--allow-build <names>`      | No              | 10.4.0 or later  | No           | No          | No             |                                                    |
| `--ignore-scripts`           | Yes             | Yes              | Yes          | 3 or later  | Yes            | On Yarn Modern, lockfile-only mode takes priority. |
| `--no-optional`              | Yes             | Yes              | Yes          | No          | Yes            |                                                    |
| `--frozen-lockfile`          | No              | No               | Yes          | No          | Yes            |                                                    |
| `--no-frozen-lockfile`       | No              | No               | No           | No          | Yes            |                                                    |
| `--lockfile-only`            | Yes             | Yes              | No           | 3 or later  | Yes            |                                                    |
| `--prefer-offline`           | Yes             | Yes              | Yes          | No          | 1.4.1 or later |                                                    |
| `--offline`                  | Yes             | Yes              | Yes          | No          | 1.4.1 or later |                                                    |
| `-f, --force`                | Yes             | Yes              | Yes          | No          | Yes            |                                                    |
| `--no-lockfile`              | Yes             | Yes              | Yes          | No          | No             |                                                    |
| `--shamefully-hoist`         | No              | Yes              | No           | No          | No             |                                                    |
| `--silent`                   | Yes             | Yes              | Yes          | No          | Yes            |                                                    |
| `--filter <pattern>`         | 7.14.0 or later | Yes              | No           | Yes         | 1.4 or later   |                                                    |
| `-w, --workspace-root`       | 8.1.0 or later  | Yes              | Yes          | No          | No             |                                                    |
| `--workspace`                | No              | Yes              | No           | No          | No             |                                                    |
| `-g, --global`               | Yes             | Yes              | Yes          | Yes         | Yes            |                                                    |
| `--node <version>`           | Yes             | Yes              | Yes          | Yes         | Yes            |                                                    |
| `--concurrency <number>`     | Yes             | Yes              | Yes          | Yes         | Yes            |                                                    |

## Native Options

For native options, see [npm install](https://docs.npmjs.com/cli/v11/commands/npm-install), [pnpm add](https://pnpm.io/cli/add), [Yarn add](https://yarnpkg.com/cli/add), [Yarn Classic add](https://classic.yarnpkg.com/en/docs/cli/add), and [Bun add](https://bun.com/docs/pm/cli/add). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
