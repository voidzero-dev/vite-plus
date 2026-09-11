# command_remove_yarn4

## `vp remove testnpm2 -D`

should error when remove not exists package

**Exit code:** 1

```
Usage Error: Pattern testnpm2 doesn't match any packages referenced by this workspace

$ yarn remove [-A,--all] [--mode #0] ...
```

*(skipped 1 step(s) to the next line boundary: step failed)*

## `vp add testnpm2`

should add packages to dependencies

```
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0085: │ + testnpm2@npm:1.0.1
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0013: │ A package was added to the project (+ <size> KiB).
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration> <duration>
```

## `vp add -D test-vite-plus-install`

```
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0085: │ + test-vite-plus-install@npm:1.0.0
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0013: │ A package was added to the project (+ <size> KiB).
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration> <duration>
```

## `vp add -O test-vite-plus-package-optional`

```
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0085: │ + test-vite-plus-package-optional@npm:1.0.0
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0013: │ A package was added to the project (+ <size> KiB).
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration> <duration>
```

## `vpt print-file package.json`

```
{
  "name": "command-remove-yarn4",
  "version": "1.0.0",
  "packageManager": "yarn@4.10.3",
  "dependencies": {
    "testnpm2": "^1.0.1"
  },
  "devDependencies": {
    "test-vite-plus-install": "^1.0.0"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "^1.0.0"
  }
}
```

## `vp remove testnpm2 test-vite-plus-install`

should remove packages from dependencies

```
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0085: │ - test-vite-plus-install@npm:1.0.0, testnpm2@npm:1.0.1
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration> <duration>
```

## `vpt print-file package.json`

```
{
  "name": "command-remove-yarn4",
  "version": "1.0.0",
  "packageManager": "yarn@4.10.3",
  "optionalDependencies": {
    "test-vite-plus-package-optional": "^1.0.0"
  }
}
```

## `vp remove -D test-vite-plus-package-optional`

support ignore -O flag and remove package from optional dependencies

```
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0085: │ - test-vite-plus-package-optional@npm:1.0.0
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration> <duration>
```

## `vpt print-file package.json`

```
{
  "name": "command-remove-yarn4",
  "version": "1.0.0",
  "packageManager": "yarn@4.10.3"
}
```

## `vp remove -g --dry-run testnpm2`

support remove global package with dry-run

**Exit code:** 1

```
Failed to uninstall testnpm2: Package testnpm2 is not installed
```

*(skipped 1 step(s) to the next line boundary: step failed)*
