# system_pnpm_preserves_independent_npm_pin

## `vpt json-edit package.json packageManager npm@10.5.0`


## `vp env on npm`


## `vp install -g --node 22.18.0 ./runtime-probe`


## `node setup-system-pnpm.cjs`


## `vp env off pnpm`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-pnpm${PATH_SEPARATOR}${PATH} npx --offline --call 'injected-pnpm-probe 10.5.0'`

Reusing Node for system pnpm must also retain independent npm precedence

```
The global package and system pnpm use Node <version>
```
