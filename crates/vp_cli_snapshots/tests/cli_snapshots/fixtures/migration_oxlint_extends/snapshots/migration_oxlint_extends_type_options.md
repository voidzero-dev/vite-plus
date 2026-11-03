# migration_oxlint_extends_type_options

## `vpt json-edit shared/base.json options '{"typeAware":true,"typeCheck":true}'`


## `vpt json-edit .oxlintrc.json rules.typescript/no-floating-promises '"error"'`


## `vpt write-file tsconfig.json '{"compilerOptions":{"target":"ESNext","strict":true},"include":["*.ts"]}'`


## `vpt write-file type-error.ts 'export const value: string = 1;
'`


## `vpt write-file type-aware.ts 'Promise.resolve(1);
'`


## `vp lint --threads=1 -c .oxlintrc.json type-error.ts`

Inherited typeCheck reports the original TypeScript diagnostic

**Exit code:** 1

```

  × typescript(TS2322): Type 'number' is not assignable to type 'string'.
   ╭─[type-error.ts:1:14]
 1 │ export const value: string = 1;
   ·              ─────
   ╰────

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --threads=1 -c .oxlintrc.json type-aware.ts`

Inherited typeAware enables no-floating-promises

**Exit code:** 1

```

  × typescript(no-floating-promises): Promises must be awaited, add void operator to ignore.
   ╭─[type-aware.ts:1:1]
 1 │ Promise.resolve(1);
   · ─────────┬─────────
   ·          ╰── This unhandled promise-like value has type `Promise<number>`.
   ╰────
  help: The promise must end with a call to .catch, or end with a call to .then with a rejection handler, or be explicitly marked as ignored with the `void` operator.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp migrate --no-interactive --no-agent --no-editor --no-hooks`

```
VITE+ - The Unified Toolchain for the Web

● Formatting code...
● Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
• 2 config updates applied
```

## `vpt print-file vite.config.ts`

Local options stay empty so inherited settings remain live

```
import oxlintConfig1 from "./shared/nested/first.js";
import oxlintConfig2 from "./shared/second.js";
import { defineConfig } from "vite-plus";

export default defineConfig({
  fmt: {},
  lint: {
    extends: [oxlintConfig1, oxlintConfig2],
    ignorePatterns: ["shared/**", "vite.config.ts"],
    rules: {
      "local/no-foo": "error",
      "no-alert": "off",
      "typescript/no-floating-promises": "error",
      "vite-plus/prefer-vite-plus-imports": "error",
    },
    options: {},
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }],
  },
});
```

## `vp lint --threads=1 type-error.ts`

Imported typeCheck produces the same diagnostic and exit code

**Exit code:** 1

```

  × typescript(TS2322): Type 'number' is not assignable to type 'string'.
   ╭─[type-error.ts:1:14]
 1 │ export const value: string = 1;
   ·              ─────
   ╰────

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --threads=1 type-aware.ts`

Imported typeAware produces the same diagnostic and exit code

**Exit code:** 1

```

  × typescript(no-floating-promises): Promises must be awaited, add void operator to ignore.
   ╭─[type-aware.ts:1:1]
 1 │ Promise.resolve(1);
   · ─────────┬─────────
   ·          ╰── This unhandled promise-like value has type `Promise<number>`.
   ╰────
  help: The promise must end with a call to .catch, or end with a call to .then with a rejection handler, or be explicitly marked as ignored with the `void` operator.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vpt replace-file-content shared/base.ts 'typeAware: true' 'typeAware: false'`

```
```

## `vpt replace-file-content shared/base.ts 'typeCheck: true' 'typeCheck: false'`

```
```

## `vp lint --threads=1 type-error.ts`

Disabling the shared options removes the TypeScript diagnostic

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --threads=1 type-aware.ts`

Disabling the shared options removes the type-aware diagnostic

```
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
