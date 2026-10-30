# vp pm rebuild

Rebuild installed packages, for example after switching Node.js versions or when a native addon needs recompilation. Alias: `vp pm rb`. The global CLI also accepts `vp rebuild`.

```bash
vp pm rebuild
vp pm rebuild better-sqlite3 sharp
vp pm rebuild -- --update-binary
```

## Options

Supply package names to rebuild only those packages. Omit them to request a full rebuild. There are no additional Vite+ options.

There are no command-specific options.

## Compatibility

- **npm:** `npm rebuild`.
- **pnpm:** `pnpm rebuild`.
- **Yarn Classic:** No command. Warns and exits successfully without performing the operation.
- **Yarn Modern:** `yarn rebuild`.
- **Bun:** No command. Warns and exits successfully without performing the operation.

With pnpm 10 and later, a bare rebuild respects build-script approvals. Naming a package explicitly bypasses that approval gate, so rebuild only packages whose scripts you trust.

On npm 12, [approving a dependency's scripts](./approve-builds) records policy but does not execute scripts skipped by an earlier install. Rebuild that package after approval.

## Native Options

For native options, see [npm rebuild](https://docs.npmjs.com/cli/v11/commands/npm-rebuild), [pnpm rebuild](https://pnpm.io/cli/rebuild), and [Yarn rebuild](https://yarnpkg.com/cli/rebuild). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
