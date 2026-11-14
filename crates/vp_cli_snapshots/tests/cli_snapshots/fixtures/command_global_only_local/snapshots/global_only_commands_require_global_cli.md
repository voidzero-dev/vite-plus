# global_only_commands_require_global_cli

## `vp env current`

environment management requires the global vp binary

**Exit code:** 2

```
error: The `env` command is only available in the global `vp` CLI.

Install it with:
  curl -fsSL https://vite.plus | bash

Then open a new terminal and run the command again.
If already installed, run `vp env` directly in your terminal instead of through a package manager or package.json script.

Installation guide: https://viteplus.dev/guide/
```

## `vp upgrade --check`

self-upgrade requires the global vp binary

**Exit code:** 2

```
error: The `upgrade` command is only available in the global `vp` CLI.

Install it with:
  curl -fsSL https://vite.plus | bash

Then open a new terminal and run the command again.
If already installed, run `vp upgrade` directly in your terminal instead of through a package manager or package.json script.

Installation guide: https://viteplus.dev/guide/
```

## `vp implode --yes`

self-removal requires the global vp binary

**Exit code:** 2

```
error: The `implode` command is only available in the global `vp` CLI.

Install it with:
  curl -fsSL https://vite.plus | bash

Then open a new terminal and run the command again.
If already installed, run `vp implode` directly in your terminal instead of through a package manager or package.json script.

Installation guide: https://viteplus.dev/guide/
```
