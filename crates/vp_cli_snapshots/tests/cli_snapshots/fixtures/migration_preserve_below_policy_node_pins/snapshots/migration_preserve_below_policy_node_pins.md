# migration_preserve_below_policy_node_pins

## `vp migrate --no-interactive`

reject a Node pin below the new CLI engine before modifying the project; never raise it silently

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 2 review items (1 block dependency updates)

.node-version
  1:1 BLOCK [node-runtime] .node-version (24.3.0) cannot run Vite+ with Vitest v5. Select Node ^22.18.0 || ^24.11.0 || >=26.0.0; use vp env pin 22 --force for a runtime pin. Do not widen a library's engines.node contract automatically.

package.json
  1:1 REVIEW [node-runtime] engines.node (24.x) includes unsupported test runtimes. Pin the test/CI runtime to Node ^22.18.0 || ^24.11.0 || >=26.0.0; keep the library's public engine contract separate.
Resolve the blocking Vitest v5 findings, then re-run `vp migrate`. No project files were changed.
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

catalog remains unchanged after failed preflight

```
packages:
  - .

catalogs:
  vite-stack:
    vite: npm:@voidzero-dev/vite-plus-core@<version>
    vitest: npm:@voidzero-dev/vite-plus-test@0.1.21
    vite-plus: <version>
```

## `vpt write-file .node-version '24.11.0
'`

the user explicitly selects a supported runtime


## `vp migrate --no-interactive`

migration can proceed without changing the public engine contract

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

package.json
  1:1 REVIEW [node-runtime] engines.node (24.x) includes unsupported test runtimes. Pin the test/CI runtime to Node ^22.18.0 || ^24.11.0 || >=26.0.0; keep the library's public engine contract separate.
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  0.1.21 → <version>
    vite              → <version>
• Package manager settings configured
! Warnings:
  - Vitest v5: 1 review item

package.json
  1:1 REVIEW [node-runtime] engines.node (24.x) includes unsupported test runtimes. Pin the test/CI runtime to Node ^22.18.0 || ^24.11.0 || >=26.0.0; keep the library's public engine contract separate.
```
