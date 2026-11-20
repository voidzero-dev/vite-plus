# vp pm config

Read and write package-manager configuration. Alias: `vp pm c`. This is distinct from `vp config`, which configures project tooling.

```bash
vp pm config list
vp pm config get registry
vp pm config set registry https://registry.npmjs.org --location project
vp pm config delete registry --location project
```

Configuration keys are manager-specific. The registry examples use npm or pnpm keys, not Yarn 2 configuration keys.

## Options

- `list` displays configuration.
- `get <key>` reads a value.
- `set <key> <value>` writes a value.
- `delete <key>` removes a value.

Here `--global` selects configuration scope, not Vite+'s global package store.

These subcommands do not accept additional native arguments after `--`.

### `list`

| Option                               | Description                     | Notes                             |
| ------------------------------------ | ------------------------------- | --------------------------------- |
| `--json`                             | Request native JSON output.     | Output retains the native schema. |
| `-g, --global`                       | Use global config.              | Overrides `--location`.           |
| `--location <project\|user\|global>` | Choose the configuration scope. |                                   |

### `get`

| Option                               | Description                     | Notes                             |
| ------------------------------------ | ------------------------------- | --------------------------------- |
| `--json`                             | Request native JSON output.     | Output retains the native schema. |
| `-g, --global`                       | Use global config.              | Overrides `--location`.           |
| `--location <project\|user\|global>` | Choose the configuration scope. |                                   |

### `set`

| Option                               | Description                     | Notes                             |
| ------------------------------------ | ------------------------------- | --------------------------------- |
| `--json`                             | Request native JSON output.     | Output retains the native schema. |
| `-g, --global`                       | Use global config.              | Overrides `--location`.           |
| `--location <project\|user\|global>` | Choose the configuration scope. |                                   |

### `delete`

| Option                               | Description                     | Notes                   |
| ------------------------------------ | ------------------------------- | ----------------------- |
| `-g, --global`                       | Use global config.              | Overrides `--location`. |
| `--location <project\|user\|global>` | Choose the configuration scope. |                         |

## Compatibility

### `list`

- **npm:** `npm config list`.
- **pnpm:** `pnpm config list`.
- **Yarn Classic:** `yarn config list`.
- **Yarn Modern:** `yarn config`.
- **Bun:** Fallback: `npm config list`. Uses npm configuration, not `bunfig.toml`.

| Option                               | npm                   | pnpm                  | Yarn Classic | Yarn Modern           | Bun                   | Notes                              |
| ------------------------------------ | --------------------- | --------------------- | ------------ | --------------------- | --------------------- | ---------------------------------- |
| `--json`                             | Yes                   | Yes                   | Yes          | Yes                   | Yes                   |                                    |
| `-g, --global`                       | Yes                   | Yes                   | Yes          | Yes                   | Yes                   |                                    |
| `--location <project\|user\|global>` | project, user, global | project, user, global | user, global | project, user, global | project, user, global | Reads retain merged configuration. |

### `get`

- **npm:** `npm config get`.
- **pnpm:** `pnpm config get`.
- **Yarn Classic:** `yarn config get`.
- **Yarn Modern:** `yarn config get`.
- **Bun:** Fallback: `npm config get`. Uses npm configuration, not `bunfig.toml`.

| Option                               | npm                   | pnpm                  | Yarn Classic | Yarn Modern           | Bun                   | Notes                              |
| ------------------------------------ | --------------------- | --------------------- | ------------ | --------------------- | --------------------- | ---------------------------------- |
| `--json`                             | Yes                   | Yes                   | Yes          | Yes                   | Yes                   |                                    |
| `-g, --global`                       | Yes                   | Yes                   | Yes          | Yes                   | Yes                   |                                    |
| `--location <project\|user\|global>` | project, user, global | project, user, global | user, global | project, user, global | project, user, global | Reads retain merged configuration. |

### `set`

- **npm:** `npm config set`.
- **pnpm:** `pnpm config set`.
- **Yarn Classic:** `yarn config set`.
- **Yarn Modern:** `yarn config set`.
- **Bun:** Fallback: `npm config set`. Uses npm configuration, not `bunfig.toml`.

| Option                               | npm                   | pnpm                  | Yarn Classic | Yarn Modern           | Bun                   | Notes |
| ------------------------------------ | --------------------- | --------------------- | ------------ | --------------------- | --------------------- | ----- |
| `--json`                             | Yes                   | Yes                   | Yes          | Yes                   | Yes                   |       |
| `-g, --global`                       | Yes                   | Yes                   | Yes          | Yes                   | Yes                   |       |
| `--location <project\|user\|global>` | project, user, global | project, user, global | user, global | project, user, global | project, user, global |       |

### `delete`

- **npm:** `npm config delete`.
- **pnpm:** `pnpm config delete`.
- **Yarn Classic:** `yarn config delete`.
- **Yarn Modern:** `yarn config unset`.
- **Bun:** Fallback: `npm config delete`. Uses npm configuration, not `bunfig.toml`.

| Option                               | npm                   | pnpm                  | Yarn Classic | Yarn Modern           | Bun                   | Notes |
| ------------------------------------ | --------------------- | --------------------- | ------------ | --------------------- | --------------------- | ----- |
| `-g, --global`                       | Yes                   | Yes                   | Yes          | Yes                   | Yes                   |       |
| `--location <project\|user\|global>` | project, user, global | project, user, global | user, global | project, user, global | project, user, global |       |

Yarn Classic user writes use native defaults.

## Native Options

For native options, see [npm config](https://docs.npmjs.com/cli/v11/commands/npm-config), [pnpm config](https://pnpm.io/cli/config), [Yarn config](https://yarnpkg.com/cli/config), [Yarn config get](https://yarnpkg.com/cli/config/get), [Yarn config set](https://yarnpkg.com/cli/config/set), [Yarn config unset](https://yarnpkg.com/cli/config/unset), and [Yarn Classic config](https://classic.yarnpkg.com/en/docs/cli/config). Native options outside this interface require the native command.

For `bunfig.toml` settings, see [Bun configuration](https://bun.com/docs/runtime/bunfig).
