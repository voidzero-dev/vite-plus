# vp dlx

Download and run a package without adding it to the project's dependencies.

```bash
vp dlx create-vite my-app
vp dlx --silent create-vite my-app
```

## Options

Put Vite+ options before the package or command:

Arguments after the command are passed to the executed program, not parsed as Vite+ options. Shell mode interprets shell syntax; use it only with trusted command strings.

| Option                 | Description                                                            | Notes            |
| ---------------------- | ---------------------------------------------------------------------- | ---------------- |
| `-p, --package <name>` | Make the selected package available to the executed command.           | Can be repeated. |
| `-c, --shell-mode`     | Execute within a shell environment.                                    |                  |
| `-s, --silent`         | Suppress Vite+ output and request native quiet output where available. |                  |

## Compatibility

- **npm:** `npm exec --yes`.
- **pnpm:** `pnpm dlx`.
- **Yarn Classic:** Fallback: `npx --yes`.
- **Yarn Modern:** `yarn dlx`.
- **Bun:** `bun x`.

| Option                 | npm | pnpm | Yarn Classic | Yarn Modern | Bun | Notes                             |
| ---------------------- | --- | ---- | ------------ | ----------- | --- | --------------------------------- |
| `-p, --package <name>` | Yes | Yes  | Yes          | Yes         | Yes |                                   |
| `-c, --shell-mode`     | Yes | Yes  | Yes          | No          | No  |                                   |
| `-s, --silent`         | Yes | Yes  | Yes          | Yes         | Yes | Bun only suppresses Vite+ output. |

For installed project binaries, use `vp exec` or `vpx` instead. See [Running Binaries](/guide/vpx) for those workflows.

## Native Options

For native options, see [npm exec](https://docs.npmjs.com/cli/v11/commands/npm-exec), [pnpm dlx](https://pnpm.io/cli/dlx), [Yarn dlx](https://yarnpkg.com/cli/dlx), and [Bun x](https://bun.com/docs/pm/bunx). For the Yarn Classic fallback, see [npx](https://docs.npmjs.com/cli/v11/commands/npx). For argument handling, see [Native Arguments](./#native-arguments).
