# command_run_flags_before_command

## `vp --filter backend run dev`

without a terminal, the corrected order is printed instead of a prompt

**Exit code:** 2

```
error: Unexpected argument '--filter'

Did you mean `vp run --filter backend dev`?
```

## `CI=1 vp --filter backend run dev`

CI prints the corrected order without a prompt, also with a terminal

**Exit code:** 2

```
VITE+ - The Unified Toolchain for the Web

error: Unexpected argument '--filter'

Did you mean `vp run --filter backend dev`?
```

## `vp --filter backend run dev`

declining the corrected order keeps the error exit code

**Exit code:** 2

**→ expect-milestone:** `text:suggested-command:ready`

```
VITE+ - The Unified Toolchain for the Web

error: Unexpected argument '--filter'

Do you want to run `vp run --filter backend dev`? (y/N):
```

**← write-line:** `n`

```
VITE+ - The Unified Toolchain for the Web

error: Unexpected argument '--filter'

Do you want to run `vp run --filter backend dev`? (y/N): n
```

## `vp --filter backend run dev`

accepting runs `vp run --filter backend dev`

**→ expect-milestone:** `text:suggested-command:ready`

```
VITE+ - The Unified Toolchain for the Web

error: Unexpected argument '--filter'

Do you want to run `vp run --filter backend dev`? (y/N):
```

**← write-line:** `y`

```
VITE+ - The Unified Toolchain for the Web

error: Unexpected argument '--filter'

Do you want to run `vp run --filter backend dev`? (y/N): y
$ echo dev from backend ⊘ cache disabled
dev from backend
```
