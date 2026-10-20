# migration_oxlint_extends_workspace_new_consumer

## `vpt rm packages/b/extra.ts`


## `vpt write-file shared/base.json '{"rules":{"no-console":"error"},"options":{"typeAware":false,"typeCheck":false}}'`


## `vpt json-edit .oxlintrc.json extends '["./shared/base.json"]'`


## `vp migrate --no-interactive --no-agent --no-editor --no-hooks`

```
VITE+ - The Unified Toolchain for the Web

▲ No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
◆ ✔ Merged .oxlintrc.json into vite.config.mjs
● Formatting code...
● Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
• 2 config updates applied
```

## `vpt stat-file shared/base.ts --assert file`

```
shared/base.ts: file
```

## `vpt write-file packages/c/package.json '{"name":"new-consumer","type":"module"}'`


## `vpt write-file packages/c/vite.config.mjs 'export default {};
'`


## `vpt write-file packages/c/.oxlintrc.json '{"extends":["../../shared/base.json"]}'`


## `vpt write-file packages/c/index.js 'console.log("hello");'`


## `cd packages/c && vp lint --threads=1 -c .oxlintrc.json index.js`

A new workspace uses the retained shared JSON

**Exit code:** 1

```

  × eslint(no-console): Unexpected console statement.
   ╭─[index.js:1:1]
 1 │ console.log("hello");
   · ───────────
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp migrate --no-interactive --no-agent --no-editor --no-hooks`

An upgrade does not treat shared modules as a pending migration for the new workspace

```
VITE+ - The Unified Toolchain for the Web

▲ No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
This project is already using Vite+! Happy coding!
```

## `vpt print-file packages/c/vite.config.mjs`

The unmarked workspace keeps its original Vite config

```
export default {};
```

## `vpt print-file packages/c/.oxlintrc.json`

Its newly authored JSON config remains intact

```
{"extends":["../../shared/base.json"]}
```

## `cd packages/c && vp lint --threads=1 -c .oxlintrc.json index.js`

The JSON config retains the same lint output

**Exit code:** 1

```

  × eslint(no-console): Unexpected console statement.
   ╭─[index.js:1:1]
 1 │ console.log("hello");
   · ───────────
   ╰────
  help: Delete this console statement.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
