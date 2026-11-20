# vp pm login

Authenticate with a package registry. Alias: `vp pm adduser`.

```bash
vp pm login
vp pm login --scope @my-org
```

## Options

| Option             | Description          | Notes |
| ------------------ | -------------------- | ----- |
| `--registry <url>` | Select the registry. |       |
| `--scope <scope>`  | Scope for the login. |       |

## Compatibility

- **npm:** `npm login`.
- **pnpm:** 11 or later: `pnpm login`. Earlier versions use `npm login` as a fallback.
- **Yarn Classic:** `yarn login`.
- **Yarn Modern:** `yarn npm login`.
- **Bun:** Fallback: `npm login`.

| Option             | npm | pnpm                       | Yarn Classic | Yarn Modern | Bun | Notes                                    |
| ------------------ | --- | -------------------------- | ------------ | ----------- | --- | ---------------------------------------- |
| `--registry <url>` | Yes | Yes                        | No           | No          | Yes | Configure the registry in Yarn settings. |
| `--scope <scope>`  | Yes | Before 11 or 11.2 or later | No           | Yes         | Yes |                                          |

Authentication prompts and credential storage belong to the native command. pnpm login does not copy credentials to npm, so commands that still use npm may require a separate login. See [whoami](./whoami) to check the selected account and [logout](./logout) to end authentication.

## Native Options

For native options, see [npm login](https://docs.npmjs.com/cli/v11/commands/npm-login), [pnpm login](https://pnpm.io/cli/login), [Yarn npm login](https://yarnpkg.com/cli/npm/login), and [Yarn Classic login](https://classic.yarnpkg.com/en/docs/cli/login). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
