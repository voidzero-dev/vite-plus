# vp pm whoami

Show the account authenticated with the registry.

```bash
vp pm whoami
vp pm whoami --registry https://registry.npmjs.org
```

## Options

| Option             | Description          | Notes |
| ------------------ | -------------------- | ----- |
| `--registry <url>` | Select the registry. |       |

## Compatibility

- **npm:** `npm whoami`.
- **pnpm:** 11 or later: `pnpm whoami`. Earlier versions use `npm whoami` as a fallback.
- **Yarn Classic:** No command. Warns and exits successfully without performing the operation.
- **Yarn Modern:** `yarn npm whoami`.
- **Bun:** `bun pm whoami`.

| Option             | npm | pnpm | Yarn Classic | Yarn Modern | Bun | Notes                                           |
| ------------------ | --- | ---- | ------------ | ----------- | --- | ----------------------------------------------- |
| `--registry <url>` | Yes | Yes  | No           | No          | Yes | Configure the registry in Yarn Modern settings. |

## Native Options

For native options, see [npm whoami](https://docs.npmjs.com/cli/v11/commands/npm-whoami), [pnpm whoami](https://pnpm.io/cli/whoami), [Yarn npm whoami](https://yarnpkg.com/cli/npm/whoami), and [Bun pm whoami](https://bun.com/docs/pm/cli/pm). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
