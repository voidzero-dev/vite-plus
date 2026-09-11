# vite_define_config_without_run_stays_static

A Vite config with no `run` block must stay on the static extraction path.
Runtime evaluation would resolve an intentionally unbuilt import and fail
before the package script can run.

## `vp run build`

```
$ vpt print package-build-ran ⊘ cache disabled
package-build-ran
```
