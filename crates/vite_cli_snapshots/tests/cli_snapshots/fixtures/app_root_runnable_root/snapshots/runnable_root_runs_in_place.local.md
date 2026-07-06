# runnable_root_runs_in_place

A workspace root that is itself a runnable app never elicits: bare vp build
runs in place (pre-elicitation behavior), without picker output. The
non-TTY shape of the same rule is pinned by settings_only_workspace, whose
tsdown output is byte-stable on Windows (a piped vite build is not).

## `vp build`

```
vite <version> building client environment for production...
✓ 2 modules transformed.
computing gzip size...
dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
