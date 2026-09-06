# migration_upgrade_vite_plus_protocol_pin_npm

## `vp migrate --no-interactive`

deliberate vite-plus protocol pin must survive bootstrap

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
◇ Updated . to Vite+ <version>
• Node <version>  npm <version>
• Dependencies:
    vite   → <version>
• Package manager settings configured
! Warnings:
  - Vitest v5: 1 review item

package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
```

## `vpt print-file package.json`

file pin should remain while stale vitest config is removed

```
{
  "name": "migration-upgrade-vite-plus-protocol-pin-npm",
  "devDependencies": {
    "vite-plus": "file:../custom-vite-plus"
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
