# create_package_manager_override_family

## `node capture-create.mjs @override:demo --directory demo --package-manager pnpm --no-interactive --no-agent --no-editor --no-hooks`

```
◇ Scaffolded demo
• Node <version>  npm <version>
✓ Dependencies installed in <duration>
→ Next: cd demo && vp run
```

## `vpt print-file create-summary.json`

the exact package-manager version reported in the summary

```
{
  "packageManager": "npm",
  "version": "10.9.2"
}
```

## `vpt print-file demo/package.json`

the npm override takes precedence over the pnpm selection

```
{
  "name": "demo",
  "version": "0.0.0",
  "scripts": {
    "postinstall": "node record-install.cjs"
  },
  "devDependencies": {
    "vite-plus": "<version>"
  },
  "overrides": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
  },
  "devEngines": {
    "packageManager": {
      "name": "npm",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `node -p require('./demo/package.json').devEngines.packageManager.version`

record the exact pin separately from manifest version redaction

```
10.9.2
```

## `vpt print-file demo/installed-with.json`

package manager recorded by the install lifecycle script

```
{
  "packageManager": "npm",
  "version": "10.9.2"
}
```

## `vpt stat-file demo/package-lock.json demo/pnpm-workspace.yaml`

```
demo/package-lock.json: file
demo/pnpm-workspace.yaml: missing
```
