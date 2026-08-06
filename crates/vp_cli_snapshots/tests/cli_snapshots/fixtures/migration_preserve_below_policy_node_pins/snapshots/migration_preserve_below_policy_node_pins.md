# migration_preserve_below_policy_node_pins

## `vp migrate --no-interactive`

existing Vite+ project: a Node pin below the supported range is preserved, not raised (native binding supports Node >=20)

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

stays 24.3.0

```
24.3.0
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

vite-stack catalog updated to the migration target

```
packages:
  - .

catalogs:
  vite-stack:
    vite: npm:@voidzero-dev/vite-plus-core@<version>
    vite-plus: <version>
overrides:
  vite: catalog:vite-stack
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
```
