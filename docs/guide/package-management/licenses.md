# vp pm licenses

List licenses reported for installed dependencies.

```bash
vp pm licenses list
vp pm licenses list --json
```

## Options

`list`, also spelled `ls`, is the supported subcommand. `--json` requests the package manager's native JSON output. Vite+ does not normalize the output into a shared schema.

### `list`

| Option   | Description                 | Notes                             |
| -------- | --------------------------- | --------------------------------- |
| `--json` | Request native JSON output. | Output retains the native schema. |

## Compatibility

### `list`

- **npm:** No command. Warns and exits successfully without performing the operation.
- **pnpm:** 7.17.0 or later: `pnpm licenses list`. Earlier versions warn and skip.
- **Yarn Classic:** `yarn licenses list`.
- **Yarn Modern:** No command. Warns and exits successfully without performing the operation.
- **Bun:** 1.4.0 or later: `bun pm licenses`. Earlier versions warn and skip.

| Option   | npm | pnpm            | Yarn Classic | Yarn Modern | Bun            | Notes |
| -------- | --- | --------------- | ------------ | ----------- | -------------- | ----- |
| `--json` | No  | 7.17.0 or later | Yes          | No          | 1.4.0 or later |       |

## Native Options

For example, a pnpm project can use:

```bash
vp pm licenses list -- --prod --long
```

For native options, see [pnpm licenses](https://pnpm.io/cli/licenses), [Yarn Classic licenses](https://classic.yarnpkg.com/en/docs/cli/licenses), and [Bun pm licenses](https://bun.com/docs/pm/cli/pm). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
