# migration_standalone_yarn_below_catalog_install

Yarn below 4.10.0 refuses to run with an rc key it does not know, so the migrated project must still install.

## `vpt write-file .yarnrc.yml 'nodeLinker: node-modules
'`


## `vpt write-file .gitignore 'node_modules
'`

keeps the import rewrite out of installed packages


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

● Formatting code...
● Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  yarn <version>
✓ Dependencies installed in <duration>
• 1 config update applied
```

## `vpt print-file .yarnrc.yml`

```
nodeLinker: node-modules
```
