# vp pm owner

Manage package owners. Alias: `vp pm author`.

```bash
vp pm owner list my-package
vp pm owner add alice my-package
vp pm owner rm alice my-package
```

## Options

- `list <package>` lists owners. Alias: `ls`.
- `add <user> <package>` grants ownership.
- `rm <user> <package>` removes ownership.

Each subcommand accepts `--otp <code>`. These subcommands do not accept additional native arguments after `--`. Ownership changes affect registry permissions, not the local manifest's author field.

### `list`

| Option        | Description                 | Notes |
| ------------- | --------------------------- | ----- |
| `--otp <otp>` | Supply a one-time password. |       |

### `add`

| Option        | Description                 | Notes |
| ------------- | --------------------------- | ----- |
| `--otp <otp>` | Supply a one-time password. |       |

### `rm`

| Option        | Description                 | Notes |
| ------------- | --------------------------- | ----- |
| `--otp <otp>` | Supply a one-time password. |       |

## Compatibility

### `list`

- **npm:** `npm owner list`.
- **pnpm:** 11.1 or later: `pnpm owner list`. Earlier versions use `npm owner list` as a fallback.
- **Yarn Classic:** `yarn owner list`.
- **Yarn Modern:** Fallback: `npm owner list`.
- **Bun:** Fallback: `npm owner list`.

| Option        | npm | pnpm | Yarn Classic    | Yarn Modern | Bun | Notes |
| ------------- | --- | ---- | --------------- | ----------- | --- | ----- |
| `--otp <otp>` | Yes | Yes  | 1.12.1 or later | Yes         | Yes |       |

### `add`

- **npm:** `npm owner add`.
- **pnpm:** 11.1 or later: `pnpm owner add`. Earlier versions use `npm owner add` as a fallback. Native pnpm takes the package before the user.
- **Yarn Classic:** `yarn owner add`. Keeps the user before the package.
- **Yarn Modern:** Fallback: `npm owner add`.
- **Bun:** Fallback: `npm owner add`.

| Option        | npm | pnpm | Yarn Classic    | Yarn Modern | Bun | Notes |
| ------------- | --- | ---- | --------------- | ----------- | --- | ----- |
| `--otp <otp>` | Yes | Yes  | 1.12.1 or later | Yes         | Yes |       |

### `rm`

- **npm:** `npm owner rm`.
- **pnpm:** 11.1 or later: `pnpm owner rm`. Earlier versions use `npm owner rm` as a fallback. Native pnpm takes the package before the user.
- **Yarn Classic:** `yarn owner remove`. Keeps the user before the package. Removal can fail in Yarn Classic 1.22.22 because of an upstream bug. Vite+ does not retry through npm.
- **Yarn Modern:** Fallback: `npm owner rm`.
- **Bun:** Fallback: `npm owner rm`.

| Option        | npm | pnpm | Yarn Classic    | Yarn Modern | Bun | Notes |
| ------------- | --- | ---- | --------------- | ----------- | --- | ----- |
| `--otp <otp>` | Yes | Yes  | 1.12.1 or later | Yes         | Yes |       |

Registry credentials and authorization follow the command that runs.

## Native Options

For registry requirements and native options, see [npm owner](https://docs.npmjs.com/cli/v11/commands/npm-owner), [pnpm owner](https://pnpm.io/cli/owner), and [Yarn Classic owner](https://classic.yarnpkg.com/en/docs/cli/owner). Options not exposed above must be used with the native command directly.
