# vp pm publish

Publish a package to a registry. Supported targets and preview options depend on the package manager:

```bash
vp pm publish --dry-run
vp pm publish --access public --tag next
vp pm publish ./package.tgz
```

## Options

| Option                          | Description                                  | Notes                                         |
| ------------------------------- | -------------------------------------------- | --------------------------------------------- |
| `--dry-run`                     | Preview without publishing.                  |                                               |
| `--tag <tag>`                   | Select a distribution tag.                   |                                               |
| `--access <public\|restricted>` | Request public or restricted package access. |                                               |
| `--otp <otp>`                   | Supply a one-time password.                  |                                               |
| `--no-git-checks`               | Skip git checks.                             |                                               |
| `--publish-branch <branch>`     | Set the branch name to publish from.         |                                               |
| `--report-summary`              | Save publish summary.                        |                                               |
| `--provenance`                  | Publish with provenance.                     | Requires a compatible publishing environment. |
| `--force`                       | Request native forced publishing.            |                                               |
| `--json`                        | Request native JSON output.                  | Output retains the native schema.             |
| `-r, --recursive`               | Publish all workspace packages.              |                                               |
| `--filter <pattern>`            | Select workspaces by pattern.                | Can be repeated.                              |

## Compatibility

- **npm:** `npm publish`.
- **pnpm:** `pnpm publish`.
- **Yarn Classic:** `yarn publish`.
- **Yarn Modern:** `yarn npm publish` for the active workspace. Explicit tarball or folder targets are rejected.
- **Bun:** `bun publish`.

| Option                          | npm             | pnpm                              | Yarn Classic    | Yarn Modern    | Bun | Notes |
| ------------------------------- | --------------- | --------------------------------- | --------------- | -------------- | --- | ----- |
| `--dry-run`                     | 5.10.0 or later | Yes                               | No              | 4.9.3 or later | Yes |       |
| `--tag <tag>`                   | Yes             | Yes                               | Yes             | Yes            | Yes |       |
| `--access <public\|restricted>` | Yes             | Yes                               | Yes             | Yes            | Yes |       |
| `--otp <otp>`                   | Yes             | Yes                               | 1.12.1 or later | 3.2.0 or later | Yes |       |
| `--no-git-checks`               | No              | Yes                               | No              | No             | No  |       |
| `--publish-branch <branch>`     | No              | Yes                               | No              | No             | No  |       |
| `--report-summary`              | No              | Yes                               | No              | No             | No  |       |
| `--provenance`                  | 9.5.0 or later  | Yes                               | No              | 4.9.0 or later | No  |       |
| `--force`                       | Yes             | Yes                               | No              | No             | No  |       |
| `--json`                        | Yes             | Before 11.0.0 or 11.0.7 and later | Yes             | 4.9.3 or later | No  |       |
| `-r, --recursive`               | 7.13.0 or later | Yes                               | No              | No             | No  |       |
| `--filter <pattern>`            | 7.13.0 or later | Yes                               | No              | No             | No  |       |

Yarn publishing uses Yarn's registry configuration and credentials. Unsupported targets and workspace selection cause an error rather than falling back to npm. Native publish failures are not retried through npm.

Vite+ rejects unsupported options before the native publish command runs. Yarn Classic can ignore `--dry-run` and publish for real, or accept `--provenance` without producing an attestation. Vite+ rejects both options on Yarn Classic.

Arguments after `--` are passed directly to the native command without these checks. Passing an unsupported safety option this way can result in a real publication or a publication without provenance.

For a workflow that separates CI uploads from maintainer approval, use [stage](./stage).

## Native Options

For native options, see [npm publish](https://docs.npmjs.com/cli/v11/commands/npm-publish), [pnpm publish](https://pnpm.io/cli/publish), [Yarn Classic publish](https://classic.yarnpkg.com/en/docs/cli/publish), [Yarn npm publish](https://yarnpkg.com/cli/npm/publish), and [Bun publish](https://bun.com/docs/pm/cli/publish). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
