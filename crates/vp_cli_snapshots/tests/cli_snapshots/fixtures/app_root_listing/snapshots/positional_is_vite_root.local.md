# positional_is_vite_root

A positional Vite root is explicit command intent. Vite+ forwards it and does
not start target selection.

## `vp build apps/web`

```
note: You are running `vp build` as a Vite+ built-in command. If you meant to run the build npm script, use `vpr build` instead.
✓ 2 modules transformed.
computing gzip size...
apps/web/dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
