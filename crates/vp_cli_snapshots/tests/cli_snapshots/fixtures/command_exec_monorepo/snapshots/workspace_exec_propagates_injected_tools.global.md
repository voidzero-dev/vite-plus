# workspace_exec_propagates_injected_tools

## `vpt json-edit package.json packageManager pnpm@10.19.0`


## `vp exec --filter app-* -- node ../../assert-injected-pnpm.cjs`

Sequential workspace execution carries the package-manager PATH and its tool set

```
VITE+ - The Unified Toolchain for the Web

app-a$ node ../../assert-injected-pnpm.cjs
Workspace exec preserves injected pnpm
app-b$ node ../../assert-injected-pnpm.cjs
Workspace exec preserves injected pnpm
```

## `vp exec --filter app-* --parallel -- node ../../assert-injected-pnpm.cjs`

Parallel workspace execution carries the same environment

```
VITE+ - The Unified Toolchain for the Web

app-a$ node ../../assert-injected-pnpm.cjs
Workspace exec preserves injected pnpm
app-b$ node ../../assert-injected-pnpm.cjs
Workspace exec preserves injected pnpm
```
