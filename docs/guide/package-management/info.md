# vp info

Read package metadata from the registry. Aliases: `vp view`, `vp show`, `vp pm view`, `vp pm info`, and `vp pm show`.

```bash
vp info react
vp info react versions --json
vp info react@19 dist.tarball
```

## Options

Supply a package name with an optional version, followed by an optional field name.

| Option   | Description                 | Notes                             |
| -------- | --------------------------- | --------------------------------- |
| `--json` | Request native JSON output. | Output retains the native schema. |

## Compatibility

- **npm:** `npm view`.
- **pnpm:** `pnpm view`.
- **Yarn Classic:** `yarn info`.
- **Yarn Modern:** `yarn npm info`. A requested field is passed as `--fields`.
- **Bun:** `bun info`.

| Option   | npm | pnpm | Yarn Classic | Yarn Modern | Bun | Notes |
| -------- | --- | ---- | ------------ | ----------- | --- | ----- |
| `--json` | Yes | Yes  | Yes          | Yes         | Yes |       |

This command queries registry metadata, not the local dependency tree. Use [list](./list) or [why](./why) to inspect installed packages.

## Native Options

For native options, see [npm view](https://docs.npmjs.com/cli/v11/commands/npm-view), [pnpm view](https://pnpm.io/cli/view), [Yarn npm info](https://yarnpkg.com/cli/npm/info), [Yarn Classic info](https://classic.yarnpkg.com/en/docs/cli/info), and [Bun info](https://bun.com/docs/pm/cli/info). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
