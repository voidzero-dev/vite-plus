# migration_partially_installed_vite_plus

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

should finish core rewrites even when vite-plus is already installed in a pnpm project

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  0.1.24 → <version>
    vite              → <version>
• 1 file had imports rewritten
• Package manager settings configured
```

## `vpt print-file package.json`

scripts should be rewritten without adding package.json overrides

```
{
  "name": "manual-vp-migrate",
  "private": true,
  "version": "0.0.0",
  "type": "module",
  "scripts": {
    "dev": "vp dev",
    "build": "tsc -b && vp build",
    "lint": "vp lint .",
    "preview": "vp preview"
  },
  "dependencies": {
    "react": "^19.2.6",
    "react-dom": "^19.2.6"
  },
  "devDependencies": {
    "@types/node": "^24.12.3",
    "@types/react": "^19.2.14",
    "@types/react-dom": "^19.2.3",
    "@vitejs/plugin-react": "^6.0.1",
    "globals": "^17.6.0",
    "typescript": "~6.0.2",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `vpt print-file pnpm-workspace.yaml`

pnpm overrides and peerDependencyRules should be configured

```
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
overrides:
  vite: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
```

## `vpt print-file vite.config.ts`

vite imports should be rewritten

```
import { defineConfig } from 'vite-plus'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
})
```

## `vpt print-file tsconfig.app.json`

vite/client is preserved (issue #2004: tsconfig is not a vite.config)

```
{
  "compilerOptions": {
    "types": ["vite/client"]
  }
}
```

## `vpt stat-file AGENTS.md --assert-not file`

disabled agent setup should not write instructions

```
AGENTS.md: missing
```

## `vpt stat-file .vite-hooks --assert-not dir`

disabled hook setup should not write hooks

```
.vite-hooks: missing
```

## `vpt stat-file .vscode --assert-not dir`

disabled editor setup should not write editor config

```
.vscode: missing
```
