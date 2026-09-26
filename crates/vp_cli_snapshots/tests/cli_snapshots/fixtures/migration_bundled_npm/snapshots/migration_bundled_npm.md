# migration_bundled_npm

## `vp migrate --no-interactive --no-hooks --no-agent`

An npm lockfile without a package-manager pin migrates using Node's bundled npm version

```
VITE+ - The Unified Toolchain for the Web

No package manager is declared in package.json; using npm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
◇ Migrated . to Vite+ <version>
• Node <version>  npm <version>
• 1 config update applied
```

## `node assert-unpinned.cjs`

Migration leaves a lockfile-only npm project unpinned

```
Migration leaves npm unpinned
```
