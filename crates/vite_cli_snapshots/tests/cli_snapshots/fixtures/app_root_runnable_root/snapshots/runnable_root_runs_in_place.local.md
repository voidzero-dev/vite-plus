# runnable_root_runs_in_place

A workspace root that is itself a runnable app never elicits: bare vp build
runs in place (pre-elicitation behavior), without picker output.

## `vp build`

```
vite <version> building client environment for production...
✓ 2 modules transformed.
computing gzip size...
dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
