# build_in_place

Regression guard: a single-package repo (no workspaces field) never goes
through target elicitation. Bare vp build runs in place even in an
interactive terminal: no picker, no Selected line, no listing
(rfcs/cwd-flag.md, resolution order step "anywhere else").

## `vp build`

```
VITE+ - The Unified Toolchain for the Web

vite <version> building client environment for production...
✓ 2 modules transformed.
computing gzip size...
dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
