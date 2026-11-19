# command_env_dev_engines_runtime_alias

devEngines.runtime.version accepts only semver, as in npm and pnpm. A Node.js alias follows the entry's onFail.

## `vp env current node --json`

the default onFail (error) fails instead of resolving the alias

**Exit code:** 1

```
error: Configuration error: devEngines.runtime.version "lts/*" in <workspace>/package.json is not a valid semver range (see devEngines spec). Use a semver version or range instead, or run `vp env pin lts` in that directory to pin the current LTS version
```

## `node --version`

the node shim reports the same error

**Exit code:** 1

```
vp: Failed to resolve Node version: Configuration error: devEngines.runtime.version "lts/*" in <workspace>/package.json is not a valid semver range (see devEngines spec). Use a semver version or range instead, or run `vp env pin lts` in that directory to pin the current LTS version
vp: Run 'vp env doctor' for diagnostics
```

## `vpt json-edit package.json devEngines.runtime.onFail warn`


## `vp env current node --json`

onFail warn skips the entry with a warning, so engines.node selects the version

```
warn: devEngines.runtime.version "lts/*" in <workspace>/package.json is not a valid semver range (see devEngines spec), ignoring
{
  "node": {
    "version": "22.23.4",
    "source": "engines.node",
    "source_path": "<workspace>/package.json",
    "project_root": "<workspace>",
    "bin_path": "<home>/.vite-plus/js_runtime/node/<version>/bin/node",
    "installed": false,
    "mode": "managed"
  }
}
```
