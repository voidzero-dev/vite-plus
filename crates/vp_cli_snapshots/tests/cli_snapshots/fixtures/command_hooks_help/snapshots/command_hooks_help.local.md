# command_hooks_help

## `vp hooks -h`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp hooks <COMMAND> [OPTIONS]

Manage the Vite+ Git hook dispatcher for this repository.

Options:
  -h, --help  Show this help message

Commands:
  enable   Install or refresh the hook dispatcher (sets core.hooksPath)
  disable  Disable hooks: unset core.hooksPath, remove <dir>/_, persist preference
  status   Show preference, core.hooksPath, and dispatcher state

Environment:
  VP_GIT_HOOKS=0  Skip dispatcher install in enable (and skip hooks at commit time)

Examples:
  vp hooks enable
  vp hooks enable --hooks-dir .custom-hooks
  vp hooks disable
  vp hooks status

Documentation: https://viteplus.dev/guide/commit-hooks
```

## `vp hooks --help`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp hooks <COMMAND> [OPTIONS]

Manage the Vite+ Git hook dispatcher for this repository.

Options:
  -h, --help  Show this help message

Commands:
  enable   Install or refresh the hook dispatcher (sets core.hooksPath)
  disable  Disable hooks: unset core.hooksPath, remove <dir>/_, persist preference
  status   Show preference, core.hooksPath, and dispatcher state

Environment:
  VP_GIT_HOOKS=0  Skip dispatcher install in enable (and skip hooks at commit time)

Examples:
  vp hooks enable
  vp hooks enable --hooks-dir .custom-hooks
  vp hooks disable
  vp hooks status

Documentation: https://viteplus.dev/guide/commit-hooks
```

## `vp hooks enable --help`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp hooks enable [OPTIONS]

Install or refresh the hook dispatcher (sets core.hooksPath)

Options:
  --hooks-dir <path>  Custom hooks directory (default: .vite-hooks, or last used)
  -h, --help          Show this help message

Documentation: https://viteplus.dev/guide/commit-hooks
```

## `vp help hooks`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp hooks <COMMAND> [OPTIONS]

Manage the Vite+ Git hook dispatcher for this repository.

Options:
  -h, --help  Show this help message

Commands:
  enable   Install or refresh the hook dispatcher (sets core.hooksPath)
  disable  Disable hooks: unset core.hooksPath, remove <dir>/_, persist preference
  status   Show preference, core.hooksPath, and dispatcher state

Environment:
  VP_GIT_HOOKS=0  Skip dispatcher install in enable (and skip hooks at commit time)

Examples:
  vp hooks enable
  vp hooks enable --hooks-dir .custom-hooks
  vp hooks disable
  vp hooks status

Documentation: https://viteplus.dev/guide/commit-hooks
```
