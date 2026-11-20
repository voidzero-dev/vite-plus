# migration_preserve_below_policy_node_pins

## `vp migrate --no-interactive`

upgrade the incompatible runtime pin without changing the public engine contract

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  0.1.21 → <version>
    vite              → <version>
• Package manager settings configured
```

## `vpt print-file .node-version`

upgraded from 24.3.0 to 24.11.0

```
24.11.0
```

## `vpt print-file package.json`

engines.node stays 24.x and devEngines.runtime node stays ^24 (preserved, not raised)

```
{
  "name": "migration-preserve-below-policy-node-pins",
  "devDependencies": {
    "vite": "catalog:vite-stack",
    "vite-plus": "catalog:vite-stack"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    },
    "runtime": [
      {
        "name": "node",
        "version": "^24"
      }
    ]
  },
  "engines": {
    "node": "24.x"
  }
}
```

## `vpt print-file pnpm-workspace.yaml`

catalog is migrated

```
packages:
  - .

catalogs:
  vite-stack:
    vite: npm:@voidzero-dev/vite-plus-core@<version>
    vite-plus: <version>
overrides:
  vite@*: catalog:vite-stack
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
```

## `vp migrate --no-interactive`

rerun preserves the upgraded runtime and public engine contract

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
