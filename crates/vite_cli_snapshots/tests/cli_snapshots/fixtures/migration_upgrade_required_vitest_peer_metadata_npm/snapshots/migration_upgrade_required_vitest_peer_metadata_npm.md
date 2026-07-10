# migration_upgrade_required_vitest_peer_metadata_npm

## `vp migrate --no-interactive`

clean checkout conservatively preserves existing Vitest

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

package-local Vitest and its shared override remain aligned

```
{
  "name": "migration-upgrade-required-vitest-peer-metadata-npm",
  "devDependencies": {
    "vite-plugin-gherkin": "0.2.0",
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

## `vpt mkdir node_modules`

simulate installed dependency metadata


## `vpt cp -r .fixture/vite-plugin-gherkin node_modules`


## `vp migrate --no-interactive`

metadata confirms the unnamed required Vitest peer

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
