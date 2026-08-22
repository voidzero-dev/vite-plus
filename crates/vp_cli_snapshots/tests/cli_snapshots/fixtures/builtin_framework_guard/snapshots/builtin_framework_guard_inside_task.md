# builtin_framework_guard_inside_task

## `vp run nested-build`

no refusal: a task-spawned `vp build` runs the bundled Vite build as invoked

```
$ vp build ⊘ cache disabled
✓ 4 modules transformed.
computing gzip size...
dist/index.html                <size> kB │ gzip: <size> kB
dist/assets/index-<hash>.js  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
