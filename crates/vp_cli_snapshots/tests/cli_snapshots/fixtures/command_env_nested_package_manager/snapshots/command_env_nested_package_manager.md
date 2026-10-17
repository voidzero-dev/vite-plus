# command_env_nested_package_manager

## `node prepare-npm.cjs`


## `cd svc && vp env current pm --json`

a nested project outside the workspace packages uses its own packageManager

```
{
  "package_manager": {
    "name": "npm",
    "version": "<version>",
    "source": "packageManager",
    "source_path": "<workspace>/svc/package.json",
    "project_root": "<workspace>/svc",
    "bin_paths": {
      "npm": "<home>/.vite-plus/package_manager/npm/<version>/npm/bin/npm",
      "npx": "<home>/.vite-plus/package_manager/npm/<version>/npm/bin/npx"
    },
    "installed": true,
    "mode": "managed"
  }
}
```

## `vpt cp member-workspace.yaml pnpm-workspace.yaml`


## `cd svc && vp env current pm --json`

a workspace package uses the workspace's packageManager, even when it declares its own

```
{
  "package_manager": {
    "name": "pnpm",
    "version": "<version>",
    "source": "packageManager",
    "source_path": "<workspace>/package.json",
    "project_root": "<workspace>",
    "bin_paths": {
      "pnpm": "<home>/.vite-plus/package_manager/pnpm/<version>/pnpm/bin/pnpm",
      "pnpx": "<home>/.vite-plus/package_manager/pnpm/<version>/pnpm/bin/pnpx"
    },
    "installed": false,
    "mode": "managed"
  }
}
```
