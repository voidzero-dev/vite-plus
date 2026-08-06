# migration_upgrade_setup_skipped_default_pnpm

## `vp migrate --no-interactive`

existing Vite+ project: upgrade the toolchain version only, skip the full setup

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  0.1.21 → <version>
    vite              → <version>
• Package manager settings configured
• Skipped editor, hooks, and lint setup. Run `vp migrate --full` to apply them.
```

## `vpt print-file package.json`

vite-plus version upgrade is applied

```
{
  "name": "migration-upgrade-setup-skipped-default-pnpm",
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
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

## `vpt print-file pnpm-workspace.yaml`

pnpm settings consolidated by the version upgrade

```
overrides:
  vite: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
```

## `vpt stat-file .nvmrc --assert file`

.nvmrc is left untouched without --full

```
.nvmrc: file
```

## `vpt stat-file .node-version --assert-not file`

no .node-version is written

```
.node-version: missing
```

## `vpt print-file tsconfig.json`

tsconfig baseUrl is left untouched without --full

```
{
  "compilerOptions": {
    "target": "ES2023",
    "module": "NodeNext",
    "baseUrl": "."
  }
}
```
