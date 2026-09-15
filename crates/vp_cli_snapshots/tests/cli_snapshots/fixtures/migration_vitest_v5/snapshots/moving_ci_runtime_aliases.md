# moving_ci_runtime_aliases

Keep current LTS and latest-release aliases without a runtime review or a numeric pin, including after repeated migration.

## `vpt write-file .github/workflows/test.yml 'jobs:
  lts:
    steps:
      - uses: actions/setup-node@v6
        with:
          node-version: '\''lts/*'\''
  latest:
    steps:
      - uses: actions/setup-node@v6
        with:
          node-version: '\''latest'\''
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 files had imports rewritten
```

## `vpt print-file .github/workflows/test.yml`

```
jobs:
  lts:
    steps:
      - uses: actions/setup-node@v6
        with:
          node-version: 'lts/*'
  latest:
    steps:
      - uses: actions/setup-node@v6
        with:
          node-version: 'latest'
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
