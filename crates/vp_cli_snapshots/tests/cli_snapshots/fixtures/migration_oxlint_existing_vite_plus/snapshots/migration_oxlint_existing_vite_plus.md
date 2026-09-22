# migration_oxlint_existing_vite_plus

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`


## `vpt print-file package.json packages/app/package.json`

Migration removes unused plugin API dependencies from the root and workspace package.

```
{
  "name": "migration-oxlint-existing-vite-plus",
  "private": true,
  "workspaces": [
    "packages/*"
  ],
  "type": "module",
  "devDependencies": {
    "vite-plus": "<version>"
  },
  "overrides": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
  },
  "packageManager": "npm@11.11.1"
}
{
  "name": "app",
  "private": true,
  "type": "module",
  "devDependencies": {
    "vite-plus": "<version>"
  }
}
```

## `vpt print-file plugin.ts packages/app/plugin.ts`

Imports and exports use the bundled plugin API.

```
import { definePlugin, defineRule, type Context, type ESTree } from "vite-plus/lint/plugins";
export { definePlugin, defineRule } from "vite-plus/lint/plugins";
```

## `vpt json-edit package.json devDependencies.@oxlint/plugins 1.79.0`


## `vp install --ignore-scripts`


## `node --input-type=module -e 'import fs from '\''node:fs'\''; import assert from '\''node:assert/strict'\''; const lock = JSON.parse(fs.readFileSync('\''package-lock.json'\'', '\''utf8'\'')); assert.equal(lock.packages['\'''\''].devDependencies['\''@oxlint/plugins'\''], '\''1.79.0'\'');'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

An already migrated project still cleans up a leftover dependency and updates its lockfile.

```
VITE+ - The Unified Toolchain for the Web

Formatting code...

Code formatted
◇ Updated . to Vite+ <version>
• Node <version>  npm <version>
✓ Dependencies installed in <duration>
```

## `node --input-type=module -e 'import fs from '\''node:fs'\''; import assert from '\''node:assert/strict'\''; for (const file of ['\''package.json'\'', '\''packages/app/package.json'\'']) { const pkg = JSON.parse(fs.readFileSync(file, '\''utf8'\'')); assert.equal(pkg.devDependencies['\''@oxlint/plugins'\''], undefined); } const lock = JSON.parse(fs.readFileSync('\''package-lock.json'\'', '\''utf8'\'')); for (const name of ['\'''\'', '\''packages/app'\'']) { assert.equal(lock.packages[name].devDependencies['\''@oxlint/plugins'\''], undefined); } console.log('\''No direct @oxlint/plugins dependencies remain in manifests or lockfile.'\'');'`

The cleanup must run an install even when it changes no imports or toolchain versions.

```
No direct @oxlint/plugins dependencies remain in manifests or lockfile.
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

A further migration leaves the project unchanged.

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
