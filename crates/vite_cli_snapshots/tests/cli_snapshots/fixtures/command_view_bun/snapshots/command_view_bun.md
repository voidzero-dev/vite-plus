# command_view_bun

## `vp pm view --help`

should show help

```
VITE+ - The Unified Toolchain for the Web

Usage: vp pm view [OPTIONS] <PACKAGE> [FIELD] [-- <PASS_THROUGH_ARGS>...]

View package information from the registry

Arguments:
  <PACKAGE>               Package name with optional version
  [FIELD]                 Specific field to view
  [PASS_THROUGH_ARGS]...  Additional arguments

Options:
  --json      Output in JSON format
  -h, --help  Print help

Documentation: https://viteplus.dev/guide/install
```

## `vp pm view testnpm2`

should view package information

```
testnpm2@1.0.1 | ISC | deps: 0 | versions: 2

dist
 .tarball: https://registry.npmjs.org/testnpm2/-/testnpm2-1.0.1.tgz
 .shasum: 8c7b209a673c360e540ab2777242171fd30fdee9
 .integrity: sha512-F4AQ+KmzhbOSlt7ae+X2O8IJktFZAcN6OK169TT4ny7M3e4Vje7NITZTOU31AtEk9L/Z8lrCrqinl/eY6WPuEw==

dist-tags:
latest: 1.0.1
release-1: 1.0.1

maintainers:
- fengmk2 <fengmk2@gmail.com>

Published: 2015-07-18T18:23:59.560Z
```

## `vp pm view testnpm2 version`

should view version field

```
1.0.1
```
