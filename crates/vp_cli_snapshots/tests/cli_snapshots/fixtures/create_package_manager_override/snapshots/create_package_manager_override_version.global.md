# create_package_manager_override_version

## `node capture-create.mjs @override:demo --directory demo --package-manager pnpm --no-interactive --no-agent --no-editor --no-hooks`

```
◇ Scaffolded demo
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
→ Next: cd demo && vp run
```

## `vpt print-file create-summary.json`

the exact package-manager version reported in the summary

```
{
  "packageManager": "pnpm",
  "version": "10.6.1"
}
```

## `vpt print-file demo/package.json`

pnpm 10.6.1 reads settings from package.json

```
{
  "name": "demo",
  "version": "0.0.0",
  "scripts": {
    "postinstall": "node record-install.cjs"
  },
  "devDependencies": {
    "vite-plus": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  },
  "pnpm": {
    "overrides": {
      "vite@*": "npm:@voidzero-dev/vite-plus-core@<version>"
    }
  }
}
```

## `node -p require('./demo/package.json').devEngines.packageManager.version`

record the exact pin separately from manifest version redaction

```
10.6.1
```

## `vpt print-file demo/installed-with.json`

package manager recorded by the install lifecycle script

```
{
  "packageManager": "pnpm",
  "version": "10.6.1"
}
```

## `vpt print-file demo/pnpm-workspace.yaml`

```
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
```

## `vpt stat-file demo/pnpm-lock.yaml`

```
demo/pnpm-lock.yaml: file
```
