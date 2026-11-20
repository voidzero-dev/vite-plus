# command_dlx_pnpm10

## `vp dlx --help`

should show help message

```
Download and execute a package without installing it globally

Usage: vp dlx [OPTIONS] <ARGS>...

Arguments:
  <ARGS>...  Package to execute and arguments

Options:
  -p, --package <NAME>  Package(s) to install before running
  -c, --shell-mode      Execute within a shell environment
  -s, --silent          Suppress all output except the executed command's output
  -h, --help            Print help
```

## `vp dlx -s vp-test-echo hello`

should run vp-test-echo with pnpm dlx

```
hello
```

## `vp dlx -s vp-test-echo@1.0.0 hello`

should run specific version

```
hello
```

## `vpt json-edit package.json packageManager pnpm@7.20.0`


## `vp dlx --shell-mode 'echo hello'`

Reject shell mode before pnpm 7.21.0 without executing the package manager.

**Exit code:** 1

```
pnpm < 7.21.0 does not support --shell-mode.
```
