# vp pm patch

Prepare an installed dependency for local patching.

```bash
vp pm patch my-dependency@1.0.0
```

Edit the package in the location reported by the native manager, then save the patch with [patch-commit](./patch-commit).

## Options

A package specification is required. There are no additional Vite+ options.

There are no command-specific options.

## Compatibility

- **npm:** 12 or later: `npm patch add`. Earlier versions warn and skip.
- **pnpm:** `pnpm patch`.
- **Yarn Classic:** No command. Warns and exits successfully without performing the operation.
- **Yarn Modern:** `yarn patch`.
- **Bun:** `bun patch`.

The edit location and patch storage format are native-manager details. Vite+ does not convert patches between managers.

## Native Options

For native options, see [npm patch add](https://docs.npmjs.com/cli/v12/commands/npm-patch), [pnpm patch](https://pnpm.io/cli/patch), [Yarn patch](https://yarnpkg.com/cli/patch), and [Bun patch](https://bun.com/docs/pm/cli/patch). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
