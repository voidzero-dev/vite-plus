# vp outdated

Check which dependencies have newer versions available.

```bash
vp outdated
vp outdated react
vp outdated --format json
vp outdated -g
```

## Options

| Option                         | Description                                         | Notes                                                                        |
| ------------------------------ | --------------------------------------------------- | ---------------------------------------------------------------------------- |
| `--long`                       | Show extended information.                          |                                                                              |
| `--format <table\|list\|json>` | Select the output format.                           |                                                                              |
| `-r, --recursive`              | Check recursively across all workspaces.            |                                                                              |
| `--filter <pattern>`           | Select workspaces by pattern.                       | Can be repeated.                                                             |
| `-w, --workspace-root`         | Include workspace root.                             |                                                                              |
| `-P, --prod`                   | Only production and optional dependencies.          |                                                                              |
| `-D, --dev`                    | Include dev dependencies.                           |                                                                              |
| `--no-optional`                | Exclude optional dependencies.                      |                                                                              |
| `--compatible`                 | Show only versions within compatible ranges.        |                                                                              |
| `--sort-by <field>`            | Sort results by field.                              |                                                                              |
| `-g, --global`                 | Check globally installed packages.                  | Requires the global CLI. Uses the [global package store](./global-packages). |
| `--concurrency <number>`       | Number of global package checks to run in parallel. | Requires --global. Must be a positive integer.                               |

## Compatibility

The commands below describe project operations. Global mode is described under [Options](#options).

- **npm:** `npm outdated`.
- **pnpm:** `pnpm outdated`.
- **Yarn Classic:** `yarn outdated`.
- **Yarn Modern:** Alternative: `yarn upgrade-interactive`.
- **Bun:** `bun outdated`.

| Option                         | npm               | pnpm              | Yarn Classic | Yarn Modern | Bun | Notes                                                                     |
| ------------------------------ | ----------------- | ----------------- | ------------ | ----------- | --- | ------------------------------------------------------------------------- |
| `--long`                       | Yes               | Yes               | No           | No          | No  |                                                                           |
| `--format <table\|list\|json>` | table, list, json | table, list, json | table, json  | No          | No  |                                                                           |
| `-r, --recursive`              | 7.14.0 or later   | Yes               | No           | No          | Yes | npm uses `--workspaces` to check workspaces, not transitive dependencies. |
| `--filter <pattern>`           | 7.14.0 or later   | Yes               | No           | No          | Yes |                                                                           |
| `-w, --workspace-root`         | 8.1.0 or later    | Yes               | No           | No          | No  |                                                                           |
| `-P, --prod`                   | 7 or later        | Yes               | No           | No          | Yes |                                                                           |
| `-D, --dev`                    | Yes               | Yes               | No           | No          | No  |                                                                           |
| `--no-optional`                | 7 or later        | Yes               | No           | No          | Yes |                                                                           |
| `--compatible`                 | No                | Yes               | No           | No          | No  |                                                                           |
| `--sort-by <field>`            | No                | Yes               | No           | No          | No  |                                                                           |
| `-g, --global`                 | Yes               | Yes               | Yes          | Yes         | Yes |                                                                           |
| `--concurrency <number>`       | Yes               | Yes               | Yes          | Yes         | Yes |                                                                           |

Yarn Modern does not forward positional package names. Its interactive updater can change dependencies, unlike a read-only outdated query.

Native output and exit codes differ. A nonzero exit can indicate outdated dependencies rather than a failure to query the registry.

## Native Options

For native options, see [npm outdated](https://docs.npmjs.com/cli/v11/commands/npm-outdated), [pnpm outdated](https://pnpm.io/cli/outdated), [Yarn upgrade-interactive](https://yarnpkg.com/cli/upgrade-interactive), [Yarn Classic outdated](https://classic.yarnpkg.com/en/docs/cli/outdated), and [Bun outdated](https://bun.com/docs/pm/cli/outdated). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
