# vp unlink

Remove native package-manager links created for local development.

```bash
vp unlink my-library
```

## Options

An optional package name or directory is forwarded to the manager. `-r, --recursive` requests unlinking across workspaces where supported.

| Option            | Description                        | Notes |
| ----------------- | ---------------------------------- | ----- |
| `-r, --recursive` | Unlink in every workspace package. |       |

## Compatibility

- **npm:** `npm unlink`.
- **pnpm:** `pnpm unlink`.
- **Yarn Classic:** `yarn unlink`.
- **Yarn Modern:** 3 or later: `yarn unlink`. Yarn 2 fails natively.
- **Bun:** `bun unlink`.

| Option            | npm            | pnpm | Yarn Classic | Yarn Modern | Bun | Notes                            |
| ----------------- | -------------- | ---- | ------------ | ----------- | --- | -------------------------------- |
| `-r, --recursive` | 8.1.0 or later | Yes  | No           | 3 or later  | No  | npm includes the workspace root. |

Vite+ forwards a basic unlink request to the selected manager, so unsupported native versions can fail. Operand and manifest-update behavior follow the native manager. This does not remove packages from Vite+'s [global store](./global-packages).

## Native Options

For native options, see [npm unlink](https://docs.npmjs.com/cli/v11/commands/npm-uninstall), [pnpm unlink](https://pnpm.io/cli/unlink), [Yarn unlink](https://yarnpkg.com/cli/unlink), [Yarn Classic unlink](https://classic.yarnpkg.com/en/docs/cli/unlink), and [Bun unlink](https://bun.com/docs/pm/cli/link). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
