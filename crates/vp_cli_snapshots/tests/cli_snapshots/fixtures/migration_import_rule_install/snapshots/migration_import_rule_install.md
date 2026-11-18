# migration_import_rule_install

An existing Vite+ project retries dynamic lint settings after the upgrade installs missing dependencies.

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
• 1 file had imports rewritten
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
