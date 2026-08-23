# build_in_place

A root-only workspace has no target ambiguity. Static extraction cannot see
the build input that the plugin supplies. Bare vp build must run in place in
a non-interactive terminal.

## `vp build`

```
transforming...
✓ 2 modules transformed.
rendering chunks...
computing gzip size...
dist/assets/index-<hash>.js  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
