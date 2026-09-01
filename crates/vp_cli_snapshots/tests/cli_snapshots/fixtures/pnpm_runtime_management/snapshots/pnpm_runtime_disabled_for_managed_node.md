# pnpm_runtime_disabled_for_managed_node

Vite+ owns Node.js runtime management independently from the selected pnpm binary.

## `vpt write-file package.json '{"name":"pnpm-runtime-management","private":true,"packageManager":"pnpm@11.1.0","devEngines":{"runtime":{"name":"node","version":"20.18.0","onFail":"download"}}}
'`


## `vpt write-file $VP_HOME/js_runtime/node/20.18.0/bin/node '#'\!'/bin/sh
'`


## `vpt chmod +x $VP_HOME/js_runtime/node/20.18.0/bin/node`


## `vpt chmod +x system-bin/pnpm`


## `vp env on node`


## `vp env off pnpm`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} PNPM_CONFIG_RUNTIME=from-user pnpm install`

direct system-first pnpm disables its duplicate Node.js runtime

```
PNPM_CONFIG_RUNTIME=false
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} PNPM_CONFIG_RUNTIME=from-user vp install`

pnpm-backed vp install applies the same system-first policy

```
VITE+ - The Unified Toolchain for the Web

PNPM_CONFIG_RUNTIME=false
```

## `vpt write-file $VP_HOME/package_manager/pnpm/11.1.0/pnpm/bin/pnpm '#'\!'/bin/sh
if [ "$1" = "--version" ]; then printf '\''11.1.0\n'\''; else printf '\''PNPM_CONFIG_RUNTIME=%s\n'\'' "${PNPM_CONFIG_RUNTIME-unset}"; fi
'`


## `vpt write-file $VP_HOME/package_manager/pnpm/11.1.0/pnpm/bin/pnpx '#'\!'/bin/sh
printf '\''PNPM_CONFIG_RUNTIME=%s\n'\'' "${PNPM_CONFIG_RUNTIME-unset}"
'`


## `vpt chmod +x $VP_HOME/package_manager/pnpm/11.1.0/pnpm/bin/pnpm`


## `vpt chmod +x $VP_HOME/package_manager/pnpm/11.1.0/pnpm/bin/pnpx`


## `vpt mkdir other`


## `vpt write-file other/package.json '{"name":"other","private":true,"devEngines":{"runtime":{"name":"deno","version":"2.0.0","onFail":"download"}}}
'`


## `vp env on pnpm`


## `PNPM_CONFIG_RUNTIME=from-user pnpm install`

direct managed pnpm disables its duplicate Node.js runtime

```
PNPM_CONFIG_RUNTIME=false
```

## `PNPM_CONFIG_RUNTIME=from-user pnpx package`

the pnpx alias receives the same child-only override

```
PNPM_CONFIG_RUNTIME=false
```

## `PNPM_CONFIG_RUNTIME=from-user pnpx package --dir other`

pnpx forwards --dir to the downloaded command without changing runtime ownership

```
PNPM_CONFIG_RUNTIME=false
```

## `PNPM_CONFIG_RUNTIME=from-user vp install`

pnpm-backed vp install applies the same managed-pnpm policy

```
VITE+ - The Unified Toolchain for the Web

PNPM_CONFIG_RUNTIME=false
```
