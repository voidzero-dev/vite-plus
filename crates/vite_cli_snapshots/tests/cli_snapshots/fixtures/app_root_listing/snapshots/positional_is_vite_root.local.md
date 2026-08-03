# positional_is_vite_root

A positional path is forwarded to Vite as [root] (upstream semantics), not
treated as a package to elicit: vp build <dir> at the workspace root skips
the picker/listing and builds that dir as the Vite root, with no Selected/Tip
elicitation lines (rfcs/cwd-flag.md).

## `vp build apps/web`

```
note: You are running `vp build` as a Vite+ built-in command. If you meant to run the build npm script, use `vpr build` instead.
note: `vp build apps/web` sets Vite's root without changing the working directory. To run as if started there, use `vp -C apps/web build`.
✓ 2 modules transformed.
computing gzip size...
apps/web/dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
