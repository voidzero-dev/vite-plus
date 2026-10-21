# migration_oxlint_extends_workspace_symlink_retry

## `vpt rm packages/a/.oxlintrc.json`


## `vpt rm packages/b/.oxlintrc.json`


## `vpt write-file shared/lint.json '{"extends":["./base.json"],"rules":{"no-console":"error"},"options":{"typeAware":false,"typeCheck":false}}'`


## `vpt write-file shared/base.json {}`


## `vpt write-file packages/a/index.js 'console.log("hello");'`


## `vpt write-file packages/b/index.js 'console.log("hello");'`


## `node create-config-links.mjs --with-base`


## `vpt write-file packages/b/vite.config.mjs 'export { default } from '\''./another-config.mjs'\'';'`


## `cd packages/a && vp lint --threads=1 -c .oxlintrc.json index.js`

The original symlinked consumer reports no-console

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

## `cd packages/b && vp lint --threads=1 -c .oxlintrc.json index.js`

The second alias uses the same rules

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

```
VITE+ - The Unified Toolchain for the Web

▲ No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
◆ ✔ Merged .oxlintrc.json into vite.config.mjs
● Formatting code...
● Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
• 1 config update applied
! Warnings:
  - Could not migrate packages/b/.oxlintrc.json: Error: Shared Oxlint configs require a Vite config with an ESM default export. The original config was preserved; use vp lint -c packages/b/.oxlintrc.json until it is migrated manually.
```

## `vpt stat-file packages/a/.oxlintrc.json --assert missing`

The successful alias is removed independently

```
packages/a/.oxlintrc.json: missing
```

## `vpt stat-file packages/b/.oxlintrc.json --assert symlink`

The failed consumer retains its own alias

```
packages/b/.oxlintrc.json: symlink
```

## `cd packages/a && vp lint --threads=1 index.js`

The migrated consumer keeps the same diagnostic

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

## `cd packages/b && vp lint --threads=1 -c .oxlintrc.json index.js`

The preserved consumer keeps a working JSON fallback

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

## `vpt replace-file-content packages/b/vite.config.mjs 'export { default } from "./another-config.mjs";' 'export default {};'`

Fix the unsupported export while retaining the workspace's pending marker

```
```

## `vp migrate --no-interactive --no-agent --no-editor --no-hooks`

```
VITE+ - The Unified Toolchain for the Web

▲ No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
● Formatting code...
● Code formatted
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite   → <version>
• 1 config update applied
```

## `vpt stat-file packages/b/.oxlintrc.json --assert missing`

The repaired alias is removed after its own migration completes

```
packages/b/.oxlintrc.json: missing
```

## `vpt stat-file shared/lint.json --assert file`

Unlinking both consumers preserves the shared source

```
shared/lint.json: file
```

## `cd packages/b && vp lint --threads=1 index.js`

The recovered consumer keeps the original diagnostic

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
