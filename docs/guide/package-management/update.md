# vp update

Update project dependencies using the selected manager's update rules. Alias: `vp up`.

```bash
vp update
vp update react
vp update --latest
vp update -g
```

## Options

| Option                      | Description                                                                      | Notes                                                                        |
| --------------------------- | -------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| `-L, --latest`              | Update to latest version (ignore semver range).                                  | Global mode clears recorded version ranges.                                  |
| `-g, --global`              | Update global packages.                                                          | Requires the global CLI. Uses the [global package store](./global-packages). |
| `--concurrency <number>`    | Number of global package updates to run in parallel.                             | Requires --global. Must be a positive integer.                               |
| `--reinstall-node-mismatch` | Reinstall up-to-date global packages installed with a different Node.js version. | Requires --global.                                                           |
| `--ignore-node-mismatch`    | Skip up-to-date global packages installed with a different Node.js version.      | Requires --global.                                                           |
| `-r, --recursive`           | Update recursively in all workspace packages.                                    |                                                                              |
| `--filter <pattern>`        | Select workspaces by pattern.                                                    | Can be repeated.                                                             |
| `-w, --workspace-root`      | Include workspace root.                                                          |                                                                              |
| `-D, --dev`                 | Select development dependencies.                                                 |                                                                              |
| `-P, --prod`                | Select production dependencies.                                                  |                                                                              |
| `-i, --interactive`         | Interactive mode.                                                                |                                                                              |
| `--no-optional`             | Exclude optionalDependencies.                                                    |                                                                              |
| `--no-save`                 | Request native no-save behavior.                                                 |                                                                              |
| `--workspace`               | Update only dependencies available in the workspace.                             |                                                                              |

## Compatibility

The commands below describe project operations. Global mode is described under [Options](#options).

- **npm:** `npm update`.
- **pnpm:** `pnpm update`.
- **Yarn Classic:** `yarn upgrade` or `yarn upgrade-interactive`.
- **Yarn Modern:** `yarn up`.
- **Bun:** `bun update`.

| Option                      | npm              | pnpm | Yarn Classic | Yarn Modern                  | Bun          | Notes |
| --------------------------- | ---------------- | ---- | ------------ | ---------------------------- | ------------ | ----- |
| `-L, --latest`              | Global mode only | Yes  | Yes          | Yes                          | Yes          |       |
| `-g, --global`              | Yes              | Yes  | Yes          | Yes                          | Yes          |       |
| `--concurrency <number>`    | Yes              | Yes  | Yes          | Yes                          | Yes          |       |
| `--reinstall-node-mismatch` | Yes              | Yes  | Yes          | Yes                          | Yes          |       |
| `--ignore-node-mismatch`    | Yes              | Yes  | Yes          | Yes                          | Yes          |       |
| `-r, --recursive`           | 8.1.0 or later   | Yes  | No           | Yes                          | 1.4 or later |       |
| `--filter <pattern>`        | 7.14.0 or later  | Yes  | One filter   | No                           | 1.4 or later |       |
| `-w, --workspace-root`      | 8.1.0 or later   | Yes  | No           | No                           | No           |       |
| `-D, --dev`                 | Yes              | Yes  | No           | No                           | 1.4 or later |       |
| `-P, --prod`                | 7 or later       | Yes  | Yes          | No                           | 1.4 or later |       |
| `-i, --interactive`         | No               | Yes  | Yes          | Yes                          | Yes          |       |
| `--no-optional`             | Yes              | Yes  | Yes          | No                           | 1.4 or later |       |
| `--no-save`                 | Yes              | Yes  | No           | --recursive without --latest | Yes          |       |
| `--workspace`               | No               | Yes  | No           | No                           | No           |       |

To change a dependency's declared range on npm, use [add](./add) with a new version specification. Yarn Modern follows project-wide update semantics. Recursive mode without `--latest` re-resolves existing ranges without rewriting manifests; combining it with `--latest` uses ordinary `yarn up`.

## Native Options

For native options, see [npm update](https://docs.npmjs.com/cli/v11/commands/npm-update), [pnpm update](https://pnpm.io/cli/update), [Yarn up](https://yarnpkg.com/cli/up), [Yarn Classic upgrade](https://classic.yarnpkg.com/en/docs/cli/upgrade), [Yarn Classic upgrade-interactive](https://classic.yarnpkg.com/en/docs/cli/upgrade-interactive), and [Bun update](https://bun.com/docs/pm/cli/update). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
