# nested_package_manager_respects_shim_mode

## `vpt chmod +x system-bin/pnpm`


## `vp env on pnpm`


## `pnpm --version`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} npx --offline --call 'pnpm --version'`

Managed mode keeps the project pin even when system pnpm is available

```
10.19.0
```

## `vp env off pnpm`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} npx --offline --call 'pnpm --version'`

System-first mode still selects system pnpm

```
system-pnpm
```
