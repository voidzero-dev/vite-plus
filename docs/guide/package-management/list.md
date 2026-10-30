# vp pm list

List installed dependencies. Alias: `vp pm ls`. The global CLI also accepts `vp list` and `vp ls`.

```bash
vp pm list
vp pm list react --depth 2
vp pm list --json
vp list -g
```

## Options

| Option                    | Description                       | Notes                                                                        |
| ------------------------- | --------------------------------- | ---------------------------------------------------------------------------- |
| `--depth <depth>`         | Maximum depth of dependency tree. |                                                                              |
| `--json`                  | Request native JSON output.       | Output retains the native schema.                                            |
| `--long`                  | Show extended information.        |                                                                              |
| `--parseable`             | Parseable output format.          |                                                                              |
| `-P, --prod`              | Only production dependencies.     |                                                                              |
| `-D, --dev`               | Include dev dependencies.         |                                                                              |
| `--no-optional`           | Exclude optional dependencies.    |                                                                              |
| `--exclude-peers`         | Exclude peer dependencies.        |                                                                              |
| `--only-projects`         | Show only project packages.       |                                                                              |
| `--find-by <finder_name>` | Use a finder function.            |                                                                              |
| `-r, --recursive`         | List across all workspaces.       |                                                                              |
| `--filter <pattern>`      | Select workspaces by pattern.     | Can be repeated.                                                             |
| `-g, --global`            | List global packages.             | Requires the global CLI. Uses the [global package store](./global-packages). |

## Compatibility

The commands below describe project operations. Global mode is described under [Options](#options).

- **npm:** `npm list`.
- **pnpm:** `pnpm list`.
- **Yarn Classic:** `yarn list`.
- **Yarn Modern:** No command. Warns and exits successfully without performing the operation.
- **Bun:** `bun pm ls`.

| Option                    | npm             | pnpm             | Yarn Classic | Yarn Modern | Bun | Notes                                                             |
| ------------------------- | --------------- | ---------------- | ------------ | ----------- | --- | ----------------------------------------------------------------- |
| `--depth <depth>`         | Yes             | Yes              | Yes          | No          | No  |                                                                   |
| `--json`                  | Yes             | Yes              | Yes          | No          | No  |                                                                   |
| `--long`                  | Yes             | Yes              | No           | No          | No  |                                                                   |
| `--parseable`             | Yes             | Yes              | No           | No          | No  |                                                                   |
| `-P, --prod`              | Yes             | Yes              | Yes          | No          | No  | npm 6 uses `--production`, which also excludes peer dependencies. |
| `-D, --dev`               | Yes             | Yes              | Yes          | No          | No  |                                                                   |
| `--no-optional`           | 7 or later      | Yes              | No           | No          | No  |                                                                   |
| `--exclude-peers`         | 7 or later      | Yes              | No           | No          | No  |                                                                   |
| `--only-projects`         | No              | Yes              | No           | No          | No  |                                                                   |
| `--find-by <finder_name>` | No              | 10.16.0 or later | No           | No          | No  |                                                                   |
| `-r, --recursive`         | 7.14.0 or later | Yes              | No           | No          | No  |                                                                   |
| `--filter <pattern>`      | 7.14.0 or later | Yes              | No           | No          | No  |                                                                   |
| `-g, --global`            | Yes             | Yes              | Yes          | Yes         | Yes |                                                                   |

These results describe installed packages, not Vite+'s bundled tools. Use `vp toolchain [tool]` to inspect bundled Vite, Rolldown, Oxc, and other tool versions.

## Native Options

For native options, see [npm ls](https://docs.npmjs.com/cli/v11/commands/npm-ls), [pnpm list](https://pnpm.io/cli/list), [Yarn Classic list](https://classic.yarnpkg.com/en/docs/cli/list), and [Bun pm ls](https://bun.com/docs/pm/cli/pm). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
