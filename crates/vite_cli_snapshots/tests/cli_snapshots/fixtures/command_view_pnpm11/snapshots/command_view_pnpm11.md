# command_view_pnpm11

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

should view lodash package information (uses npm view)

```
testnpm2@1.0.1 | ISC | deps: none | versions: 2

dist
.tarball: https://registry.npmjs.org/testnpm2/-/testnpm2-1.0.1.tgz
.shasum: 8c7b209a673c360e540ab2777242171fd30fdee9
.integrity: sha512-F4AQ+KmzhbOSlt7ae+X2O8IJktFZAcN6OK169TT4ny7M3e4Vje7NITZTOU31AtEk9L/Z8lrCrqinl/eY6WPuEw==

maintainers:
- fengmk2

dist-tags:
latest: 1.0.1
release-1: 1.0.1
```

## `vp pm view testnpm2 version`

should view lodash version field (uses npm view)

```
1.0.1
```

## `vp pm view testnpm2@1.0.0`

should view specific version of lodash (uses npm view)

```
testnpm2@1.0.0 | ISC | deps: none | versions: 2

dist
.tarball: https://registry.npmjs.org/testnpm2/-/testnpm2-1.0.0.tgz
.shasum: 3f9430987a1d4ff52c8c5162e99b5d8596efefa6
.integrity: sha512-8gdtqxKad+83Iog2v514VsHsSk/R+we9j5/9zX9tB+QC2ubvB06zJ08k0PSl5uzviXByEMiWm7EzSbBAh2GZ/w==

maintainers:
- fengmk2

dist-tags:
latest: 1.0.1
release-1: 1.0.1
```

## `vp pm view testnpm2 dist.tarball`

should view nested field (uses npm view)

```
https://registry.npmjs.org/testnpm2/-/testnpm2-1.0.1.tgz
```

## `vp pm view testnpm2 dependencies`

should view dependencies object (uses npm view)

```
```

## `vp pm view testnpm2 dist.tarball --json`

should view package.dist.tarball info in JSON format (uses npm view)

```
"https://registry.npmjs.org/testnpm2/-/testnpm2-1.0.1.tgz"
```

## `vp pm view testnpm2 version --json`

should view field in JSON format (uses npm view)

```
"1.0.1"
```

## `vp pm view testnpm2 -- --loglevel=warn`

should support pass through arguments (uses npm view)

```
testnpm2@1.0.1 | ISC | deps: none | versions: 2

dist
.tarball: https://registry.npmjs.org/testnpm2/-/testnpm2-1.0.1.tgz
.shasum: 8c7b209a673c360e540ab2777242171fd30fdee9
.integrity: sha512-F4AQ+KmzhbOSlt7ae+X2O8IJktFZAcN6OK169TT4ny7M3e4Vje7NITZTOU31AtEk9L/Z8lrCrqinl/eY6WPuEw==

maintainers:
- fengmk2

dist-tags:
latest: 1.0.1
release-1: 1.0.1
```
