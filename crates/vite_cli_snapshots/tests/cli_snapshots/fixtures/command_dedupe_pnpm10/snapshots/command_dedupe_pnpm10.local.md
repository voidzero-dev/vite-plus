# command_dedupe_pnpm10

## `vp dedupe --help`

should show help

```
Deduplicate dependencies

Usage: vp dedupe [OPTIONS] [-- <PASS_THROUGH_ARGS>...]

Arguments:
  [PASS_THROUGH_ARGS]...  Additional arguments to pass through to the package manager

Options:
      --check  Check if deduplication would make changes
  -h, --help   Print help
```

## `vp dedupe`

should dedupe dependencies

```
Already up to date

dependencies:
 testnpm2 1.0.1

optionalDependencies:
 test-vite-plus-package-optional 1.0.0

devDependencies:
 test-vite-plus-package 1.0.0
```

## `vpt print-file package.json`

```
{
  "name": "command-dedupe-pnpm10",
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
  "packageManager": "pnpm@10.18.0"
}
```

## `vp dedupe --check`

should check if deduplication would make changes

```
```

## `vpt print-file package.json`

```
{
  "name": "command-dedupe-pnpm10",
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
  "packageManager": "pnpm@10.18.0"
}
```

## `vp dedupe -- --loglevel=warn`

support pass through arguments

```
```

## `vpt print-file package.json`

```
{
  "name": "command-dedupe-pnpm10",
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
  "packageManager": "pnpm@10.18.0"
}
```

## `vpt json-edit package.json dependencies {}`

should check fails because no dependencies


## `vpt print-file package.json`

```
{
  "dependencies": {},
  "devDependencies": {
    "test-vite-plus-package": "1.0.0"
  },
  "name": "command-dedupe-pnpm10",
  "optionalDependencies": {
    "test-vite-plus-package-optional": "1.0.0"
  },
  "packageManager": "pnpm@10.18.0",
  "version": "1.0.0"
}
```

## `vp dedupe --check`

**Exit code:** 1

```
 ERR_PNPM_DEDUPE_CHECK_ISSUES  Dedupe --check found changes to the lockfile

Importers
.
└── - testnpm2 1.0.1

Packages
- testnpm2@1.0.1

Run pnpm dedupe to apply the changes above.
```

## `vp dedupe`

should dedupe fix the change by removing the dependencies

```
Packages: -1
-

dependencies:
- testnpm2 1.0.1
```

## `vpt print-file package.json`

```
{
  "dependencies": {},
  "devDependencies": {
    "test-vite-plus-package": "1.0.0"
  },
  "name": "command-dedupe-pnpm10",
  "optionalDependencies": {
    "test-vite-plus-package-optional": "1.0.0"
  },
  "packageManager": "pnpm@10.18.0",
  "version": "1.0.0"
}
```

## `vp dedupe --check`

```
```
