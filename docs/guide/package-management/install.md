# vp install

Install the dependencies declared by the project. Alias: `vp i`. Supplying package names switches to [add](./add) behavior.

```bash
vp install
vp install --frozen-lockfile
vp install --lockfile-only
vp install --filter web
vp install react
```

## Options

| Option                   | Description                                                                 | Notes                                                                                                |
| ------------------------ | --------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `-P, --prod`             | Select production dependencies or save added packages to dependencies.      |                                                                                                      |
| `-D, --dev`              | Include development dependencies or save added packages to devDependencies. |                                                                                                      |
| `--no-optional`          | Do not install optionalDependencies.                                        |                                                                                                      |
| `--frozen-lockfile`      | Prevent lockfile changes.                                                   | Overrides an earlier `--no-frozen-lockfile`.                                                         |
| `--no-frozen-lockfile`   | Explicitly allow lockfile updates.                                          | Overrides an earlier `--frozen-lockfile`.                                                            |
| `--lockfile-only`        | Update the lockfile without a full install.                                 |                                                                                                      |
| `--prefer-offline`       | Use cached packages when available.                                         |                                                                                                      |
| `--offline`              | Only use packages already in cache.                                         |                                                                                                      |
| `-f, --force`            | Request a forced reinstall.                                                 |                                                                                                      |
| `--ignore-scripts`       | Do not run lifecycle scripts.                                               |                                                                                                      |
| `--no-lockfile`          | Skip reading or generating a lockfile.                                      |                                                                                                      |
| `--fix-lockfile`         | Repair lockfile entries.                                                    | Cannot be combined with package names.                                                               |
| `--shamefully-hoist`     | Create a flat `node_modules` layout.                                        |                                                                                                      |
| `--resolution-only`      | Re-run dependency resolution for analysis.                                  | Cannot be combined with package names.                                                               |
| `--silent`               | Suppress Vite+ output and request native quiet output.                      |                                                                                                      |
| `--filter <pattern>`     | Select workspaces by pattern.                                               | Can be repeated.                                                                                     |
| `-w, --workspace-root`   | Select the workspace root.                                                  |                                                                                                      |
| `-E, --save-exact`       | Save an exact dependency version.                                           | Requires package names.                                                                              |
| `--save-peer`            | Request the manager's peer-dependency save behavior.                        | Requires package names.                                                                              |
| `-O, --save-optional`    | Save to optionalDependencies.                                               | Requires package names.                                                                              |
| `--save-catalog`         | Save the dependency to the default catalog.                                 | Requires package names.                                                                              |
| `-g, --global`           | Install globally.                                                           | Requires the global CLI. Uses the [global package store](./global-packages). Requires package names. |
| `--node <version>`       | Node.js version to use for global installation.                             | Requires --global.                                                                                   |
| `--concurrency <number>` | Number of global package installs to run in parallel.                       | Requires --global. Must be a positive integer.                                                       |

## Compatibility

The commands below describe project operations. Global mode is described under [Options](#options).

- **npm:** `npm install`, or `npm ci` with `--frozen-lockfile` and no package names. With package names: `npm install`.
- **pnpm:** `pnpm install`. With package names: `pnpm add`.
- **Yarn Classic:** `yarn install`. With package names: `yarn add`.
- **Yarn Modern:** `yarn install`. With package names: `yarn add`.
- **Bun:** `bun install`. With package names: `bun add`.

| Option                   | npm                       | pnpm                  | Yarn Classic          | Yarn Modern           | Bun                              | Notes                                              |
| ------------------------ | ------------------------- | --------------------- | --------------------- | --------------------- | -------------------------------- | -------------------------------------------------- |
| `-P, --prod`             | Yes                       | Yes                   | Yes                   | With package names    | Yes                              | Yarn Modern only warns in install mode.            |
| `-D, --dev`              | Yes                       | Yes                   | Yes                   | With package names    | Yes                              |                                                    |
| `--no-optional`          | Yes                       | Yes                   | Yes                   | No                    | Yes                              |                                                    |
| `--frozen-lockfile`      | Without package names     | Without package names | Yes                   | Without package names | Yes                              |                                                    |
| `--no-frozen-lockfile`   | Without package names     | Without package names | Without package names | Without package names | Yes                              |                                                    |
| `--lockfile-only`        | Without --frozen-lockfile | Yes                   | No                    | 3 or later            | Yes                              | npm ignores this in CI mode.                       |
| `--prefer-offline`       | Yes                       | Yes                   | Yes                   | No                    | 1.4.1 or later                   |                                                    |
| `--offline`              | Yes                       | Yes                   | Yes                   | No                    | 1.4.1 or later                   |                                                    |
| `-f, --force`            | Without --frozen-lockfile | Yes                   | Yes                   | No                    | Yes                              | npm ignores this in CI mode.                       |
| `--ignore-scripts`       | Yes                       | Yes                   | Yes                   | 2.4 or later          | Yes                              | On Yarn Modern, lockfile-only mode takes priority. |
| `--no-lockfile`          | Without --frozen-lockfile | Yes                   | Yes                   | No                    | No                               | npm ignores this in CI mode.                       |
| `--fix-lockfile`         | No                        | Yes                   | No                    | 4 or later            | No                               |                                                    |
| `--shamefully-hoist`     | No                        | Yes                   | No                    | No                    | No                               |                                                    |
| `--resolution-only`      | No                        | Yes                   | No                    | No                    | No                               |                                                    |
| `--silent`               | Yes                       | Yes                   | Yes                   | No                    | Yes                              |                                                    |
| `--filter <pattern>`     | 7.14.0 or later           | Yes                   | No                    | Yes                   | Install: Yes. Add: 1.4 or later. |                                                    |
| `-w, --workspace-root`   | 8.1.0 or later            | Yes                   | With package names    | No                    | No                               |                                                    |
| `-E, --save-exact`       | Yes                       | Yes                   | Yes                   | Yes                   | Yes                              |                                                    |
| `--save-peer`            | Yes                       | Yes                   | Yes                   | Yes                   | Yes                              |                                                    |
| `-O, --save-optional`    | Yes                       | Yes                   | Yes                   | Yes                   | Yes                              |                                                    |
| `--save-catalog`         | No                        | 10.12.0 or later      | No                    | No                    | 1.4 or later                     |                                                    |
| `-g, --global`           | Yes                       | Yes                   | Yes                   | Yes                   | Yes                              |                                                    |
| `--node <version>`       | Yes                       | Yes                   | Yes                   | Yes                   | Yes                              |                                                    |
| `--concurrency <number>` | Yes                       | Yes                   | Yes                   | Yes                   | Yes                              |                                                    |

With package names, [add](./add) compatibility rules apply.

### Git and Remote Tarball Dependencies

npm 12 disables git dependencies and remote tarball URLs by default. Installs can fail with `EALLOWGIT` or `EALLOWREMOTE`. Opt in through npm's project `allow-git` or `allow-remote` configuration only when those sources are intended.

For dependency build-script approval, see [approve-builds](./approve-builds).

## Native Options

For native options, see [npm install](https://docs.npmjs.com/cli/v11/commands/npm-install), [pnpm install](https://pnpm.io/cli/install), [Yarn install](https://yarnpkg.com/cli/install), [Yarn Classic install](https://classic.yarnpkg.com/en/docs/cli/install), and [Bun install](https://bun.com/docs/pm/cli/install). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
