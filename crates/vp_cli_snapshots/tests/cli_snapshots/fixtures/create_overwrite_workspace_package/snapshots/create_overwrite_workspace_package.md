# create_overwrite_workspace_package

Overwriting the only workspace package, even with a trailing separator, skips the dependency prompt instead of offering the new package as its own dependency.

## `vp create vite:generator --directory packages/old/ --interactive --no-agent --no-editor --no-hooks`

**→ expect-milestone:** `select:select:0`

```
VITE+ - The Unified Toolchain for the Web

› Target directory "<workspace>/packages/old" is not empty. Please choose how to proceed:
  › Cancel operation
    Remove existing files and continue
  ↑/↓ navigate · Enter confirm
```

**← write-key:** `down`

**→ expect-milestone:** `select:select:1`

```
VITE+ - The Unified Toolchain for the Web

› Target directory "<workspace>/packages/old" is not empty. Please choose how to proceed:
    Cancel operation
  › Remove existing files and continue
  ↑/↓ navigate · Enter confirm
```

**← write-key:** `enter`

**→ expect-milestone:** `text:text:`

```
VITE+ - The Unified Toolchain for the Web

◇ Target directory "<workspace>/packages/old" is not empty. Please choose how to proceed:
  Remove existing files and continue

› Description:
  Generate new components for our monorepo
```

**← write-key:** `enter`

```
VITE+ - The Unified Toolchain for the Web

◇ Target directory "<workspace>/packages/old" is not empty. Please choose how to proceed:
  Remove existing files and continue

◇ Description:
  Generate new components for our monorepo

◇ Scaffolded packages/old/ with generator scaffold
• Node <version>  pnpm <version>
→ Next: cd packages/old/ && vp run
```
