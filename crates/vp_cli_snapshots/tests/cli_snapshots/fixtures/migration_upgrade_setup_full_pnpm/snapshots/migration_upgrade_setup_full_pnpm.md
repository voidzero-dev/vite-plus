# migration_upgrade_setup_full_pnpm

## `vpt chmod +x fix-baseurl.mjs`

stub the baseUrl fixer so the dlx stays offline

```
```

## `vp migrate --full --no-interactive`

existing Vite+ project: upgrade plus the full setup

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  0.1.21 → <version>
    vite              → <version>
• 2 config updates applied
• Node version manager file migrated to .node-version
• Package manager settings configured
→ Manual follow-up:
  - Remove the "volta" field from package.json
```

## `vpt print-file package.json`

vite-plus version upgrade is applied

```
{
  "name": "migration-upgrade-setup-full-pnpm",
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  },
  "volta": {
    "node": "24.11.0"
  },
  "scripts": {
    "prepare": "vp config"
  }
}
```

## `vpt print-file .node-version`

volta.node is migrated to .node-version by the full setup

```
24.11.0
```

## `vpt print-file tsconfig.json`

tsconfig baseUrl is removed by the full setup

```
{
  "compilerOptions": {
    "target": "ES2023",
    "module": "NodeNext"
  }
}
```
