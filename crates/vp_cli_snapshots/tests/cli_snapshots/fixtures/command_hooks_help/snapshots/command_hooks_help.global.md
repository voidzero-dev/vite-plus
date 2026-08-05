# command_hooks_help

## `vp hooks -h`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp hooks <COMMAND> [OPTIONS]

Manage the Vite+ Git hook dispatcher for this repository.

Commands:
  setup    Install or refresh the hook dispatcher (sets core.hooksPath)
  disable  Disable hooks: unset core.hooksPath, remove <dir>/_, persist preference
  enable   Re-enable hooks after disable (same as setup)
  status   Show preference, core.hooksPath, and dispatcher state

Options:
  --hooks-dir <path>  Custom hooks directory (default: .vite-hooks, or last used)
  -h, --help          Show this help message

Environment:
  VP_GIT_HOOKS=0  Skip dispatcher install in setup/enable (and skip hooks at commit time)

Examples:
  vp hooks setup
  vp hooks setup --hooks-dir .custom-hooks
  vp hooks disable
  vp hooks enable
  vp hooks status

Documentation: https://viteplus.dev/guide/commit-hooks
```

## `vp hooks --help`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp hooks <COMMAND> [OPTIONS]

Manage the Vite+ Git hook dispatcher for this repository.

Commands:
  setup    Install or refresh the hook dispatcher (sets core.hooksPath)
  disable  Disable hooks: unset core.hooksPath, remove <dir>/_, persist preference
  enable   Re-enable hooks after disable (same as setup)
  status   Show preference, core.hooksPath, and dispatcher state

Options:
  --hooks-dir <path>  Custom hooks directory (default: .vite-hooks, or last used)
  -h, --help          Show this help message

Environment:
  VP_GIT_HOOKS=0  Skip dispatcher install in setup/enable (and skip hooks at commit time)

Examples:
  vp hooks setup
  vp hooks setup --hooks-dir .custom-hooks
  vp hooks disable
  vp hooks enable
  vp hooks status

Documentation: https://viteplus.dev/guide/commit-hooks
```

## `vp help hooks`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp hooks <COMMAND> [OPTIONS]

Manage the Vite+ Git hook dispatcher for this repository.

Commands:
  setup    Install or refresh the hook dispatcher (sets core.hooksPath)
  disable  Disable hooks: unset core.hooksPath, remove <dir>/_, persist preference
  enable   Re-enable hooks after disable (same as setup)
  status   Show preference, core.hooksPath, and dispatcher state

Options:
  --hooks-dir <path>  Custom hooks directory (default: .vite-hooks, or last used)
  -h, --help          Show this help message

Environment:
  VP_GIT_HOOKS=0  Skip dispatcher install in setup/enable (and skip hooks at commit time)

Examples:
  vp hooks setup
  vp hooks setup --hooks-dir .custom-hooks
  vp hooks disable
  vp hooks enable
  vp hooks status

Documentation: https://viteplus.dev/guide/commit-hooks
```
