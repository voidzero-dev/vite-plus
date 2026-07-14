# new_vite_monorepo

## `vp create vite:monorepo --no-interactive --git --editor vscode`

create monorepo with default values


## `vpt list-dir vite-plus-monorepo`

check files created

```
AGENTS.md
README.md
apps
package.json
packages
pnpm-workspace.yaml
tsconfig.json
vite.config.ts
```

## `vpt print-file vite-plus-monorepo/package.json`

check package.json

```
{
  "name": "vite-plus-monorepo",
  "version": "0.0.0",
  "private": true,
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
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  },
  "engines": {
    "node": ">=22.18.0"
  }
}
```

## `vpt print-file vite-plus-monorepo/vite.config.ts`

check vite config has cache enabled

```
import { defineConfig } from "vite-plus";

export default defineConfig({
  staged: {
    "*": "vp check --fix",
  },
  fmt: {},
  lint: {
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }],
    rules: { "vite-plus/prefer-vite-plus-imports": "error" },
    options: { typeAware: true, typeCheck: true },
  },
  run: {
    cache: true,
  },
});
```

## `vpt print-file vite-plus-monorepo/pnpm-workspace.yaml`

check workspace config

```
packages:
  - apps/*
  - packages/*
  - tools/*

catalogMode: prefer

catalog:
  "@types/node": ^24
  typescript: ^7.0.2
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
overrides:
  vite: "catalog:"
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: "*"
```

## `vpt stat-file vite-plus-monorepo/.gitignore --assert file`

verify gitignore renamed from _gitignore

```
vite-plus-monorepo/.gitignore: file
```

## `vpt stat-file vite-plus-monorepo/.yarnrc.yml --assert-not file`

verify no yarn config for pnpm

```
vite-plus-monorepo/.yarnrc.yml: missing
```

## `vpt stat-file vite-plus-monorepo/.git --assert dir`

check git init

```
vite-plus-monorepo/.git: dir
```

## `vp create vite:monorepo --interactive --verbose --no-git --no-hooks --no-agent --no-editor --package-manager pnpm --directory verbose-no-git-monorepo`

explicit --no-git should skip verbose monorepo git prompt


## `vpt stat-file verbose-no-git-monorepo/.git --assert-not dir`

check verbose --no-git is respected

```
verbose-no-git-monorepo/.git: missing
```

## `vpt stat-file vite-plus-monorepo/.vscode/settings.json --assert file`

check VS Code settings created

```
vite-plus-monorepo/.vscode/settings.json: file
```

## `vpt stat-file vite-plus-monorepo/.vscode/extensions.json --assert file`

check VS Code extensions created

```
vite-plus-monorepo/.vscode/extensions.json: file
```

## `node check-trackable.cjs vite-plus-monorepo .vscode/settings.json`

check VS Code settings are trackable

```
.vscode/settings.json trackable
```

## `node check-trackable.cjs vite-plus-monorepo .vscode/extensions.json`

check VS Code extensions are trackable

```
.vscode/extensions.json trackable
```

## `vpt list-dir vite-plus-monorepo/apps`

check apps directory created

```
website
```

## `vpt print-file vite-plus-monorepo/apps/website/package.json`

check website keeps aliased vite for pnpm (workspace override stays effective)

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
    "vite": "catalog:",
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
    "vite": "catalog:",
    "vite-plus": "catalog:"
  }
}
```

## `cd vite-plus-monorepo && vp create --no-interactive vite:application`

create application in non-interactive mode


## `vpt list-dir vite-plus-monorepo/apps`

check apps directory created

```
vite-plus-application
website
```

## `vpt list-dir vite-plus-monorepo/apps/vite-plus-application/package.json`

check vite-plus-application package.json

```
vite-plus-monorepo/apps/vite-plus-application/package.json
```

## `vpt stat-file vite-plus-monorepo/apps/vite-plus-application/.vscode --assert missing`

monorepo package create without --editor should not write VS Code config

```
vite-plus-monorepo/apps/vite-plus-application/.vscode: missing
```

## `cd vite-plus-monorepo && vp create --no-interactive vite:application --directory apps/no-editor --no-editor`

create application with explicit editor opt-out inside monorepo


## `vpt stat-file vite-plus-monorepo/apps/no-editor/.vscode --assert missing`

--no-editor should not write VS Code config

```
vite-plus-monorepo/apps/no-editor/.vscode: missing
```

## `cd vite-plus-monorepo && vp create --no-interactive vite:application --directory apps/editor-opt-in --editor vscode`

create application with explicit editor opt-in inside monorepo


## `vpt stat-file vite-plus-monorepo/apps/editor-opt-in/.vscode/settings.json --assert file`

explicit --editor should write VS Code settings

```
vite-plus-monorepo/apps/editor-opt-in/.vscode/settings.json: file
```

## `vpt stat-file vite-plus-monorepo/apps/editor-opt-in/.vscode/extensions.json --assert file`

explicit --editor should write VS Code extensions

```
vite-plus-monorepo/apps/editor-opt-in/.vscode/extensions.json: file
```

## `cd vite-plus-monorepo && vp create --no-interactive vite:library`

create library in non-interactive mode


## `vpt list-dir vite-plus-monorepo/packages/vite-plus-library/package.json`

check vite-plus-library package.json

```
vite-plus-monorepo/packages/vite-plus-library/package.json
```

## `vpt stat-file vite-plus-monorepo/packages/vite-plus-library/.vscode --assert missing`

monorepo package create without --editor should not write VS Code config

```
vite-plus-monorepo/packages/vite-plus-library/.vscode: missing
```

## `cd vite-plus-monorepo && vp create --no-interactive vite:generator`

create generator in non-interactive mode


## `vpt list-dir vite-plus-monorepo/tools`

check tools directory created

```
vite-plus-generator
```

## `vpt print-file vite-plus-monorepo/tools/vite-plus-generator/package.json`

check vite-plus-generator package.json

```
{
  "name": "vite-plus-generator",
  "version": "0.0.0",
  "private": true,
  "description": "A starter for creating a Vite+ code generator.",
  "keywords": [
    "vite-plus-generator"
  ],
  "bin": "./bin/index.ts",
  "type": "module",
  "scripts": {
    "test": "vp test",
    "dev": "node bin/index.ts"
  },
  "dependencies": {
    "bingo": "^0.9.3",
    "zod": "^3.25.76"
  },
  "devDependencies": {
    "@types/node": "catalog:",
    "typescript": "catalog:"
  },
  "engines": {
    "node": ">=22.18.0"
  }
}
```

## `vp create vite:monorepo --no-interactive --directory my-vite-plus-monorepo`

create monorepo with custom directory


## `vpt list-dir my-vite-plus-monorepo`

check files created

```
AGENTS.md
README.md
apps
package.json
packages
pnpm-workspace.yaml
tsconfig.json
vite.config.ts
```

## `vpt print-file my-vite-plus-monorepo/package.json`

check package.json

```
{
  "name": "my-vite-plus-monorepo",
  "version": "0.0.0",
  "private": true,
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
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  },
  "engines": {
    "node": ">=22.18.0"
  }
}
```
