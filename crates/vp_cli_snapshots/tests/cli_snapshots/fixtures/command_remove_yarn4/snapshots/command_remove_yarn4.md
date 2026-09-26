# command_remove_yarn4

## `vp remove testnpm2 -D`

reject unsupported --save-dev even when the package is absent

**Exit code:** 1

```
yarn does not support --save-dev.
```

## `vpt print-file package.json`

```
{
  "name": "command-remove-yarn4",
  "version": "1.0.0",
  "packageManager": "yarn@4.10.3"
}
```

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
➤ YN0000: · Done in <duration>
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
➤ YN0000: · Done in <duration>
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
➤ YN0000: · Done in <duration>
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
➤ YN0000: · Done in <duration>
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

## `vp remove -O test-vite-plus-package-optional`

reject unsupported --save-optional without removing the optional dependency

**Exit code:** 1

```
yarn does not support --save-optional.
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

## `vp remove test-vite-plus-package-optional`

remove optional dependencies without a section selector

```
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0085: │ - test-vite-plus-package-optional@npm:1.0.0
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration>
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
