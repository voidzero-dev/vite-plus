# picker_select

A bare app command at a workspace root with several candidates opens the
fuzzy package picker (the vp run selector component); typing filters, Enter
runs the selection as an implicit -C (rfcs/cwd-flag.md).

## `vp build`

**→ expect-milestone:** `package-select::0`

```
note: You are running `vp build` as a Vite+ built-in command. If you meant to run the build npm script, use `vpr build` instead.
Select a package to build (↑/↓, Enter to run, type to search):

  › admin apps/admin
    web   apps/web
    ui    packages/ui
```

**← write:** `web`

**→ expect-milestone:** `package-select:web:0`

```
note: You are running `vp build` as a Vite+ built-in command. If you meant to run the build npm script, use `vpr build` instead.
Select a package to build (↑/↓, Enter to run, type to search): web

  › web apps/web
```

**← write-key:** `enter`

```
note: You are running `vp build` as a Vite+ built-in command. If you meant to run the build npm script, use `vpr build` instead.
Selected package: web (apps/web)
Tip: run this directly with `vp -C apps/web build`
✓ 2 modules transformed.
computing gzip size...
dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
