# command_update_yarn4

## `vp update testnpm2 --no-save`

reject unsupported --no-save without updating dependencies

**Exit code:** 1

```
yarn does not support --no-save.
```

## `vpt print-file package.json`

```
{
  "name": "command-update-yarn4",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "*"
  },
  "devDependencies": {
    "test-vite-plus-package": "*"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "*"
  },
  "packageManager": "yarn@4.10.3"
}
```

## `vp rm testnpm2`

should to absolute latest version

```
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0085: │ + test-vite-plus-package-optional@npm:1.0.0, test-vite-plus-package@npm:1.0.0
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0013: │ 2 packages were added to the project (+ <size> KiB).
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration>
```

## `vp add testnpm2@1.0.0 -D`

```
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0085: │ + testnpm2@npm:1.0.0
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0013: │ A package was added to the project (+ <size> KiB).
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration>
```

## `vp update testnpm2 --latest`

```
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0085: │ + testnpm2@npm:1.0.1
➤ YN0085: │ - testnpm2@npm:1.0.0
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
  "name": "command-update-yarn4",
  "version": "1.0.0",
  "devDependencies": {
    "test-vite-plus-package": "*",
    "testnpm2": "^1.0.1"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "*"
  },
  "packageManager": "yarn@4.10.3"
}
```

## `vp update -D`

reject unsupported dev-only updates

**Exit code:** 1

```
yarn does not support --dev.
```

## `vpt print-file package.json`

```
{
  "name": "command-update-yarn4",
  "version": "1.0.0",
  "devDependencies": {
    "test-vite-plus-package": "*",
    "testnpm2": "^1.0.1"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "*"
  },
  "packageManager": "yarn@4.10.3"
}
```

## `vp update --recursive`

should update all packages but won't change the package.json

```
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
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
  "name": "command-update-yarn4",
  "version": "1.0.0",
  "devDependencies": {
    "test-vite-plus-package": "*",
    "testnpm2": "^1.0.1"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "*"
  },
  "packageManager": "yarn@4.10.3"
}
```
