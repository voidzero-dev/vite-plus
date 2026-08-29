# command_add_bun_with_workspace

## `vp add testnpm2 --filter app -- --silent`

should add package to packages/app

```
```

## `vpt print-file packages/app/package.json`

```
{
  "name": "app",
  "dependencies": {
    "testnpm2": "^1.0.1"
  }
}
```

## `vp add test-vite-plus-package --save-catalog -- --silent`

should add package to the default catalog

```
```

## `vpt print-file package.json`

```
{
  "name": "command-add-bun-with-workspace",
  "version": "1.0.0",
  "workspaces": ["packages/*"],
  "packageManager": "bun@1.4.0",
  "dependencies": {
    "test-vite-plus-package": "catalog:"
  },
  "catalog": {
    "test-vite-plus-package": "^1.0.0"
  }
}
```
