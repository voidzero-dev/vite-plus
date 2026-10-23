# vp pm prune

Remove unnecessary installed packages.

```bash
vp pm prune
vp pm prune --prod
```

## Options

| Option          | Description                   | Notes |
| --------------- | ----------------------------- | ----- |
| `--prod`        | Remove devDependencies.       |       |
| `--no-optional` | Remove optional dependencies. |       |

## Compatibility

- **npm:** `npm prune`.
- **pnpm:** `pnpm prune`.
- **Yarn Classic:** No command. Warns and exits successfully without performing the operation.
- **Yarn Modern:** No command. Warns and exits successfully without performing the operation.
- **Bun:** 1.4 or later: `bun prune`. Earlier versions warn and skip.

| Option          | npm | pnpm | Yarn Classic | Yarn Modern | Bun          | Notes |
| --------------- | --- | ---- | ------------ | ----------- | ------------ | ----- |
| `--prod`        | Yes | Yes  | No           | No          | 1.4 or later |       |
| `--no-optional` | Yes | Yes  | No           | No          | 1.4 or later |       |

npm translates selection flags to `--omit=dev` and `--omit=optional`. Bun translates `--prod` to `--production`. Yarn and older Bun remove extraneous packages during installation instead of providing a native prune command.

## Native Options

For native options, see [npm prune](https://docs.npmjs.com/cli/v11/commands/npm-prune), [pnpm prune](https://pnpm.io/cli/prune), and [Bun prune](https://bun.com/docs/pm/cli/prune). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
