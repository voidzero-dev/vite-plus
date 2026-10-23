# vp pm stage

Use staged publishing to separate a package upload from its release. A CI job can upload a staged version without 2FA, then a maintainer approves or rejects it from a trusted device with 2FA.

```bash
vp pm stage publish --dry-run
vp pm stage publish
vp pm stage list
vp pm stage view <stage-id>
vp pm stage download <stage-id>
vp pm stage approve <stage-id>
vp pm stage reject <stage-id>
```

## Options

Use the subcommands below to upload, inspect, approve, or reject staged versions.

### `publish`

Upload a package. Accepts an optional tarball or folder target.

| Option                          | Description                                  | Notes                                         |
| ------------------------------- | -------------------------------------------- | --------------------------------------------- |
| `--tag <tag>`                   | Publish tag.                                 |                                               |
| `--access <public\|restricted>` | Request public or restricted package access. |                                               |
| `--otp <otp>`                   | Supply a one-time password.                  |                                               |
| `--dry-run`                     | Preview without staging.                     |                                               |
| `--json`                        | Request native JSON output.                  | Output retains the native schema.             |
| `-r, --recursive`               | Stage all publishable workspace packages.    |                                               |
| `--filter <pattern>`            | Select workspaces by pattern.                | Can be repeated.                              |
| `--provenance`                  | Stage with provenance.                       | Requires a compatible publishing environment. |
| `--registry <url>`              | Select the registry.                         |                                               |

### `list`

List staged versions. Accepts an optional package specification. Alias: `ls`.

| Option             | Description                 | Notes                             |
| ------------------ | --------------------------- | --------------------------------- |
| `--json`           | Request native JSON output. | Output retains the native schema. |
| `--registry <url>` | Select the registry.        |                                   |

### `view`

Inspect a staged version. Requires a stage ID.

| Option             | Description                 | Notes                             |
| ------------------ | --------------------------- | --------------------------------- |
| `--json`           | Request native JSON output. | Output retains the native schema. |
| `--registry <url>` | Select the registry.        |                                   |

### `download`

Download a staged tarball. Requires a stage ID.

| Option             | Description          | Notes |
| ------------------ | -------------------- | ----- |
| `--registry <url>` | Select the registry. |       |

### `approve`

Promote a staged version to the live registry. Requires a stage ID.

| Option             | Description                 | Notes |
| ------------------ | --------------------------- | ----- |
| `--otp <otp>`      | Supply a one-time password. |       |
| `--registry <url>` | Select the registry.        |       |

### `reject`

Discard a staged version. Requires a stage ID.

| Option             | Description                 | Notes |
| ------------------ | --------------------------- | ----- |
| `--otp <otp>`      | Supply a one-time password. |       |
| `--registry <url>` | Select the registry.        |       |

## Compatibility

All npm paths, including fallbacks, require npm 11.15 or later and Node.js 22.14 or later.

### `publish`

- **npm:** 11.15 or later: `npm stage publish`.
- **pnpm:** 11.3 or later: `pnpm stage publish`.
- **Yarn Classic:** Fallback: `npm stage publish`.
- **Yarn Modern:** 4.16.0 or later: `yarn npm publish --staged`. Earlier versions use `npm stage publish` as a fallback. Explicit targets use npm fallback.
- **Bun:** Fallback: `npm stage publish`.

| Option                          | npm                     | pnpm | Yarn Classic            | Yarn Modern                               | Bun                     | Notes                                            |
| ------------------------------- | ----------------------- | ---- | ----------------------- | ----------------------------------------- | ----------------------- | ------------------------------------------------ |
| `--tag <tag>`                   | Yes                     | Yes  | Yes                     | Yes                                       | Yes                     |                                                  |
| `--access <public\|restricted>` | Yes                     | Yes  | Yes                     | Yes                                       | Yes                     |                                                  |
| `--otp <otp>`                   | Yes                     | Yes  | Yes                     | Yes                                       | Yes                     |                                                  |
| `--dry-run`                     | Yes                     | Yes  | Yes                     | Yes                                       | Yes                     |                                                  |
| `--json`                        | Yes                     | Yes  | Yes                     | Yes                                       | Yes                     |                                                  |
| `-r, --recursive`               | No target or target `.` | Yes  | No target or target `.` | Before 4.16 with no target, or target `.` | No target or target `.` |                                                  |
| `--filter <pattern>`            | No target or target `.` | Yes  | No target or target `.` | Before 4.16 with no target, or target `.` | No target or target `.` |                                                  |
| `--provenance`                  | Yes                     | Yes  | Yes                     | Yes                                       | Yes                     |                                                  |
| `--registry <url>`              | Yes                     | Yes  | Yes                     | Before 4.16, or an explicit target        | Yes                     | Use Yarn registry settings on native Yarn paths. |

### `list`

- **npm:** 11.15 or later: `npm stage list`.
- **pnpm:** 11.3 or later: `pnpm stage list`.
- **Yarn Classic:** Fallback: `npm stage list`.
- **Yarn Modern:** 4.16.0 or later: `yarn npm stage list`. Earlier versions use `npm stage list` as a fallback.
- **Bun:** Fallback: `npm stage list`.

| Option             | npm | pnpm | Yarn Classic | Yarn Modern   | Bun | Notes                                            |
| ------------------ | --- | ---- | ------------ | ------------- | --- | ------------------------------------------------ |
| `--json`           | Yes | Yes  | Yes          | Yes           | Yes |                                                  |
| `--registry <url>` | Yes | Yes  | Yes          | Before 4.16.0 | Yes | Use Yarn registry settings on native Yarn paths. |

### `view`

- **npm:** 11.15 or later: `npm stage view`.
- **pnpm:** 11.3 or later: `pnpm stage view`.
- **Yarn Classic:** Fallback: `npm stage view`.
- **Yarn Modern:** Fallback: `npm stage view`.
- **Bun:** Fallback: `npm stage view`.

| Option             | npm | pnpm | Yarn Classic | Yarn Modern | Bun | Notes |
| ------------------ | --- | ---- | ------------ | ----------- | --- | ----- |
| `--json`           | Yes | Yes  | Yes          | Yes         | Yes |       |
| `--registry <url>` | Yes | Yes  | Yes          | Yes         | Yes |       |

### `download`

- **npm:** 11.15 or later: `npm stage download`.
- **pnpm:** 11.3 or later: `pnpm stage download`.
- **Yarn Classic:** Fallback: `npm stage download`.
- **Yarn Modern:** Fallback: `npm stage download`.
- **Bun:** Fallback: `npm stage download`.

| Option             | npm | pnpm | Yarn Classic | Yarn Modern | Bun | Notes |
| ------------------ | --- | ---- | ------------ | ----------- | --- | ----- |
| `--registry <url>` | Yes | Yes  | Yes          | Yes         | Yes |       |

### `approve`

- **npm:** 11.15 or later: `npm stage approve`.
- **pnpm:** 11.3 or later: `pnpm stage approve`.
- **Yarn Classic:** Fallback: `npm stage approve`.
- **Yarn Modern:** 4.16.0 or later: `yarn npm stage approve`. Earlier versions use `npm stage approve` as a fallback.
- **Bun:** Fallback: `npm stage approve`.

| Option             | npm | pnpm | Yarn Classic | Yarn Modern   | Bun | Notes                                            |
| ------------------ | --- | ---- | ------------ | ------------- | --- | ------------------------------------------------ |
| `--otp <otp>`      | Yes | Yes  | Yes          | Yes           | Yes |                                                  |
| `--registry <url>` | Yes | Yes  | Yes          | Before 4.16.0 | Yes | Use Yarn registry settings on native Yarn paths. |

### `reject`

- **npm:** 11.15 or later: `npm stage reject`.
- **pnpm:** 11.3 or later: `pnpm stage reject`.
- **Yarn Classic:** Fallback: `npm stage reject`.
- **Yarn Modern:** 4.16.0 or later: `yarn npm stage reject`. Earlier versions use `npm stage reject` as a fallback.
- **Bun:** Fallback: `npm stage reject`.

| Option             | npm | pnpm | Yarn Classic | Yarn Modern   | Bun | Notes                                            |
| ------------------ | --- | ---- | ------------ | ------------- | --- | ------------------------------------------------ |
| `--otp <otp>`      | Yes | Yes  | Yes          | Yes           | Yes |                                                  |
| `--registry <url>` | Yes | Yes  | Yes          | Before 4.16.0 | Yes | Use Yarn registry settings on native Yarn paths. |

Yarn fallback paths and Bun warn before invoking npm.

This command is unrelated to Yarn's version-control staging command or Vite+'s staged-file checks.

## Native Options

For native options, see [npm stage](https://docs.npmjs.com/cli/v11/commands/npm-stage), [pnpm stage](https://pnpm.io/cli/stage), [Yarn npm publish](https://yarnpkg.com/cli/npm/publish), [Yarn npm stage list](https://yarnpkg.com/cli/npm/stage/list), [Yarn npm stage approve](https://yarnpkg.com/cli/npm/stage/approve), and [Yarn npm stage reject](https://yarnpkg.com/cli/npm/stage/reject). To pass these options through Vite+, see [Native Arguments](./#native-arguments).

For the registry workflow, see [npm staged publishing](https://docs.npmjs.com/staged-publishing).
