# auto_select_root

A workspace root that is itself the only runnable app stays selectable: bare
vp build auto-selects the root (shown as `.`) and runs in place, matching
today's behavior for root apps (rfcs/cwd-flag.md, picker contents).

## `vp build`

```
Selected package: runnable-root (.)
Tip: run this directly with `vp -C . build`
vite <version> building client environment for production...
✓ 2 modules transformed.
computing gzip size...
dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
