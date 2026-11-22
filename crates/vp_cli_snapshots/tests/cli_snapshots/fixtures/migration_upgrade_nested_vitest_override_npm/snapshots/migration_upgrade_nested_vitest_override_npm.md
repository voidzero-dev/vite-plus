# migration_upgrade_nested_vitest_override_npm

## `vp migrate --no-interactive`

add the Vite alias to the nested override

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  npm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• Package manager settings configured
```

## `vpt print-file package.json`

preserve the Rollup and Vitest runner overrides

```
{
  "name": "migration-upgrade-nested-vitest-override-npm",
  "devDependencies": {
    "vite-plus": "<version>"
  },
  "overrides": {
    "vite": {
      "rollup": "^4.0.0",
      ".": "npm:@voidzero-dev/vite-plus-core@<version>"
    },
    "vitest": {
      "@vitest/runner": "<version>"
    }
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

## `vp migrate --no-interactive`

nested override must not make migration permanently pending

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
