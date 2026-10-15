# command_build_debug_filter

## `vp build --debug config --filter 'loaded in'`

--debug enables the vite:config logs, and --filter keeps only the line containing 'loaded in' (the resolved config dump is filtered out)

```
  vite:config config file loaded in <duration> +<duration>
✓ 2 modules transformed.
computing gzip size...
dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
