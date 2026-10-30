# vp pm team

Manage registry organization teams and their membership.

```bash
vp pm team list my-org
vp pm team list my-org:developers --json
vp pm team create my-org:developers
vp pm team add my-org:developers alice
vp pm team rm my-org:developers alice
vp pm team destroy my-org:developers
```

## Options

- `create <scope:team>` creates a team.
- `destroy <scope:team>` deletes a team.
- `add <scope:team> <user>` adds a member.
- `rm <scope:team> <user>` removes a member. Alias: `remove`.
- `list <scope[:team]>` lists an organization's teams or a team's members. Alias: `ls`.

All subcommands accept `--registry <url>`, `--otp <code>`, and `--json`. JSON output retains the native manager's schema. Registry permissions determine which changes the authenticated account may make.

### `create`

| Option             | Description                 | Notes                             |
| ------------------ | --------------------------- | --------------------------------- |
| `--otp <otp>`      | Supply a one-time password. |                                   |
| `--json`           | Request native JSON output. | Output retains the native schema. |
| `--registry <url>` | Select the registry.        |                                   |

### `destroy`

| Option             | Description                 | Notes                             |
| ------------------ | --------------------------- | --------------------------------- |
| `--otp <otp>`      | Supply a one-time password. |                                   |
| `--json`           | Request native JSON output. | Output retains the native schema. |
| `--registry <url>` | Select the registry.        |                                   |

### `add`

| Option             | Description                 | Notes                             |
| ------------------ | --------------------------- | --------------------------------- |
| `--otp <otp>`      | Supply a one-time password. |                                   |
| `--json`           | Request native JSON output. | Output retains the native schema. |
| `--registry <url>` | Select the registry.        |                                   |

### `rm`

| Option             | Description                 | Notes                             |
| ------------------ | --------------------------- | --------------------------------- |
| `--otp <otp>`      | Supply a one-time password. |                                   |
| `--json`           | Request native JSON output. | Output retains the native schema. |
| `--registry <url>` | Select the registry.        |                                   |

### `list`

| Option             | Description                 | Notes                             |
| ------------------ | --------------------------- | --------------------------------- |
| `--otp <otp>`      | Supply a one-time password. |                                   |
| `--json`           | Request native JSON output. | Output retains the native schema. |
| `--registry <url>` | Select the registry.        |                                   |

## Compatibility

Fallbacks to npm emit warnings.

### `create`

- **npm:** `npm team create`.
- **pnpm:** 11.13.0 or later: `pnpm team create`. Earlier versions use `npm team create` as a fallback.
- **Yarn Classic:** `yarn team create`.
- **Yarn Modern:** No command. Warns and exits successfully without performing the operation.
- **Bun:** Fallback: `npm team create`.

| Option             | npm | pnpm | Yarn Classic    | Yarn Modern | Bun | Notes                                                                       |
| ------------------ | --- | ---- | --------------- | ----------- | --- | --------------------------------------------------------------------------- |
| `--otp <otp>`      | Yes | Yes  | 1.12.1 or later | No          | Yes | Older Yarn Classic can ignore --otp. Vite+ does not check its version here. |
| `--json`           | Yes | Yes  | Yes             | No          | Yes |                                                                             |
| `--registry <url>` | Yes | Yes  | Yes             | No          | Yes |                                                                             |

### `destroy`

- **npm:** `npm team destroy`.
- **pnpm:** 11.13.0 or later: `pnpm team destroy`. Earlier versions use `npm team destroy` as a fallback.
- **Yarn Classic:** `yarn team destroy`.
- **Yarn Modern:** No command. Warns and exits successfully without performing the operation.
- **Bun:** Fallback: `npm team destroy`.

| Option             | npm | pnpm | Yarn Classic    | Yarn Modern | Bun | Notes                                                                       |
| ------------------ | --- | ---- | --------------- | ----------- | --- | --------------------------------------------------------------------------- |
| `--otp <otp>`      | Yes | Yes  | 1.12.1 or later | No          | Yes | Older Yarn Classic can ignore --otp. Vite+ does not check its version here. |
| `--json`           | Yes | Yes  | Yes             | No          | Yes |                                                                             |
| `--registry <url>` | Yes | Yes  | Yes             | No          | Yes |                                                                             |

### `add`

- **npm:** `npm team add`.
- **pnpm:** 11.13.0 or later: `pnpm team add`. Earlier versions use `npm team add` as a fallback.
- **Yarn Classic:** `yarn team add`.
- **Yarn Modern:** No command. Warns and exits successfully without performing the operation.
- **Bun:** Fallback: `npm team add`.

| Option             | npm | pnpm | Yarn Classic    | Yarn Modern | Bun | Notes                                                                       |
| ------------------ | --- | ---- | --------------- | ----------- | --- | --------------------------------------------------------------------------- |
| `--otp <otp>`      | Yes | Yes  | 1.12.1 or later | No          | Yes | Older Yarn Classic can ignore --otp. Vite+ does not check its version here. |
| `--json`           | Yes | Yes  | Yes             | No          | Yes |                                                                             |
| `--registry <url>` | Yes | Yes  | Yes             | No          | Yes |                                                                             |

### `rm`

- **npm:** `npm team rm`.
- **pnpm:** 11.13.0 or later: `pnpm team rm`. Earlier versions use `npm team rm` as a fallback.
- **Yarn Classic:** `yarn team remove`.
- **Yarn Modern:** No command. Warns and exits successfully without performing the operation.
- **Bun:** Fallback: `npm team rm`.

| Option             | npm | pnpm | Yarn Classic    | Yarn Modern | Bun | Notes                                                                       |
| ------------------ | --- | ---- | --------------- | ----------- | --- | --------------------------------------------------------------------------- |
| `--otp <otp>`      | Yes | Yes  | 1.12.1 or later | No          | Yes | Older Yarn Classic can ignore --otp. Vite+ does not check its version here. |
| `--json`           | Yes | Yes  | Yes             | No          | Yes |                                                                             |
| `--registry <url>` | Yes | Yes  | Yes             | No          | Yes |                                                                             |

### `list`

- **npm:** `npm team ls`.
- **pnpm:** 11.13.0 or later: `pnpm team ls`. Earlier versions use `npm team ls` as a fallback.
- **Yarn Classic:** `yarn team list`.
- **Yarn Modern:** No command. Warns and exits successfully without performing the operation.
- **Bun:** Fallback: `npm team ls`.

| Option             | npm | pnpm | Yarn Classic    | Yarn Modern | Bun | Notes                                                                       |
| ------------------ | --- | ---- | --------------- | ----------- | --- | --------------------------------------------------------------------------- |
| `--otp <otp>`      | Yes | Yes  | 1.12.1 or later | No          | Yes | Older Yarn Classic can ignore --otp. Vite+ does not check its version here. |
| `--json`           | Yes | Yes  | Yes             | No          | Yes |                                                                             |
| `--registry <url>` | Yes | Yes  | Yes             | No          | Yes |                                                                             |

## Native Options

For native options, see [npm team](https://docs.npmjs.com/cli/v11/commands/npm-team), [pnpm team](https://pnpm.io/cli/team), and [Yarn Classic team](https://classic.yarnpkg.com/en/docs/cli/team). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
