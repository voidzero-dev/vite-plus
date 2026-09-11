# command_dedupe_yarn4

## `vp dedupe`

should dedupe dependencies

```
➤ YN0000: ┌ Deduplication step
➤ YN0000: │ No packages can be deduped using the highest strategy
➤ YN0000: └ Completed
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0085: │ + test-vite-plus-package-optional@npm:1.0.0, test-vite-plus-package@npm:1.0.0, testnpm2@npm:1.0.1
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0013: │ 3 packages were added to the project (+ <size> KiB).
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration> <duration>
```

## `vpt print-file package.json`

```
{
  "name": "command-dedupe-yarn4",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "1.0.1"
  },
  "devDependencies": {
    "test-vite-plus-package": "1.0.0"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "1.0.0"
  },
  "packageManager": "yarn@4.10.3"
}
```

## `vp dedupe --check`

should check if deduplication would make changes

```
➤ YN0000: ┌ Deduplication step
➤ YN0000: │ No packages can be deduped using the highest strategy
➤ YN0000: └ Completed
```

## `vpt print-file package.json`

```
{
  "name": "command-dedupe-yarn4",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "1.0.1"
  },
  "devDependencies": {
    "test-vite-plus-package": "1.0.0"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "1.0.0"
  },
  "packageManager": "yarn@4.10.3"
}
```

## `vp dedupe -- --json`

support pass through arguments

```
```

## `vpt print-file package.json`

```
{
  "name": "command-dedupe-yarn4",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "1.0.1"
  },
  "devDependencies": {
    "test-vite-plus-package": "1.0.0"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "1.0.0"
  },
  "packageManager": "yarn@4.10.3"
}
```
