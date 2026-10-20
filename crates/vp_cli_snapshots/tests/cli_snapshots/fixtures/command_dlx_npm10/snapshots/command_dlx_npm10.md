# command_dlx_npm10

## `vp dlx --help`

should show help message

```
VITE+ - The Unified Toolchain for the Web

Usage: vp dlx [OPTIONS] <ARGS>...

Download and execute a package without installing it globally

Arguments:
  <ARGS>...  Package to execute and arguments

Options:
  -p, --package <NAME>  Package(s) to install before running
  -c, --shell-mode      Execute within a shell environment
  -s, --silent          Suppress all output except the executed command's output
  -h, --help            Print help

Documentation: https://viteplus.dev/guide/vpx
```

## `vp dlx -s vp-test-echo hello`

should run vp-test-echo with npm exec

```
hello
```

## `vp dlx -s vp-test-echo@1.0.0 hello`

should run specific version

```
hello
```

## `vp dlx -s -p vp-test-echo -p vp-test-upper vp-test-echo hello`

should run with multiple packages

```
hello
```

## `vp dlx -s -p vp-test-echo -p vp-test-upper -c 'echo hello-from-shell | vp-test-echo | vp-test-upper'`

should run shell mode command (vp's own shell handles the pipe)

```
HELLO-FROM-SHELL
```
