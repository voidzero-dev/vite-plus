# vp pm ci

Install dependencies with a frozen lockfile for CI.

```bash
vp pm ci
vp pm ci -- --ignore-scripts
```

## Options

This command has no additional Vite+ options. For translated Vite+ options, use [`vp install --frozen-lockfile`](./install).

There are no command-specific options.

## Compatibility

- **npm:** `npm ci`.
- **pnpm:** `pnpm install --frozen-lockfile`.
- **Yarn Classic:** `yarn install --frozen-lockfile`.
- **Yarn Modern:** `yarn install --immutable`.
- **Bun:** `bun install --frozen-lockfile`.

The native manager determines installation cleanup and lockfile validation. This command does not make all managers behave like `npm ci` beyond the frozen-lockfile request.

## Native Options

For native options, see [npm ci](https://docs.npmjs.com/cli/v11/commands/npm-ci), [pnpm install](https://pnpm.io/cli/install), [Yarn install](https://yarnpkg.com/cli/install), [Yarn Classic install](https://classic.yarnpkg.com/en/docs/cli/install), and [Bun install](https://bun.com/docs/pm/cli/install). To pass these options through Vite+, see [Native Arguments](./#native-arguments). For CI setup, see the [CI guide](/guide/ci).
