# migration_add_git_hooks

## `git init`


## `vp migrate --no-interactive`

migration should add git hooks setup

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
• Git hooks configured
```

## `vpt print-file package.json`

check package.json has prepare script and lint-staged config

```
{
  "name": "migration-add-git-hooks",
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  },
  "scripts": {
    "prepare": "vp config"
  }
}
```

## `vpt print-file pnpm-workspace.yaml`

check pnpm-workspace.yaml has overrides and catalog

```
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
overrides:
  vite: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
```

## `vpt print-file .vite-hooks/pre-commit`

check pre-commit hook

```
vp staged
```

## `vpt stat-file .vite-hooks/_ --assert dir`

hook shims exist (vp config ran)

```
.vite-hooks/_: dir
```

## `git config --local core.hooksPath`

should be set to .vite-hooks/_

```
.vite-hooks/_
```

## `vpt print-file .vite-hooks/_/.gitignore`

internal gitignore should exclude all files

```
*
```

## `vpt print-file .vite-hooks/_/h`

hook dispatcher script content

```
#!/usr/bin/env sh
{ [ "$HUSKY" = "2" ] || [ "$VITE_GIT_HOOKS" = "2" ]; } && set -x
n=$(basename "$0")
s=$(dirname "$(dirname "$0")")/$n

[ ! -f "$s" ] && exit 0

i="${XDG_CONFIG_HOME:-$HOME/.config}/vite-plus/hooks-init.sh"
[ ! -f "$i" ] && i="${XDG_CONFIG_HOME:-$HOME/.config}/husky/init.sh"
[ -f "$i" ] && . "$i"

{ [ "${HUSKY-}" = "0" ] || [ "${VITE_GIT_HOOKS-}" = "0" ]; } && exit 0

d="$(dirname "$(dirname "$(dirname "$0")")")"
__vp_shell=/bin/sh
[ -x "$__vp_shell" ] || __vp_shell=$(command -v sh)

if [ -n "${VP_HOME-}" ]; then
  __vp_bin="$VP_HOME/bin"
elif [ -n "${HOME-}" ]; then
  __vp_bin="$HOME/.vite-plus/bin"
else
  __vp_bin=""
fi
[ -n "$__vp_bin" ] && [ -d "$__vp_bin" ] && export PATH="$PATH:$__vp_bin"

export PATH="$d/node_modules/.bin:$PATH"
"$__vp_shell" -e "$s" "$@"
c=$?

[ $c != 0 ] && echo "VITE+ - $n script failed (code $c)"
[ $c = 127 ] && echo "VITE+ - command not found in PATH=$PATH"
exit $c
```

## `vpt print-file .vite-hooks/_/pre-commit`

hook shim should source the dispatcher

```
#!/usr/bin/env sh
. "$(dirname "$0")/h"
```

## `vpt list-dir .vite-hooks/_`

list all generated hook shims

```
applypatch-msg
commit-msg
h
post-applypatch
post-checkout
post-commit
post-merge
post-rewrite
pre-applypatch
pre-auto-gc
pre-commit
pre-merge-commit
pre-push
pre-rebase
prepare-commit-msg
```
