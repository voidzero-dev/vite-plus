# new_vite_monorepo_bun

## `vp create vite:monorepo --no-interactive --package-manager bun --git`

create monorepo with bun


## `vpt list-dir vite-plus-monorepo`

check files created

```
AGENTS.md
README.md
apps
package.json
packages
tsconfig.json
vite.config.ts
```

## `vpt print-file vite-plus-monorepo/package.json`

check package.json with catalog

```
{
  "name": "vite-plus-monorepo",
  "version": "0.0.0",
  "private": true,
  "workspaces": [
    "packages/*",
    "apps/*",
    "tools/*"
  ],
  "type": "module",
  "scripts": {
    "ready": "vp check && vp run -r test && vp run -r build",
    "dev": "vp run website#dev",
    "prepare": "vp config"
  },
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "overrides": {
    "vite": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "bun",
      "version": "<version>",
      "onFail": "download"
    }
  },
  "engines": {
    "node": ">=22.18.0"
  },
  "catalog": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
    "vite-plus": "<version>"
  }
}
```

## `vpt stat-file vite-plus-monorepo/pnpm-workspace.yaml --assert-not file`

verify no pnpm config

```
vite-plus-monorepo/pnpm-workspace.yaml: missing
```

## `vpt stat-file vite-plus-monorepo/.yarnrc.yml --assert-not file`

verify no yarn config

```
vite-plus-monorepo/.yarnrc.yml: missing
```

## `vpt stat-file vite-plus-monorepo/.git --assert dir`

check git init

```
vite-plus-monorepo/.git: dir
```

## `vpt list-dir vite-plus-monorepo/apps`

check apps directory

```
website
```

## `vpt print-file vite-plus-monorepo/apps/website/package.json`

check website uses catalog:

```
{
  "name": "website",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vp dev",
    "build": "tsc && vp build",
    "preview": "vp preview"
  },
  "devDependencies": {
    "typescript": "^7.0.2",
    "vite-plus": "catalog:"
  }
}
```

## `vpt print-file vite-plus-monorepo/packages/utils/package.json`

check utils normalizes vite-plus to catalog:

```
{
  "name": "utils",
  "version": "0.0.0",
  "description": "A starter for creating a TypeScript package.",
  "homepage": "https://github.com/author/library#readme",
  "bugs": {
    "url": "https://github.com/author/library/issues"
  },
  "license": "MIT",
  "author": "Author Name <author.name@mail.com>",
  "repository": {
    "type": "git",
    "url": "git+https://github.com/author/library.git"
  },
  "files": [
    "dist"
  ],
  "type": "module",
  "exports": {
    ".": "./dist/index.mjs",
    "./package.json": "./package.json"
  },
  "publishConfig": {
    "access": "public"
  },
  "scripts": {
    "build": "vp pack",
    "dev": "vp pack --watch",
    "test": "vp test",
    "check": "vp check",
    "prepublishOnly": "vp run build"
  },
  "devDependencies": {
    "@types/node": "^26.1.1",
    "bumpp": "^11.1.0",
    "typescript": "^7.0.2",
    "vite-plus": "catalog:"
  }
}
```
