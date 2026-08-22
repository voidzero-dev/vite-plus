# builtin_framework_guard_default_package_to_nuxt

## `cd to_nuxt && vp build`

defaultPackage points at a Nuxt app, so the refusal fires for the resolved target and the hint carries -C

**Exit code:** 1

```
note: vp build: using ./app (defaultPackage in vite.config.ts)
error: this project uses Nuxt (nuxt.config.ts). `vp build` runs the bundled Vite CLI, not the Nuxt CLI.
hint: run the Nuxt CLI with `vp -C app exec nuxt build`.
```
