# vp pm version

Run the package manager's version command. This manages package versions, not the Vite+ version displayed by `vp --version`.

```bash
vp pm version patch
vp pm version 2.0.0
```

## Options

Supply a version or increment strategy such as `major`, `minor`, or `patch`. Omitting it retains the manager's native behavior, which may display information, prompt, or require a strategy.

| Option   | Description                 | Notes                             |
| -------- | --------------------------- | --------------------------------- |
| `--json` | Request native JSON output. | Output retains the native schema. |

## Compatibility

- **npm:** `npm version`.
- **pnpm:** `pnpm version`.
- **Yarn Classic:** `yarn version`.
- **Yarn Modern:** `yarn version`.
- **Bun:** 1.2.18 or later: `bun pm version`. Earlier versions warn before the command is invoked.

| Option   | npm | pnpm | Yarn Classic | Yarn Modern | Bun | Notes |
| -------- | --- | ---- | ------------ | ----------- | --- | ----- |
| `--json` | Yes | Yes  | Yes          | No          | No  |       |

Yarn Classic converts increment strategies to flags such as `--patch`. An explicit version becomes `--new-version <version>`. Other managers retain their native operand syntax.

Version commands can modify manifests and lockfiles and may create Git commits or tags. Vite+ preserves the selected manager's behavior.

## Native Options

For native options, see [npm version](https://docs.npmjs.com/cli/v11/commands/npm-version), [pnpm version](https://pnpm.io/cli/version), [Yarn version](https://yarnpkg.com/cli/version), [Yarn Classic version](https://classic.yarnpkg.com/en/docs/cli/version), and [Bun pm version](https://bun.com/docs/pm/cli/pm). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
