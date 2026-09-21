# prefers_existing_family_and_records_choice

## `vpt rm -f $VP_HOME/config.json`


## `vpt chmod +x system-bin/pnpm`


## `vpt chmod +x system-bin/yarn`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} pnpm --version`


## `vpt stat-file $VP_HOME/bin/pnpm --assert symlink`


## `vpt stat-file $VP_HOME/fallback-bin/pnpm --assert missing`


## `vpt print-file $VP_HOME/config.json`

the explicit system choice records only pnpm

```
{
  "packageManagerShimModes": {
    "pnpm": "system_first"
  }
}
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} pnpm --version`

later pnpm invocations use the recorded choice without prompting

```
system-pnpm
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} yarn --version`


## `vpt print-file $VP_HOME/config.json`

Yarn records its own decision without changing pnpm

```
{
  "packageManagerShimModes": {
    "pnpm": "system_first",
    "yarn": "system_first"
  }
}
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} pnpm --version`

The retained shim also honors the saved choice without a terminal.

```
system-pnpm
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} vp env which pnpm`

Internal resolution agrees with the retained shim.

```
VITE+ - The Unified Toolchain for the Web

<workspace>/system-bin/pnpm
```

## `vpt cp system-dispatch-bin/pnpm system-bin/pnpm`


## `vpt chmod +x system-bin/pnpm`


## `vpt write-file package.json '{"name":"legacy-system-choice","private":true,"packageManager":"pnpm@10.18.0"}
'`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} NPM_CONFIG_REGISTRY=http://127.0.0.1:9 vp install`

Package-manager commands use the saved system choice before shim relocation.

```
VITE+ - The Unified Toolchain for the Web

<version>
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} NPM_CONFIG_REGISTRY=http://127.0.0.1:9 vp env exec --node 20.18.0 pnpm --version`

Explicit Node execution also uses the system package manager.

```
10.18.0
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} vp env print pnpm`

Shell activation selects the same system directory.

```
VITE+ - The Unified Toolchain for the Web

# Add to your shell to use this environment for this session:
export PATH="<workspace>/system-bin:$PATH"
```

## `vp env setup --refresh`


## `vpt stat-file $VP_HOME/bin/pnpm --assert missing`


## `vpt stat-file $VP_HOME/fallback-bin/pnpm --assert symlink`

Explicit setup reconciles the saved choice into fallback-bin.

```
<home>/.vite-plus/fallback-bin/pnpm: symlink
```

## `vpt write-file $VP_HOME/package_manager/pnpm/10.18.0/pnpm/bin/pnpm '#'\!'/bin/sh
printf '\''managed-pnpm\n'\''
'`


## `vpt chmod +x $VP_HOME/package_manager/pnpm/10.18.0/pnpm/bin/pnpm`


## `PATH=${VP_HOME}/fallback-bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} pnpm --version`

A reconciled fallback shim before the system manager still selects managed execution.

```
managed-pnpm
```
