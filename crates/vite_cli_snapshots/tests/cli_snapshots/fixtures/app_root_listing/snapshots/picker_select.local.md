# picker_select

A bare app command at a workspace root with several candidates opens the
fuzzy package picker (the vp run selector component); typing filters, Enter
runs the selection as an implicit -C (rfcs/cwd-flag.md).

## `vp build`

**→ expect-milestone:** `package-select::0`

```
Select a package to build (↑/↓, Enter to run, type to search):

  › admin apps/admin
    web   apps/web
    ui    packages/ui
```

**← write:** `web`

**→ expect-milestone:** `package-select:web:0`

```
Select a package to build (↑/↓, Enter to run, type to search): web

  › web apps/web
```

**← write-key:** `enter`

```
Selected package: web (apps/web)
Tip: run this directly with `vp -C apps/web build`
vite <version> building client environment for production...
✓ 2 modules transformed.
computing gzip size...
dist/index.html  0.06 kB │ gzip: 0.06 kB

✓ built in <duration>
```
