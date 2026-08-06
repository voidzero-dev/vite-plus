# migration_upgrade_pnpm9_overrides

## `vp migrate --no-interactive`

pnpm 9.5-10.6.1: settings stay in package.json, catalog is still rewritten off the wrappers

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus            0.1.20 → <version>
    vite                        → <version>
    vitest               0.1.20 → <version>
    @vitest/coverage-v8  4.1.6  → <version>
• Package manager settings configured
```

## `vpt print-file package.json`

pnpm.overrides stay catalog: (not inlined to a version)

```
{
  "name": "migration-upgrade-pnpm9-overrides",
  "devDependencies": {
    "@vitest/coverage-v8": "catalog:",
    "vite": "catalog:",
    "vite-plus": "catalog:",
    "vitest": "catalog:"
  },
  "packageManager": "pnpm@9.15.9",
  "pnpm": {
    "overrides": {
      "vite": "catalog:",
      "vitest": "catalog:"
    },
    "peerDependencyRules": {
      "allowAny": [
        "vite",
        "vitest"
      ],
      "allowedVersions": {
        "vite": "*",
        "vitest": "*"
      }
    }
  }
}
```

## `vpt print-file pnpm-workspace.yaml`

catalog rewritten off the vite-plus-test wrapper; overrides remain in package.json (< 10.6.2)

```
packages:
  - .

catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
  vitest: <version>
  '@vitest/coverage-v8': <version>
```
