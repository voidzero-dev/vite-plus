# vp pm search

Search the package registry.

```bash
vp pm search vite plugin
vp pm search typescript --json
```

## Options

At least one search term is required.

| Option             | Description                 | Notes                             |
| ------------------ | --------------------------- | --------------------------------- |
| `--json`           | Request native JSON output. | Output retains the native schema. |
| `--long`           | Show extended information.  |                                   |
| `--registry <url>` | Select the registry.        |                                   |

## Compatibility

- **npm:** `npm search`.
- **pnpm:** 11 or later: `pnpm search`. Earlier versions use `npm search` as a fallback.
- **Yarn Classic:** Fallback: `npm search`.
- **Yarn Modern:** Fallback: `npm search`.
- **Bun:** Fallback: `npm search`.

| Option             | npm | pnpm      | Yarn Classic | Yarn Modern | Bun | Notes |
| ------------------ | --- | --------- | ------------ | ----------- | --- | ----- |
| `--json`           | Yes | Yes       | Yes          | Yes         | Yes |       |
| `--long`           | Yes | Before 11 | Yes          | Yes         | Yes |       |
| `--registry <url>` | Yes | Yes       | Yes          | Yes         | Yes |       |

Registry settings and output follow the command that runs.

## Native Options

For native options, see [npm search](https://docs.npmjs.com/cli/v11/commands/npm-search) and [pnpm search](https://pnpm.io/cli/search). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
