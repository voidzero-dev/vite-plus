# migration_standalone_yarn_below_catalog

## `vp migrate --no-interactive`

Yarn < 4.10.0 cannot resolve `catalog:`, so managed specs stay concrete

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.

⚠ Vite+ does not currently support Yarn Plug'n'Play (PnP).

✔ Switched Yarn to node-modules mode
◇ Migrated . to Vite+ <version>
• Node <version>  yarn <version>
• 2 config updates applied
• Package manager settings configured
```

## `vpt print-file package.json`

concrete specs: `vite` via the @voidzero-dev/vite-plus-core alias, no `catalog:` references

```
{
  "name": "migration-standalone-yarn-below-catalog",
  "scripts": {
    "test": "vp test run",
    "prepare": "vp config"
  },
  "devDependencies": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
    "vite-plus": "<version>"
  },
  "packageManager": "yarn@3.6.0",
  "resolutions": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
  }
}
```

## `vpt print-file .yarnrc.yml`

node-modules linker is configured, but no catalog field is written

```
nodeLinker: node-modules
npmPreapprovedPackages:
  - vitest
  - '@vitest/*'
```
