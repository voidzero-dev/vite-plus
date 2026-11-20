# vp dedupe

Reduce duplicate dependency entries using the selected package manager.

```bash
vp dedupe
vp dedupe --check
```

## Options

| Option    | Description                                | Notes |
| --------- | ------------------------------------------ | ----- |
| `--check` | Check if deduplication would make changes. |       |

## Compatibility

- **npm:** `npm dedupe`.
- **pnpm:** `pnpm dedupe`.
- **Yarn Classic:** Fallback: `yarn install`.
- **Yarn Modern:** `yarn dedupe`.
- **Bun:** 1.4 or later: `bun dedupe`. Earlier versions use `bun install` as a fallback.

| Option    | npm | pnpm | Yarn Classic | Yarn Modern | Bun          | Notes |
| --------- | --- | ---- | ------------ | ----------- | ------------ | ----- |
| `--check` | Yes | Yes  | No           | Yes         | 1.4 or later |       |

A fallback install is not a read-only check. Unsupported `--check` requests fail before the fallback runs.

## Native Options

For native options, see [npm dedupe](https://docs.npmjs.com/cli/v11/commands/npm-dedupe), [pnpm dedupe](https://pnpm.io/cli/dedupe), [Yarn dedupe](https://yarnpkg.com/cli/dedupe), [Yarn Classic install](https://classic.yarnpkg.com/en/docs/cli/install), and [Bun dedupe](https://bun.com/docs/pm/cli/dedupe). For the Bun fallback, see [Bun install](https://bun.com/docs/pm/cli/install). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
