# foreign_manager_falls_back_without_recursion

## `node setup-foreign-manager.cjs`


## `vp env off node`

```
VITE+ - The Unified Toolchain for the Web

✓ Node.js management set to system-first.

Selected commands and shims will now prefer system tools, falling back to managed tools.

Run `vp env on` to always use Vite+ managed tools.
```

## `PATH=${workspace}/foreign-bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin${PATH_SEPARATOR}${PATH} node --version`

The external manager forwards to the managed fallback without re-entering itself.

```
<version>
```

## `PATH=${workspace}/foreign-bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin${PATH_SEPARATOR}${PATH} VP_PATH_INJECTED_TOOLS=node node --version`

An inherited injection marker must not cause recursion either.

```
<version>
```

## `PATH=${workspace}/foreign-bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin${PATH_SEPARATOR}${PATH} vp env exec node --version`

env exec still runs managed Node when the inherited PATH contains the external manager.

```
<version>
```
