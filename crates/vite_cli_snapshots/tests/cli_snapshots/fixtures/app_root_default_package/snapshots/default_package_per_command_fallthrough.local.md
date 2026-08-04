# default_package_per_command_fallthrough

A command absent from the defaultPackage object falls through to the
normal resolution: the map only declares `pack`, so bare `vp build` at
this workspace root runs in place at the (runnable) root with no note.

## `cd per_command_fallthrough && vp build`

```
✓ 2 modules transformed.
computing gzip size...
dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
