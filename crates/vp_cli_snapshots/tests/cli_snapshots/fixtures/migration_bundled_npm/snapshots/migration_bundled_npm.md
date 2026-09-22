# migration_bundled_npm

## `vp migrate --no-interactive --no-hooks --no-agent`

An npm lockfile without a package-manager pin migrates using Node's bundled npm version

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  npm <version>
• 1 config update applied
```

## `node assert-npm-version.cjs`

Migration persists a concrete npm version matching the selected Node runtime

```
Migration pins the bundled npm version
```
