# command_self_setup_external_homebrew_env_forbidden

## `vpt mkdir -p brew-prefix/bin home empty`


## `vpt cp $VP_HOME/bin/vp brew-prefix/bin/vp`


## `vpt write-file brew-prefix/INSTALL_RECEIPT.json '{"homebrew_version":"7.0.2","source":{"tap":"fengmk2/core"}}'`

A Homebrew formula without bundled dependencies installs them into the user package directory


## `vpt write-file home/config.json '{"nodeShimMode":"managed","packageManagerShimModes":{"bun":"managed","npm":"managed","pnpm":"managed","yarn":"managed"}}'`


## `VP_HOME=${workspace}/home VP_ENV_FORBIDDEN=1 PATH=${workspace}/empty ./brew-prefix/bin/vp --help`

Without a system node and pnpm, setup fails before it saves preferences or creates the package directory

**Exit code:** 1

```
error: Setup error: VP_ENV_FORBIDDEN=1: Vite+ installs its dependencies with the system 'node', but no 'node' is on PATH; install it with your system package manager
```

## `vpt print-file home/config.json`

```
{"nodeShimMode":"managed","packageManagerShimModes":{"bun":"managed","npm":"managed","pnpm":"managed","yarn":"managed"}}
```

## `vpt stat-file home/cli-packages --assert missing`

```
home/cli-packages: missing
```
