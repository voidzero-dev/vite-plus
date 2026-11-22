# vp pm fund

Show funding information for installed dependencies.

```bash
vp pm fund
vp pm fund --json
```

## Options

| Option   | Description                 | Notes                             |
| -------- | --------------------------- | --------------------------------- |
| `--json` | Request native JSON output. | Output retains the native schema. |

## Compatibility

- **npm:** `npm fund`.
- **pnpm:** Fallback: `npm fund`.
- **Yarn Classic:** Fallback: `npm fund`.
- **Yarn Modern:** Fallback: `npm fund`.
- **Bun:** Fallback: `npm fund`.

| Option   | npm | pnpm | Yarn Classic | Yarn Modern | Bun | Notes |
| -------- | --- | ---- | ------------ | ----------- | --- | ----- |
| `--json` | Yes | Yes  | Yes          | Yes         | Yes |       |

Bun emits a fallback warning. Results follow npm's view of the installed dependencies.

## Native Options

For native options, see [npm fund](https://docs.npmjs.com/cli/v11/commands/npm-fund), which Vite+ uses for all package managers. To pass these options through Vite+, see [Native Arguments](./#native-arguments).
