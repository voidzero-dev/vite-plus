# vp pm pkg

Read and update `package.json` fields without editing the manifest by hand.

```bash
vp pm pkg get name version
vp pm pkg set scripts.check='vp check'
vp pm pkg set private=true --json
vp pm pkg delete scripts.old
vp pm pkg fix
```

## Options

- `get [key...]` reads fields, or the entire manifest if no keys are supplied. `--json` requests native JSON output.
- `set <key=value...>` writes fields. `--json` parses values as JSON so booleans, numbers, arrays, and objects are not saved as strings.
- `delete <key...>` removes fields.
- `fix` asks the manager to correct common manifest errors.

Field paths and normalization follow the native manager. Quote paths with brackets or shell-sensitive characters.

### `get`

| Option   | Description            | Notes                             |
| -------- | ---------------------- | --------------------------------- |
| `--json` | Output in JSON format. | Output retains the native schema. |

### `set`

| Option   | Description                          | Notes                                                                     |
| -------- | ------------------------------------ | ------------------------------------------------------------------------- |
| `--json` | Parse values as JSON before writing. | Parses booleans, numbers, arrays, and objects rather than saving strings. |

### `delete`

There are no command-specific options.

### `fix`

There are no command-specific options.

## Compatibility

### `get`

- **npm:** 7.20.0 or later: `npm pkg get`. Earlier versions warn and skip.
- **pnpm:** 11.3.0 or later: `pnpm pkg get`. Earlier versions warn and skip.
- **Yarn Classic:** No command. Warns and exits successfully without performing the operation.
- **Yarn Modern:** No command. Warns and exits successfully without performing the operation.
- **Bun:** 1.2.19 or later: `bun pm pkg get`. Earlier versions warn and skip.

| Option   | npm             | pnpm            | Yarn Classic | Yarn Modern | Bun             | Notes |
| -------- | --------------- | --------------- | ------------ | ----------- | --------------- | ----- |
| `--json` | 7.20.0 or later | 11.3.0 or later | No           | No          | 1.2.19 or later |       |

### `set`

- **npm:** 7.20.0 or later: `npm pkg set`. Earlier versions warn and skip.
- **pnpm:** 11.3.0 or later: `pnpm pkg set`. Earlier versions warn and skip.
- **Yarn Classic:** No command. Warns and exits successfully without performing the operation.
- **Yarn Modern:** No command. Warns and exits successfully without performing the operation.
- **Bun:** 1.2.19 or later: `bun pm pkg set`. Earlier versions warn and skip.

| Option   | npm             | pnpm            | Yarn Classic | Yarn Modern | Bun             | Notes |
| -------- | --------------- | --------------- | ------------ | ----------- | --------------- | ----- |
| `--json` | 7.20.0 or later | 11.3.0 or later | No           | No          | 1.2.19 or later |       |

### `delete`

- **npm:** 7.20.0 or later: `npm pkg delete`. Earlier versions warn and skip.
- **pnpm:** 11.3.0 or later: `pnpm pkg delete`. Earlier versions warn and skip.
- **Yarn Classic:** No command. Warns and exits successfully without performing the operation.
- **Yarn Modern:** No command. Warns and exits successfully without performing the operation.
- **Bun:** 1.2.19 or later: `bun pm pkg delete`. Earlier versions warn and skip.

### `fix`

- **npm:** 9.8.0 or later: `npm pkg fix`. Earlier versions warn and skip.
- **pnpm:** 11.3.0 or later: `pnpm pkg fix`. Earlier versions warn and skip.
- **Yarn Classic:** No command. Warns and exits successfully without performing the operation.
- **Yarn Modern:** No command. Warns and exits successfully without performing the operation.
- **Bun:** 1.2.19 or later: `bun pm pkg fix`. Earlier versions warn and skip.

Versions below the stated requirements warn and exit successfully without running the operation. There is no npm fallback for unsupported managers or versions.

## Native Options

For native options, see [npm pkg](https://docs.npmjs.com/cli/v11/commands/npm-pkg), [pnpm pkg](https://pnpm.io/cli/pkg), and [Bun pm pkg](https://bun.com/docs/pm/cli/pm). To pass these options through Vite+, see [Native Arguments](./#native-arguments).
