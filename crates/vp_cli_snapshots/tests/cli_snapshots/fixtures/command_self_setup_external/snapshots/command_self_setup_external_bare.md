# command_self_setup_external_bare

## `vpt mkdir -p external home/external-test/bin`


## `vpt cp $VP_HOME/bin/vp external/vp`


## `vpt chmod 555 external/vp`


## `vpt cp external/vp home/external-test/bin/vp`


## `vpt chmod 555 home/external-test/bin/vp`

An interrupted bootstrap left a read-only binary before activation


## `VP_HOME=${workspace}/home VP_NODE_MANAGER=no ./external/vp --help`

Retry replaces the incomplete copy and finishes setup


## `vpt stat-file home/current/bin/.vp-setup-complete --assert file`

```
home/current/bin/.vp-setup-complete: file
```

## `vpt stat-file external/.vp-setup-complete --assert missing`

```
external/.vp-setup-complete: missing
```

## `VP_HOME=${workspace}/home ./external/vp env off`

The external binary reuses its completed installation

```
VITE+ - The Unified Toolchain for the Web

✓ Node.js and package-manager management set to system-first.

Selected commands and shims will now prefer system tools, falling back to managed tools.

Run `vp env on` to always use Vite+ managed tools.
```

## `VP_HOME=${workspace}/home VP_NODE_MANAGER=yes ./external/vp --help`


## `vpt stat-file home/.previous-version --assert missing`

```
home/.previous-version: missing
```

## `vpt print-file home/config.json`

```
{
  "nodeShimMode": "system_first",
  "packageManagerShimModes": {
    "bun": "system_first",
    "npm": "system_first",
    "pnpm": "system_first",
    "yarn": "system_first"
  }
}
```

## `VP_HOME=${workspace}/home VP_NODE_MANAGER=no VP_SELF_SETUP_SHELL=sh ./external/vp`

An explicit installer handoff still reinstalls the same version


## `vpt print-file home/.previous-version`

```
external-test
```

## `vpt stat-file home/current/bin/.vp-setup-complete --assert file`

```
home/current/bin/.vp-setup-complete: file
```
