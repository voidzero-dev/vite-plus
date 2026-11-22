# migration_oxlint_published_plugin_rerun

## `vp migrate --no-interactive`

this package declares `oxlint` as a runtime dependency, which marks it a published Oxlint plugin

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
```

## `vp migrate --no-interactive`

a second run finds nothing left to do. The kept dependency must not make the migrated project look unfinished

```
VITE+ - The Unified Toolchain for the Web

▲ No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
This project is already using Vite+! Happy coding!
```

## `vpt print-file package.json`

the `oxlint` runtime dependency survives both runs. Consumers of a published plugin install it through this entry, so the rerun path keeps it as the first run does

```
{
  "name": "oxlint-plugin-example",
  "version": "1.0.0",
  "dependencies": {
    "oxlint": "^1.0.0"
  },
  "devDependencies": {
    "vite-plus": "catalog:"
  },
  "scripts": {
    "prepare": "vp config"
  }
}
```
