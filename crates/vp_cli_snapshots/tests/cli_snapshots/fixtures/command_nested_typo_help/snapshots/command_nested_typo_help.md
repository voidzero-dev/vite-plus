# command_nested_typo_help

## `vp pm apprev-build --help`

typo should not print pm parent help

**Exit code:** 2

```
VITE+ - The Unified Toolchain for the Web

error: Command 'apprev-build' not found

Did you mean `vp pm approve-builds`?
```

## `vp help pm apprev-build`

help alias should not print pm parent help for a typo

**Exit code:** 2

```
VITE+ - The Unified Toolchain for the Web

error: Command 'apprev-build' not found

Did you mean `vp pm approve-builds`?
```

## `vp pm --help apprev-build`

help flag before typo should not print pm parent help

**Exit code:** 2

```
VITE+ - The Unified Toolchain for the Web

error: Command 'apprev-build' not found

Did you mean `vp pm approve-builds`?
```
