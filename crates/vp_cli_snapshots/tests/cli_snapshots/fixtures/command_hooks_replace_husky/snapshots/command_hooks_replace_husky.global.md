# command_hooks_replace_husky

## `git init`


## `git config core.hooksPath .husky/_`


## `vpt write-file .husky/_/pre-commit '#'\!'/bin/sh
echo original-check-rejected >&2
exit 1
'`


## `vpt write-file .vite-hooks/commit-msg 'echo migrated-check-rejected >&2
exit 1
'`


## `vp config --no-agent`

preserve non-empty Husky directory and explain recovery

```
core.hooksPath is already set to ".husky/_", skipping. Vite+ hooks at .vite-hooks are not active. Migrate any custom Husky checks, then run `vp hooks enable --replace-husky` (with the same --hooks-dir if customized). Switching stops running hooks from ".husky/_"; the old files are preserved.
```

## `vp hooks enable`

ordinary enable also preserves old hooks

```
core.hooksPath is already set to ".husky/_", skipping. Vite+ hooks at .vite-hooks are not active. Migrate any custom Husky checks, then run `vp hooks enable --replace-husky` (with the same --hooks-dir if customized). Switching stops running hooks from ".husky/_"; the old files are preserved.
```

## `git config --local core.hooksPath`

Husky remains active until explicit replacement

```
.husky/_
```

## `VP_GIT_HOOKS=0 vp hooks enable --replace-husky`

explicit replacement honors environment opt-out

```
skip install (git hooks disabled)
```

## `git config --local core.hooksPath`

opt-out left the path unchanged

```
.husky/_
```

## `vp hooks enable --replace-husky`

explicitly activate the migrated hooks, explaining the old hooks no longer run

```
Git hook dispatcher installed at .vite-hooks/_. Replaced core.hooksPath ".husky/_". Old Husky files were preserved, but hooks from ".husky/_" no longer run. Custom checks must be migrated to .vite-hooks.
```

## `git config --local core.hooksPath`

Vite+ dispatcher is now active

```
.vite-hooks/_
```

## `vpt print-file .husky/_/pre-commit`

old Husky script was preserved

```
#!/bin/sh
echo original-check-rejected >&2
exit 1
```

## `vpt print-file package.json`

prepare does not acquire the explicit replacement flag

```
{
  "name": "hooks-replace-husky",
  "private": true,
  "scripts": {
    "prepare": "vp config"
  }
}
```

## `git -c user.name=Test -c user.email=test@example.invalid -c commit.gpgsign=false commit --allow-empty -m test`

the migrated check rejects the commit

**Exit code:** 1

```
migrated-check-rejected
VITE+ - commit-msg script failed (code 1)
```

## `vp hooks disable --replace-husky`

the replacement option is enable-only

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: unexpected argument '--replace-husky' found

Usage: vp hooks disable [OPTIONS]

For more information, try '--help'.
```
