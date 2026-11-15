# create_package_manager_override_monorepo

## `vpt write-file pnpm-workspace.yaml 'packages:
  - packages/*
'`


## `node capture-create.mjs @override:demo --directory packages/demo --package-manager pnpm --no-interactive --no-agent --no-editor --no-hooks`

```
◇ Scaffolded packages/demo
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
→ Next: cd packages/demo && vp run
```

## `vpt print-file create-summary.json`

the exact package-manager version reported in the summary

```
{
  "packageManager": "pnpm",
  "version": "10.4.1"
}
```

## `vpt print-file package.json`

the session override preserves the existing project pin

```
{
  "name": "create-override-parent",
  "private": true,
  "devEngines": {
    "packageManager": { "name": "pnpm", "version": "<version>", "onFail": "download" }
  },
  "scripts": { "postinstall": "node record-install.cjs" }
}
```

## `node -p require('./package.json').devEngines.packageManager.version`

record the exact existing pin separately from manifest version redaction

```
12.11.2
```

## `vpt print-file installed-with.json`

the install uses the session override

```
{
  "packageManager": "pnpm",
  "version": "10.4.1"
}
```

## `vpt print-file pnpm-workspace.yaml`

preserve the workspace patterns without adding a standalone root

```
packages:
  - packages/*
```

## `vpt stat-file pnpm-lock.yaml`

```
pnpm-lock.yaml: file
```
