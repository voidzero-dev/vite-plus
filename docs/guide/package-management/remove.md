# vp remove

Remove dependencies from the project. Aliases: `vp rm`, `vp un`, and `vp uninstall`.

```bash
vp remove react
vp remove --filter web react
vp remove -g --dry-run typescript
```

## Options

With npm, `-D`, `-O`, and `-P` do not limit removal to the selected dependency section.

| Option                 | Description                                              | Notes                                                                        |
| ---------------------- | -------------------------------------------------------- | ---------------------------------------------------------------------------- |
| `-D, --save-dev`       | On pnpm, target `devDependencies`.                       |                                                                              |
| `-O, --save-optional`  | On pnpm, target `optionalDependencies`.                  |                                                                              |
| `-P, --save-prod`      | On pnpm, target `dependencies`.                          |                                                                              |
| `--filter <pattern>`   | Select workspaces by pattern.                            | Can be repeated.                                                             |
| `-w, --workspace-root` | Remove from workspace root.                              |                                                                              |
| `-r, --recursive`      | Remove recursively from all workspace packages.          |                                                                              |
| `-g, --global`         | Remove global packages.                                  | Requires the global CLI. Uses the [global package store](./global-packages). |
| `--dry-run`            | Preview what would be removed without actually removing. | Requires --global.                                                           |

## Compatibility

The commands below describe project operations. Global mode is described under [Options](#options).

- **npm:** `npm uninstall`.
- **pnpm:** `pnpm remove`.
- **Yarn Classic:** `yarn remove`.
- **Yarn Modern:** `yarn remove`.
- **Bun:** `bun remove`.

| Option                 | npm             | pnpm | Yarn Classic | Yarn Modern | Bun          | Notes                            |
| ---------------------- | --------------- | ---- | ------------ | ----------- | ------------ | -------------------------------- |
| `-D, --save-dev`       | Yes             | Yes  | No           | No          | No           | npm can remove across sections.  |
| `-O, --save-optional`  | Yes             | Yes  | No           | No          | No           | npm can remove across sections.  |
| `-P, --save-prod`      | Yes             | Yes  | No           | No          | No           | npm can remove across sections.  |
| `--filter <pattern>`   | 7.14.0 or later | Yes  | One filter   | Yes         | 1.4 or later |                                  |
| `-w, --workspace-root` | 8.1.0 or later  | Yes  | No           | No          | No           |                                  |
| `-r, --recursive`      | 8.1.0 or later  | Yes  | No           | Yes         | No           | npm includes the workspace root. |
| `-g, --global`         | Yes             | Yes  | Yes          | Yes         | Yes          |                                  |
| `--dry-run`            | Yes             | Yes  | Yes          | Yes         | Yes          |                                  |

When pnpm removes a dependency, it can also remove the same package from `peerDependencies`.

With npm, if `package.json` lists `vite` only in `dependencies`, `vp remove -D vite` removes it from `dependencies`.

## Native Options

For native options, see [npm uninstall](https://docs.npmjs.com/cli/v11/commands/npm-uninstall), [pnpm remove](https://pnpm.io/cli/remove), [Yarn remove](https://yarnpkg.com/cli/remove), [Yarn Classic remove](https://classic.yarnpkg.com/en/docs/cli/remove), and [Bun remove](https://bun.com/docs/pm/cli/remove). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
