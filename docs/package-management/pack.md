# vp pm pack

Create a package tarball from the project. This is different from [`vp pack`](/guide/pack), which builds libraries with tsdown.

```bash
vp pm pack
vp pm pack --pack-destination ./artifacts
```

## Options

| Option                                  | Description                                        | Notes                             |
| --------------------------------------- | -------------------------------------------------- | --------------------------------- |
| `-r, --recursive`                       | Pack all workspace packages.                       |                                   |
| `--filter <pattern>`                    | Select workspaces by pattern.                      | Can be repeated.                  |
| `--out <out>`                           | Choose the tarball output path.                    |                                   |
| `--pack-destination <pack-destination>` | Choose the tarball output directory.               |                                   |
| `--pack-gzip-level <pack-gzip-level>`   | Request a gzip compression level from 0 through 9. |                                   |
| `--json`                                | Request native JSON output.                        | Output retains the native schema. |

## Compatibility

- **npm:** `npm pack`.
- **pnpm:** `pnpm pack`.
- **Yarn Classic:** `yarn pack`.
- **Yarn Modern:** `yarn pack`.
- **Bun:** `bun pm pack`.

| Option                                  | npm             | pnpm             | Yarn Classic | Yarn Modern | Bun | Notes                                |
| --------------------------------------- | --------------- | ---------------- | ------------ | ----------- | --- | ------------------------------------ |
| `-r, --recursive`                       | 7.9.0 or later  | 10.11.0 or later | No           | Yes         | No  |                                      |
| `--filter <pattern>`                    | 7.9.0 or later  | 10.11.0 or later | No           | Yes         | No  |                                      |
| `--out <out>`                           | No              | Yes              | Yes          | Yes         | Yes |                                      |
| `--pack-destination <pack-destination>` | 7.18.0 or later | Yes              | No           | No          | Yes | Vite+ creates the directory for npm. |
| `--pack-gzip-level <pack-gzip-level>`   | No              | Yes              | No           | No          | Yes |                                      |
| `--json`                                | Yes             | Yes              | Yes          | Yes         | No  |                                      |

## Native Options

For native options, see [npm pack](https://docs.npmjs.com/cli/v11/commands/npm-pack), [pnpm pack](https://pnpm.io/cli/pack), [Yarn pack](https://yarnpkg.com/cli/pack), [Yarn Classic pack](https://classic.yarnpkg.com/en/docs/cli/pack), and [Bun pm pack](https://bun.com/docs/pm/cli/pm). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
