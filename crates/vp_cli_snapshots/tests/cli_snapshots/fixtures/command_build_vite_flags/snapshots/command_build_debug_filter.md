# command_build_debug_filter

## `vp build --debug config --filter 'no config file'`

--debug enables the vite:config logs, and --filter keeps only the line containing 'no config file' (the resolved config dump is filtered out)

```
  vite:config no config file found. +<duration>
✓ 2 modules transformed.
computing gzip size...
dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
