# command_update_pnpm10_with_workspace

## `vp update testnpm2 --latest -w`

should update in workspace root

```

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json`

```
{
  "name": "command-update-pnpm10-with-workspace",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "^1.0.1"
  },
  "packageManager": "pnpm@10.18.0"
}
```

## `vp update testnpm2 --latest --filter app`

should update in specific package

```
.                                        |  WARN  `node_modules` is present. Lockfile only installation will make it out-of-date
.                                        |   +2 +

Done in <duration> using pnpm <version>
```

## `vpt print-file packages/app/package.json`

```
{
  "name": "app",
  "dependencies": {
    "@vite-plus-test/utils": "workspace:*",
    "test-vite-plus-install": "*",
    "testnpm2": "^1.0.1"
  },
  "devDependencies": {
    "test-vite-plus-package": "*"
  }
}
```

## `vp up -D --filter app`

should update dev dependencies in app

```
.                                        |  WARN  `node_modules` is present. Lockfile only installation will make it out-of-date

Done in <duration> using pnpm <version>
```

## `vpt print-file packages/app/package.json`

```
{
  "name": "app",
  "dependencies": {
    "@vite-plus-test/utils": "workspace:*",
    "test-vite-plus-install": "*",
    "testnpm2": "^1.0.1"
  },
  "devDependencies": {
    "test-vite-plus-package": "^1.0.0"
  }
}
```

## `vp update --latest --filter *`

should update in all packages

```
Scope: all 3 workspace projects

Done in <duration> using pnpm <version>
```

## `vpt print-file packages/app/package.json packages/utils/package.json`

```
{
  "name": "app",
  "dependencies": {
    "@vite-plus-test/utils": "workspace:*",
    "test-vite-plus-install": "^1.0.0",
    "testnpm2": "^1.0.1"
  },
  "devDependencies": {
    "test-vite-plus-package": "^1.0.0"
  }
}
{
  "name": "@vite-plus-test/utils",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "^1.0.1"
  }
}
```

## `vp update -r --no-save`

should update recursively without saving

```
Scope: all 3 workspace projects

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json packages/app/package.json`

```
{
  "name": "command-update-pnpm10-with-workspace",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "^1.0.1"
  },
  "packageManager": "pnpm@10.18.0"
}
{
  "name": "app",
  "dependencies": {
    "@vite-plus-test/utils": "workspace:*",
    "test-vite-plus-install": "^1.0.0",
    "testnpm2": "^1.0.1"
  },
  "devDependencies": {
    "test-vite-plus-package": "^1.0.0"
  }
}
```

## `vp update --workspace --filter app @vite-plus-test/utils`

should update workspace dependency

```
.                                        |  WARN  `node_modules` is present. Lockfile only installation will make it out-of-date

Done in <duration> using pnpm <version>
```

## `vpt print-file packages/app/package.json`

```
{
  "name": "app",
  "dependencies": {
    "@vite-plus-test/utils": "workspace:*",
    "test-vite-plus-install": "^1.0.0",
    "testnpm2": "^1.0.1"
  },
  "devDependencies": {
    "test-vite-plus-package": "^1.0.0"
  }
}
```
