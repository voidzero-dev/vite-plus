# picker_selects_root_fallback

The workspace root is the last picker row when it has no root intent signal.
Selecting that row runs the command in the workspace root.

## `vp build`

**→ expect-milestone:** `package-select::0`

```
VITE+ - The Unified Toolchain for the Web

note: You are running `vp build` as a Vite+ built-in command. If you meant to run the build npm script, use `vpr build` instead.
Select a package to build (↑/↓, Enter to run, type to search):

  › admin            apps/admin
    web              apps/web
    ui               packages/ui
    app-root-listing .
```

**← write:** `app-root-listing`

**→ expect-milestone:** `package-select:app-root-listing:0`

```
VITE+ - The Unified Toolchain for the Web

note: You are running `vp build` as a Vite+ built-in command. If you meant to run the build npm script, use `vpr build` instead.
Select a package to build (↑/↓, Enter to run, type to search): app-root-listing

  › app-root-listing .
```

**← write-key:** `enter`

```
VITE+ - The Unified Toolchain for the Web

note: You are running `vp build` as a Vite+ built-in command. If you meant to run the build npm script, use `vpr build` instead.
Selected package: app-root-listing (.)
Tip: run this directly with `vp -C . build`
✓ 2 modules transformed.
computing gzip size...
dist/assets/root-entry-<hash>.js  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
