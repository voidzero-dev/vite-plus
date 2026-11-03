# migration_oxlint_extends_workspace_inherited_alias

## `vpt rm packages/b/extra.ts`


## `vpt write-file packages/a/.oxlintrc.json '{"extends":["./alias.json"]}'`


## `node create-config-links.mjs --owner-alias`


## `cd packages/a && vp lint --threads=1 -c alias.json index.js`

The inherited alias loads the original owner config

```

  ⚠ eslint(no-debugger): `debugger` statement is not allowed
   ╭─[index.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 1 warning and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

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

## `vpt stat-file .oxlintrc.json --assert file`

The surviving inherited alias retains its target

```
.oxlintrc.json: file
```

## `vpt stat-file packages/a/alias.json --assert symlink`

```
packages/a/alias.json: symlink
```

## `vpt stat-file packages/a/.oxlintrc.json --assert missing`

```
packages/a/.oxlintrc.json: missing
```

## `cd packages/a && vp lint --threads=1 -c alias.json index.js`

The inherited alias still loads with the same diagnostic

```

  ⚠ eslint(no-debugger): `debugger` statement is not allowed
   ╭─[index.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 1 warning and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `cd packages/a && vp lint --threads=1 index.js`

The migrated consumer uses its shared module

```

  ⚠ eslint(no-debugger): `debugger` statement is not allowed
   ╭─[index.js:1:1]
 1 │ debugger;
   · ─────────
   ╰────
  help: Remove the debugger statement

Found 1 warning and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
