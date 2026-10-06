# migration_vitest_unmanaged_override

## `vp migrate --no-interactive`

vitest omitted from managed overrides must remain user-owned

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
```

## `vpt print-file package.json`

user's Vitest and @vitest/ui stay direct devDependencies (not converted to catalog:), so they remain user-owned

```
{
  "name": "migration-vitest-unmanaged-override",
  "scripts": {
    "test": "vp test",
    "prepare": "vp config"
  },
  "devDependencies": {
    "@vitest/ui": "<version>",
    "vite": "catalog:",
    "vitest": "<version>",
    "vite-plus": "catalog:"
  }
}
```

## `vpt print-file pnpm-workspace.yaml`

no vitest catalog or override should be introduced

```
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@latest
  vite-plus: <version>
overrides:
  vite@*: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
```

## `vp migrate --no-interactive`

unmanaged Vitest ecosystem versions remain stable on rerun

```
VITE+ - The Unified Toolchain for the Web

No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite               → <version>
    vitest      4.0.13 → <version>
    @vitest/ui  4.0.13 → <version>
```
