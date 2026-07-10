# command_run_without_vite_plus

## `vp run hello`

should execute via global vite-plus task runner

```
VITE+ - The Unified Toolchain for the Web

$ echo hello from script ⊘ cache disabled
hello from script
```

## `vp run greet --arg1 value1`

should pass through args

```
VITE+ - The Unified Toolchain for the Web

$ echo greet --arg1 value1 ⊘ cache disabled
greet --arg1 value1
```

## `vp run nonexistent`

should show task not found error

**Exit code:** 1

```
Task "nonexistent" not found.
```
