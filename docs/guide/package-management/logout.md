# vp pm logout

Log out from a package registry.

```bash
vp pm logout
vp pm logout --scope @my-org
```

## Options

| Option             | Description           | Notes |
| ------------------ | --------------------- | ----- |
| `--registry <url>` | Select the registry.  |       |
| `--scope <scope>`  | Scope for the logout. |       |

## Compatibility

- **npm:** `npm logout`.
- **pnpm:** 11 or later: `pnpm logout`. Earlier versions use `npm logout` as a fallback.
- **Yarn Classic:** `yarn logout`.
- **Yarn Modern:** `yarn npm logout`.
- **Bun:** Fallback: `npm logout`.

| Option             | npm | pnpm      | Yarn Classic | Yarn Modern | Bun | Notes                                    |
| ------------------ | --- | --------- | ------------ | ----------- | --- | ---------------------------------------- |
| `--registry <url>` | Yes | Yes       | No           | No          | Yes | Configure the registry in Yarn settings. |
| `--scope <scope>`  | Yes | Before 11 | No           | Yes         | Yes |                                          |

Credential removal and token invalidation follow the native manager and registry. Native pnpm logout does not run npm logout or perform additional credential cleanup. Use [login](./login) to authenticate again.

## Native Options

For native options, see [npm logout](https://docs.npmjs.com/cli/v11/commands/npm-logout), [pnpm logout](https://pnpm.io/cli/logout), [Yarn npm logout](https://yarnpkg.com/cli/npm/logout), and [Yarn Classic logout](https://classic.yarnpkg.com/en/docs/cli/logout). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
