# npm_uses_powershell_for_interactive_shim

## `node prepare.cjs npm`


## `npm --flag "value with spaces" path\file`

The npm.exe proxy shares the same managed launcher policy.

```
launcher=ps1
arguments, environment, cwd, and Node selection preserved
```

## `npm --flag "value with spaces" path\file`

Non-interactive npm calls retain the batch launcher.

```
launcher=cmd
arguments, environment, cwd, and Node selection preserved
```
