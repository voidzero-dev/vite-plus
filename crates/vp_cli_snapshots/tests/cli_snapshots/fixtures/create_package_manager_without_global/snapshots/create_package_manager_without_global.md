# create_package_manager_without_global

Without a global install, `vp create` defaults to and only offers the package managers on PATH.

## `vpt rm ../home/.vite-plus/config.json`

the global install's config is the local CLI's only sign of a global install


## `vpt chmod +x system-bin/npm`


## `PATH=${workspace}/system-bin${PATH_SEPARATOR}${PATH} vp create vite:monorepo --no-interactive --directory default-monorepo`

pnpm is missing, so the default falls back to npm

```
◇ Scaffolded default-monorepo with Vite+ monorepo
• Node <version>  npm <version>
→ Next: cd default-monorepo && vp run
```

## `PATH=${workspace}/system-bin${PATH_SEPARATOR}${PATH} vp create vite:monorepo --directory selected-monorepo --no-agent --no-editor --no-git`

missing package managers are shown but cannot be selected

**→ expect-milestone:** `select:select:2`

```
VITE+ - The Unified Toolchain for the Web

› Which package manager would you like to use?
    pnpm: not installed
    yarn: not installed
  › npm
    bun: not installed
  ↑/↓ navigate · Enter confirm
```

**← write-key:** `enter`

```
VITE+ - The Unified Toolchain for the Web

◇ Which package manager would you like to use?
  npm

◇ Scaffolded selected-monorepo with Vite+ monorepo
• Node <version>  npm <version>
→ Next: cd selected-monorepo && vp run
```

## `vpt stat-file ../home/.vite-plus/package_manager`

no package manager was downloaded

```
../home/.vite-plus/package_manager: missing
```
