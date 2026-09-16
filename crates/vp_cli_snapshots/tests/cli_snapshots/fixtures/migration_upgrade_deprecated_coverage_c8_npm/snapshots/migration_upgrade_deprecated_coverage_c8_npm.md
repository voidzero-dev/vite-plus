# migration_upgrade_deprecated_coverage_c8_npm

## `vp migrate --no-interactive`

deprecated coverage-c8 has an independent version line

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  npm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
    vitest     4.1.8  → <version>
• Package manager settings configured
```

## `vpt print-file package.json`

coverage-c8 must not be rewritten to a nonexistent Vitest 4 version

```
{
  "name": "migration-upgrade-deprecated-coverage-c8-npm",
  "devDependencies": {
    "@vitest/coverage-c8": "^0.33.0",
    "vite-plus": "<version>",
    "vitest": "<version>"
  },
  "overrides": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
    "vitest": "<version>"
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
