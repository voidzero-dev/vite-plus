# vp pm token

Manage registry authentication tokens.

```bash
vp pm token list
vp pm token list --json
vp pm token revoke <token-id>
```

## Options

- `list`, also spelled `ls`, lists tokens. Supports `--json` and `--registry <url>`.
- `create` requests a new token. Supports `--json`, `--registry <url>`, `--readonly`, and repeatable `--cidr <range>` restrictions.
- `revoke <token-or-id>` revokes a token. Supports `--registry <url>`.

Treat tokens as secrets. Prefer a token ID for revocation rather than placing a raw token in shell history. Token creation and supported token types depend on the registry's current policy and the native package manager version; exposing `create` does not guarantee that a registry permits CLI token creation.

### `list`

| Option             | Description                 | Notes                             |
| ------------------ | --------------------------- | --------------------------------- |
| `--json`           | Request native JSON output. | Output retains the native schema. |
| `--registry <url>` | Select the registry.        |                                   |

### `create`

| Option             | Description                           | Notes                             |
| ------------------ | ------------------------------------- | --------------------------------- |
| `--json`           | Request native JSON output.           | Output retains the native schema. |
| `--registry <url>` | Select the registry.                  |                                   |
| `--cidr <cidr>`    | CIDR ranges to restrict the token to. | Can be repeated.                  |
| `--readonly`       | Create a read-only token.             |                                   |

### `revoke`

| Option             | Description          | Notes |
| ------------------ | -------------------- | ----- |
| `--registry <url>` | Select the registry. |       |

## Compatibility

### `list`

- **npm:** `npm token list`.
- **pnpm:** 12.13 or later: `pnpm token list`. Earlier versions use `npm token list` as a fallback.
- **Yarn Classic:** Fallback: `npm token list`.
- **Yarn Modern:** Fallback: `npm token list`.
- **Bun:** Fallback: `npm token list`.

| Option             | npm | pnpm | Yarn Classic | Yarn Modern | Bun | Notes |
| ------------------ | --- | ---- | ------------ | ----------- | --- | ----- |
| `--json`           | Yes | Yes  | Yes          | Yes         | Yes |       |
| `--registry <url>` | Yes | Yes  | Yes          | Yes         | Yes |       |

### `create`

- **npm:** `npm token create`.
- **pnpm:** 12.13 or later: `pnpm token create`. Earlier versions use `npm token create` as a fallback.
- **Yarn Classic:** Fallback: `npm token create`.
- **Yarn Modern:** Fallback: `npm token create`.
- **Bun:** Fallback: `npm token create`.

| Option             | npm | pnpm | Yarn Classic | Yarn Modern | Bun | Notes |
| ------------------ | --- | ---- | ------------ | ----------- | --- | ----- |
| `--json`           | Yes | Yes  | Yes          | Yes         | Yes |       |
| `--registry <url>` | Yes | Yes  | Yes          | Yes         | Yes |       |
| `--cidr <cidr>`    | Yes | Yes  | Yes          | Yes         | Yes |       |
| `--readonly`       | Yes | Yes  | Yes          | Yes         | Yes |       |

### `revoke`

- **npm:** `npm token revoke`.
- **pnpm:** 12.13 or later: `pnpm token revoke`. Earlier versions use `npm token revoke` as a fallback.
- **Yarn Classic:** Fallback: `npm token revoke`.
- **Yarn Modern:** Fallback: `npm token revoke`.
- **Bun:** Fallback: `npm token revoke`.

| Option             | npm | pnpm | Yarn Classic | Yarn Modern | Bun | Notes |
| ------------------ | --- | ---- | ------------ | ----------- | --- | ----- |
| `--registry <url>` | Yes | Yes  | Yes          | Yes         | Yes |       |

`revoke` accepts one token or ID. Authentication, password prompts, and token resolution follow the native command. Vite+ does not normalize token types or output schemas.

## Native Options

For native options, see [npm token](https://docs.npmjs.com/cli/v11/commands/npm-token) and [pnpm token](https://pnpm.io/cli/token). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
