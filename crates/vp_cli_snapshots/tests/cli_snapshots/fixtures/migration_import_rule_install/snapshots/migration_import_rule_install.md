# migration_import_rule_install

After installation makes dynamic lint settings available, the upgrade keeps a direct Vitest dependency for the excluded template and removes it from the app.

## `vpt write-file pnpm-workspace.yaml 'packages:
  - templates
  - packages/*
nodeLinker: isolated
hoist: false
'`


## `vpt write-file templates/package.json '{"name":"template","private":true,"type":"module","devDependencies":{"vitest":"^5.0.0"}}
'`


## `vpt write-file packages/app/package.json '{"name":"app","private":true,"type":"module","devDependencies":{"vitest":"^5.0.0"}}
'`


## `vpt cp unit.test.ts packages/app/unit.test.ts`


## `vpt stat-file node_modules --assert missing`

```
node_modules: missing
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

● Formatting code...
● Code formatted
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
✓ Dependencies installed in <duration>
• 2 files had imports rewritten
• Package manager settings configured
```

## `vpt stat-file node_modules/vite-plus/package.json --assert file`

```
node_modules/vite-plus/package.json: file
```

## `vpt print-file templates/unit.test.ts`

```
import { test } from "vitest";
test("template", () => {});
```

## `vpt print-file templates/package.json`

```
{
  "name": "template",
  "private": true,
  "type": "module",
  "devDependencies": {
    "vitest": "catalog:"
  }
}
```

## `vpt print-file packages/app/unit.test.ts`

```
import { test } from "vite-plus/test";
test("application", () => {});
```

## `vpt print-file packages/app/package.json`

```
{
  "name": "app",
  "private": true,
  "type": "module",
  "devDependencies": {}
}
```

## `vpt print-file unit.test.ts`

```
import { test } from "vite-plus/test";
test("application", () => {});
```

## `vpt print-file package.json`

```
{
  "name": "migration-import-rule-install",
  "private": true,
  "type": "module",
  "scripts": {
    "build": "vp build"
  },
  "devDependencies": {
    "lint-policy": "file:./lint-policy",
    "vite-plus": "catalog:"
  },
  "packageManager": "pnpm@12.11.2"
}
```

## `vpt stat-file templates/node_modules/vitest/package.json --assert file`

```
templates/node_modules/vitest/package.json: file
```

## `cd templates && node --input-type=module -e 'import { test } from '\''vitest'\''; console.log(typeof test);'`

```
function
```

## `vpt stat-file packages/app/node_modules/vitest/package.json --assert missing`

```
packages/app/node_modules/vitest/package.json: missing
```

## `vp install --frozen-lockfile --ignore-scripts`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
