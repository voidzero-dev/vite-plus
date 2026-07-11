# positional_is_vite_root

A positional path is forwarded to Vite as [root] (upstream semantics), not
treated as a package to elicit: vp build <dir> at the workspace root skips
the picker/listing and builds that dir as the Vite root, with no Selected/Tip
elicitation lines (rfcs/cwd-flag.md).

## `vp build apps/web`

```
VITE+ - The Unified Toolchain for the Web

vite <version> building client environment for production...
✓ 2 modules transformed.
computing gzip size...
apps/web/dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
