# picker_cancel

Ctrl+C in the package picker cancels with exit 130 and runs nothing.

## `vp build`

**Exit code:** 130

**→ expect-milestone:** `package-select::0`

```
Select a package to build (↑/↓, Enter to run, type to search):

  › admin apps/admin
    web   apps/web
    ui    packages/ui
```

**← write-key:** `ctrl-c`

```
```
