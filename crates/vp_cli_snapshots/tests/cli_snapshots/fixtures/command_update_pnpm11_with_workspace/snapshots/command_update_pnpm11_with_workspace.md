# command_update_pnpm11_with_workspace

## `vp update testnpm2 --latest -w`

should update in workspace root

```

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json`

```
{
  "name": "command-update-pnpm11-with-workspace",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "^1.0.1"
  },
  "packageManager": "pnpm@11.0.6"
}
```

## `vp update testnpm2 --latest --filter app`

should update in specific package

```
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
  "name": "command-update-pnpm11-with-workspace",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "^1.0.1"
  },
  "packageManager": "pnpm@11.0.6"
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
