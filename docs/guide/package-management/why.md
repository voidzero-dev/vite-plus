# vp why

Explain why a dependency is installed. Alias: `vp explain`.

```bash
vp why react
vp why react --json
vp why react --filter web
```

## Options

| Option                    | Description                                     | Notes                             |
| ------------------------- | ----------------------------------------------- | --------------------------------- |
| `--json`                  | Request native JSON output.                     | Output retains the native schema. |
| `--long`                  | Show extended information.                      |                                   |
| `--parseable`             | Show parseable output.                          |                                   |
| `-r, --recursive`         | Check recursively across all workspaces.        |                                   |
| `--filter <pattern>`      | Select workspaces by pattern.                   | Can be repeated.                  |
| `-w, --workspace-root`    | Check in workspace root.                        |                                   |
| `-P, --prod`              | Only production dependencies.                   |                                   |
| `-D, --dev`               | Only dev dependencies.                          |                                   |
| `--depth <depth>`         | Limit tree depth.                               |                                   |
| `--no-optional`           | Exclude optional dependencies.                  |                                   |
| `--exclude-peers`         | Exclude peer dependencies.                      |                                   |
| `--find-by <finder_name>` | Use a finder function defined in .pnpmfile.cjs. |                                   |

## Compatibility

- **npm:** `npm explain`.
- **pnpm:** `pnpm why`.
- **Yarn Classic:** `yarn why`. With multiple package names, warns and checks only the first.
- **Yarn Modern:** `yarn why`. With multiple package names, warns and checks only the first.
- **Bun:** `bun why`.

| Option                    | npm             | pnpm             | Yarn Classic | Yarn Modern | Bun | Notes                                  |
| ------------------------- | --------------- | ---------------- | ------------ | ----------- | --- | -------------------------------------- |
| `--json`                  | Yes             | Yes              | Yes          | Yes         | No  |                                        |
| `--long`                  | No              | Yes              | No           | No          | No  |                                        |
| `--parseable`             | No              | Yes              | No           | No          | No  |                                        |
| `-r, --recursive`         | 7.14.0 or later | Yes              | No           | Yes         | No  |                                        |
| `--filter <pattern>`      | 7.14.0 or later | Yes              | No           | No          | No  |                                        |
| `-w, --workspace-root`    | No              | Yes              | No           | No          | No  |                                        |
| `-P, --prod`              | No              | Yes              | No           | No          | No  |                                        |
| `-D, --dev`               | No              | Yes              | No           | No          | No  |                                        |
| `--depth <depth>`         | No              | Yes              | No           | No          | Yes |                                        |
| `--no-optional`           | No              | Yes              | No           | No          | No  |                                        |
| `--exclude-peers`         | No              | Yes              | No           | Yes         | No  | Yarn Modern includes peers by default. |
| `--find-by <finder_name>` | No              | 10.16.0 or later | No           | No          | No  |                                        |

The dependency graph does not include tools Vite+ bundles or compiles. For readable output, Vite+ shows a hint when it also provides the requested package. Use `vp toolchain [tool]` to inspect bundled tools.

## Native Options

For native options, see [npm explain](https://docs.npmjs.com/cli/v11/commands/npm-explain), [pnpm why](https://pnpm.io/cli/why), [Yarn why](https://yarnpkg.com/cli/why), [Yarn Classic why](https://classic.yarnpkg.com/en/docs/cli/why), and [Bun why](https://bun.com/docs/pm/cli/why). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
