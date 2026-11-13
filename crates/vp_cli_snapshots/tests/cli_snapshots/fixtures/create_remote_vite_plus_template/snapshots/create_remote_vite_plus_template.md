# create_remote_vite_plus_template

## `vp create vp-test-vite-plus --no-interactive --no-agent --no-editor --no-git --no-hooks -- my-app`

create from a template that already generates a Vite+ project


## `vpt print-file my-app/package.json`

dependencies and scripts are not rewritten

```
{
  "name": "my-app",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vp dev",
    "lint": "eslint ."
  },
  "devDependencies": {
    "eslint": "^9.0.0",
    "vite-plus": "latest"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `vpt stat-file my-app/pnpm-workspace.yaml --assert-not file`

no Vite+ overrides are added

```
my-app/pnpm-workspace.yaml: missing
```
