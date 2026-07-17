# command_update_npm10_with_workspace

## `vp update testnpm2 -w -- --no-audit`

should update in workspace root

```

added 5 packages in <duration>
```

## `vpt print-file package.json`

```
{
  "name": "command-update-npm10-with-workspace",
  "version": "1.0.0",
  "workspaces": [
    "packages/*"
  ],
  "dependencies": {
    "testnpm2": "*"
  },
  "packageManager": "npm@10.9.4"
}
```

## `vp update testnpm2 --latest --filter app -- --no-audit`

should update in specific package

```
warn: npm doesn't support --latest flag. Updating within semver range only.

up to date in <duration>
```

## `vpt print-file packages/app/package.json`

```
{
  "name": "app",
  "dependencies": {
    "test-vite-plus-install": "*",
    "testnpm2": "*"
  },
  "devDependencies": {
    "test-vite-plus-package": "*"
  }
}
```

## `vp up -D --filter app -- --no-audit`

should update dev dependencies in app

```
npm warn workspaces app in filter set, but no workspace folder present

up to date in <duration>
```

## `vpt print-file packages/app/package.json`

```
{
  "name": "app",
  "dependencies": {
    "test-vite-plus-install": "*",
    "testnpm2": "*"
  },
  "devDependencies": {
    "test-vite-plus-package": "*"
  }
}
```

## `vp update --filter * -- --no-audit`

should update in all packages

```
npm warn workspaces app in filter set, but no workspace folder present
npm warn workspaces @vite-plus-test/utils in filter set, but no workspace folder present

up to date in <duration>
```

## `vpt print-file packages/app/package.json packages/utils/package.json`

```
{
  "name": "app",
  "dependencies": {
    "test-vite-plus-install": "*",
    "testnpm2": "*"
  },
  "devDependencies": {
    "test-vite-plus-package": "*"
  }
}
{
  "name": "@vite-plus-test/utils",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "*"
  }
}
```

## `vp update -r --no-save -- --no-audit`

should update recursively without saving

```
npm warn workspaces app in filter set, but no workspace folder present
npm warn workspaces @vite-plus-test/utils in filter set, but no workspace folder present

up to date in <duration>
```

## `vpt print-file package.json packages/app/package.json`

```
{
  "name": "command-update-npm10-with-workspace",
  "version": "1.0.0",
  "workspaces": [
    "packages/*"
  ],
  "dependencies": {
    "testnpm2": "*"
  },
  "packageManager": "npm@10.9.4"
}
{
  "name": "app",
  "dependencies": {
    "test-vite-plus-install": "*",
    "testnpm2": "*"
  },
  "devDependencies": {
    "test-vite-plus-package": "*"
  }
}
```

## `vp update --workspace --filter app @vite-plus-test/utils -- --no-audit`

should update workspace dependency

```

up to date in <duration>
```

## `vpt print-file packages/app/package.json`

```
{
  "name": "app",
  "dependencies": {
    "test-vite-plus-install": "*",
    "testnpm2": "*"
  },
  "devDependencies": {
    "test-vite-plus-package": "*"
  }
}
```
