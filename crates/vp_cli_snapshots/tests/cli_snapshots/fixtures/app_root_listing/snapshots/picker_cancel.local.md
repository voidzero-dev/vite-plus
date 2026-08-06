# picker_cancel

Ctrl+C in the package picker cancels with exit 130 and runs nothing.

## `vp build`

**Exit code:** 130

**→ expect-milestone:** `package-select::0`

```
note: You are running `vp build` as a Vite+ built-in command. If you meant to run the build npm script, use `vpr build` instead.
Select a package to build (↑/↓, Enter to run, type to search):

  › admin apps/admin
    web   apps/web
    ui    packages/ui
```

**← write-key:** `ctrl-c`

```
note: You are running `vp build` as a Vite+ built-in command. If you meant to run the build npm script, use `vpr build` instead.
```
