# system_pnpm_inherits_global_package_node

## `vp install -g --node 22.18.0 ./runtime-probe`


## `node setup-system-pnpm.cjs`


## `vp env off pnpm`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-pnpm${PATH_SEPARATOR}${PATH} npx --offline --call injected-pnpm-probe`

System-first pnpm inherits the global package's Node instead of reselecting the project's Node

```
The global package and system pnpm use Node <version>
```
