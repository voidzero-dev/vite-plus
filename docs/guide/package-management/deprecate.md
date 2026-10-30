# vp pm deprecate

Set a deprecation message for a published package version or version range.

```bash
vp pm deprecate 'my-package@<2.0.0' 'Upgrade to version 2 or later'
```

## Options

A package specification and a message are required.

Deprecation warns consumers; it does not unpublish the package.

| Option             | Description                 | Notes |
| ------------------ | --------------------------- | ----- |
| `--otp <otp>`      | Supply a one-time password. |       |
| `--registry <url>` | Select the registry.        |       |

## Compatibility

- **npm:** `npm deprecate`.
- **pnpm:** 11 or later: `pnpm deprecate`. Earlier versions use `npm deprecate` as a fallback.
- **Yarn Classic:** Fallback: `npm deprecate`.
- **Yarn Modern:** Fallback: `npm deprecate`.
- **Bun:** Fallback: `npm deprecate`.

| Option             | npm | pnpm | Yarn Classic | Yarn Modern | Bun | Notes |
| ------------------ | --- | ---- | ------------ | ----------- | --- | ----- |
| `--otp <otp>`      | Yes | Yes  | Yes          | Yes         | Yes |       |
| `--registry <url>` | Yes | Yes  | Yes          | Yes         | Yes |       |

Bun emits a fallback warning. Registry configuration and authentication follow the command that runs.

An empty message is forwarded unchanged. Native pnpm rejects it; Vite+ does not translate it to `undeprecate`.

## Native Options

For native options, see [npm deprecate](https://docs.npmjs.com/cli/v11/commands/npm-deprecate) and [pnpm deprecate](https://pnpm.io/cli/deprecate). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
