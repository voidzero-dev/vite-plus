# command_prune_pnpm11

## `vp install`

should install packages first

```
VITE+ - The Unified Toolchain for the Web

dependencies:
 testnpm2 1.0.1

optionalDependencies:
 test-vite-plus-package-optional 1.0.0

devDependencies:
 test-vite-plus-package 1.0.0

Done in <duration> using pnpm <version>
```

## `vp pm prune --help`

should show help

```
VITE+ - The Unified Toolchain for the Web

Usage: vp pm prune [OPTIONS] [-- <PASS_THROUGH_ARGS>...]

Remove unnecessary packages

Arguments:
  [PASS_THROUGH_ARGS]...  Additional arguments

Options:
  --prod         Remove devDependencies
  --no-optional  Remove optional dependencies
  -h, --help     Print help

Documentation: https://viteplus.dev/guide/install
```

## `vp pm prune`

should prune extraneous dependencies

```
Already up to date
```

## `vpt print-file package.json`

```
{
  "name": "command-prune-pnpm11",
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
  "packageManager": "pnpm@11.0.6"
}
```

## `vp pm prune --prod`

should prune dev dependencies

```
Lockfile is up to date, resolution step is skipped
Packages: -1
-

devDependencies:
- test-vite-plus-package 1.0.0
```

## `vpt print-file package.json`

```
{
  "name": "command-prune-pnpm11",
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
  "packageManager": "pnpm@11.0.6"
}
```

## `vp pm prune --no-optional`

should prune optional dependencies

```
Lockfile is up to date, resolution step is skipped
Packages: -1
-

optionalDependencies:
- test-vite-plus-package-optional 1.0.0

devDependencies:
 test-vite-plus-package 1.0.0
```

## `vpt print-file package.json`

```
{
  "name": "command-prune-pnpm11",
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
  "packageManager": "pnpm@11.0.6"
}
```

## `vp pm prune --prod --no-optional`

should prune both dev and optional dependencies

```
Lockfile is up to date, resolution step is skipped
Packages: -1
-

optionalDependencies: skipped

devDependencies:
- test-vite-plus-package 1.0.0
```

## `vpt print-file package.json`

```
{
  "name": "command-prune-pnpm11",
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
  "packageManager": "pnpm@11.0.6"
}
```

## `vp pm prune -- --loglevel=warn`

should support pass through arguments

```
```

## `vpt print-file package.json`

```
{
  "name": "command-prune-pnpm11",
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
  "packageManager": "pnpm@11.0.6"
}
```
