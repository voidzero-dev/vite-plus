# vp pm audit

Audit dependencies for known vulnerabilities.

```bash
vp pm audit
vp pm audit --json
vp pm audit --production --level high
```

## Options

| Option            | Description                                  | Notes                             |
| ----------------- | -------------------------------------------- | --------------------------------- |
| `--fix`           | Request automatic vulnerability remediation. |                                   |
| `--json`          | Request native JSON output.                  | Output retains the native schema. |
| `--level <level>` | Minimum vulnerability level to report.       |                                   |
| `--production`    | Only audit production dependencies.          |                                   |

## Compatibility

- **npm:** `npm audit` or `npm audit fix`.
- **pnpm:** `pnpm audit`.
- **Yarn Classic:** `yarn audit`.
- **Yarn Modern:** `yarn npm audit`.
- **Bun:** `bun audit` or `bun audit fix`.

| Option            | npm | pnpm | Yarn Classic | Yarn Modern | Bun             | Notes |
| ----------------- | --- | ---- | ------------ | ----------- | --------------- | ----- |
| `--fix`           | Yes | Yes  | No           | No          | 1.4 or later    |       |
| `--json`          | Yes | Yes  | Yes          | Yes         | Yes             |       |
| `--level <level>` | Yes | Yes  | Yes          | Yes         | Yes             |       |
| `--production`    | Yes | Yes  | Yes          | Yes         | 1.2.21 or later |       |

Vulnerability output and failure thresholds retain the native manager's semantics.

## Native Options

For native options, see [npm audit](https://docs.npmjs.com/cli/v11/commands/npm-audit), [pnpm audit](https://pnpm.io/cli/audit), [Yarn npm audit](https://yarnpkg.com/cli/npm/audit), [Yarn Classic audit](https://classic.yarnpkg.com/en/docs/cli/audit), and [Bun audit](https://bun.com/docs/pm/cli/audit). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
