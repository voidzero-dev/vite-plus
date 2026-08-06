# command_unlink_pnpm10

## `vp unlink -h`

should show help message

```
VITE+ - The Unified Toolchain for the Web

Usage: vp unlink [OPTIONS] [PACKAGE|DIR] [ARGS]...

Unlink packages

Arguments:
  [PACKAGE|DIR]  Package name to unlink
  [ARGS]...      Arguments to pass to package manager

Options:
  -r, --recursive  Unlink in every workspace package
  -h, --help       Print help

Documentation: https://viteplus.dev/guide/install
```

## `vpt mkdir -p ../unlink-test-lib`

create test library

```
```

## `vpt write-file ../unlink-test-lib/package.json '{"name": "unlink-test-lib", "version": "1.0.0"}
'`

```
```

## `vp link ../unlink-test-lib`

link the library first

```

dependencies:
 unlink-test-lib 1.0.0 <- ../unlink-test-lib
```

## `vpt print-file package.json`

```
{
  "name": "command-unlink-pnpm10",
  "version": "1.0.0",
  "packageManager": "pnpm@10.19.0",
  "dependencies": {
    "unlink-test-lib": "link:../unlink-test-lib"
  }
}
```

## `vp unlink unlink-test-lib`

should unlink the package

```
Nothing to unlink
```

## `vpt print-file package.json`

```
{
  "name": "command-unlink-pnpm10",
  "version": "1.0.0",
  "packageManager": "pnpm@10.19.0",
  "dependencies": {
    "unlink-test-lib": "link:../unlink-test-lib"
  }
}
```

## `vp link ../unlink-test-lib`

link again

```
Lockfile is up to date, resolution step is skipped
```

## `vp unlink`

should unlink all packages

```
Nothing to unlink
```

## `vpt print-file package.json`

```
{
  "name": "command-unlink-pnpm10",
  "version": "1.0.0",
  "packageManager": "pnpm@10.19.0",
  "dependencies": {
    "unlink-test-lib": "link:../unlink-test-lib"
  }
}
```
