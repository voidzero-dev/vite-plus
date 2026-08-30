# command_env_doctor_system_package_manager

## `vpt write-file package.json '{"name":"doctor-system-package-manager","private":true,"devEngines":{"packageManager":{"name":"pnpm","version":"^10.0.0"}}}
'`


## `vpt chmod +x system-bin/pnpm`


## `vp env off pnpm`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} NPM_CONFIG_REGISTRY=http://127.0.0.1:9 node assert-doctor-system-package-manager.cjs`

doctor checks a system-first package manager without registry access

```
doctor reports the system pnpm binary without resolving the declared range
current reports the system pnpm binary without resolving the declared range
print uses the system pnpm binary without resolving the declared range
```
