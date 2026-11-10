# env_pin_scope_help

## `vp env pin --help`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp env pin [OPTIONS] [SPECS]...

Pin Node.js locally and package-manager versions at the workspace root

Arguments:
  [SPECS]...  Versions to pin. Bare versions select Node.js; package managers use name@version

Options:
  --unpin                                                        Remove Node.js pins locally and package-manager pins at the workspace root
  --no-install                                                   Skip pre-downloading the pinned version
  --force                                                        Overwrite an existing pin without confirmation
  --target <TARGET>                                              Explicitly choose the write target (overrides the default selection)
                                                                 Possible values:
  - node-version:                                                Pin via the .node-version file
  - nvmrc:                                                       Pin via the .nvmrc file
  - dev-engines:                                                 Pin via package.json#devEngines (runtime or packageManager)
  - package-manager: Pin via the top-level packageManager field
  -h, --help                                                     Print help (see a summary with '-h')

Examples:
  vp env pin lts                  # Pin Node.js to latest LTS
  vp env pin pnpm@10              # Pin the package manager
  vp env pin 22 pnpm@10           # Pin both components
  vp env pin --unpin              # Remove both effective pins
  vp env pin "^20.0.0" --force    # Overwrite existing pin
  vp env pin 24 --target node-version   # Force the .node-version file
Node.js pins use the current directory. Package-manager pins use the workspace
root package.json, or the current project package.json outside a workspace.
Pinning both components can update two different files. --target selects a pin
source, not a project directory.
The Node.js write target follows the compatibility-first rule: an existing
.node-version keeps being updated, as does an effective .nvmrc in the current
directory.
Otherwise the pin is written to package.json#devEngines.runtime, or .node-version
when the directory has no package.json.

Documentation: https://viteplus.dev/guide/env
```

## `vp env unpin --help`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp env unpin [OPTIONS] [SCOPE]

Remove Node.js pins locally and package-manager pins at the workspace root

Arguments:
  [SCOPE]  Limit removal to node, pm, or a package-manager family

Options:
  --target <TARGET>                                                               Explicitly choose which pin source to remove
                                                                                  Possible values:
  - node-version:                                                                 Pin via the .node-version file
  - nvmrc:                                                                        Pin via the .nvmrc file
  - dev-engines:                                                                  Pin via package.json#devEngines (runtime or packageManager)
  - package-manager: Pin via the top-level packageManager field
  -h, --help                                                                      Print help (see a summary with '-h')
  Alias for pin --unpin. Node.js pins use the current directory. Package-manager
  pins use the workspace root package.json, or the current project package.json
  outside a workspace. Removing both components can update two different files.
  --target selects a pin source, not a project directory.

Documentation: https://viteplus.dev/guide/env
```
