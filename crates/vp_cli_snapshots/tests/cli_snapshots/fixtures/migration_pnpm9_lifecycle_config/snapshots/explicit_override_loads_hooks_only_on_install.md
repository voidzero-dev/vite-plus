# explicit_override_loads_hooks_only_on_install

## `vpt cp record-pnpm-hook.cjs .pnpmfile.cjs`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`


## `vpt print-file pnpm-hook-events`

The selected pnpm loads the project hook for both installs and skips it during version probes.

```
install
install
```
