# migration_oxlint_extends

Regression for #3089: a fresh migration must preserve JSON extends, inherited rules, and a type-safe Vite config.

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

● Formatting code...
● Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  npm <version>
✓ Dependencies installed in <duration>
• 1 file had imports rewritten
! Warnings:
  - Kept .oxlintrc.json because JSON extends paths cannot be merged into vite.config.ts. Use vp lint -c ".oxlintrc.json", or migrate its extends entries to config objects manually.
```

## `vpt print-file .oxlintrc.json`

```
{
  "$schema": "./node_modules/oxlint/configuration_schema.json",
  "extends": ["./shared/oxlint-config.json"]
}
```

## `vpt print-file shared/oxlint-config.json`

```
{
  "rules": { "no-debugger": "error" }
}
```

## `vpt print-file vite.config.ts`

```
import { defineConfig } from "vite-plus";

export default defineConfig({
  fmt: {},
});
```

## `vp lint`

```
VITE+ - The Unified Toolchain for the Web

note: You are running `vp lint` as a Vite+ built-in command. If you meant to run the lint npm script, use `vpr lint` instead.
Found 0 warnings and 0 errors.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```

## `vp lint -c .oxlintrc.json`

```
VITE+ - The Unified Toolchain for the Web

note: You are running `vp lint` as a Vite+ built-in command. If you meant to run the lint npm script, use `vpr lint` instead.
Found 0 warnings and 0 errors.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```

## `vp exec tsc -p .`

```
VITE+ - The Unified Toolchain for the Web
```

## `vpt write-file src/index.ts 'debugger;
export const value = 1;
'`


## `vp lint -c .oxlintrc.json`

the command in the migration warning must enforce the inherited rule as an error

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

note: You are running `vp lint` as a Vite+ built-in command. If you meant to run the lint npm script, use `vpr lint` instead.

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[src/index.ts:1:1]
 1 │ debugger;
   · ─────────
 2 │ export const value = 1;
   ╰────
  help: Remove the debugger statement

Found 0 warnings and 1 error.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```

## `vpt write-file src/index.ts 'export const value = 1;
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  npm <version>
! Warnings:
  - Kept .oxlintrc.json because JSON extends paths cannot be merged into vite.config.ts. Use vp lint -c ".oxlintrc.json", or migrate its extends entries to config objects manually.
```

## `vpt print-file .oxlintrc.json`

```
{
  "$schema": "./node_modules/oxlint/configuration_schema.json",
  "extends": ["./shared/oxlint-config.json"]
}
```

## `vpt print-file vite.config.ts`

```
import { defineConfig } from "vite-plus";

export default defineConfig({
  fmt: {},
});
```

## `vp lint`

```
VITE+ - The Unified Toolchain for the Web

note: You are running `vp lint` as a Vite+ built-in command. If you meant to run the lint npm script, use `vpr lint` instead.
Found 0 warnings and 0 errors.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```

## `vp lint -c .oxlintrc.json`

```
VITE+ - The Unified Toolchain for the Web

note: You are running `vp lint` as a Vite+ built-in command. If you meant to run the lint npm script, use `vpr lint` instead.
Found 0 warnings and 0 errors.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```

## `vp exec tsc -p .`

```
VITE+ - The Unified Toolchain for the Web
```

## `vpt write-file src/index.ts 'debugger;
export const value = 1;
'`


## `vp lint -c .oxlintrc.json`

the retained JSON config must still enforce the inherited rule after a second migration

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

note: You are running `vp lint` as a Vite+ built-in command. If you meant to run the lint npm script, use `vpr lint` instead.

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[src/index.ts:1:1]
 1 │ debugger;
   · ─────────
 2 │ export const value = 1;
   ╰────
  help: Remove the debugger statement

Found 0 warnings and 1 error.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```
