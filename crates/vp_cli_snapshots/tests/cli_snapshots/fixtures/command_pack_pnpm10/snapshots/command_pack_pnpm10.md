# command_pack_pnpm10

## `vp pm pack --help`

should show help

```
VITE+ - The Unified Toolchain for the Web

Usage: vp pm pack [OPTIONS] [-- <PASS_THROUGH_ARGS>...]

Create a tarball of the package

Arguments:
  [PASS_THROUGH_ARGS]...  Additional arguments

Options:
  -r, --recursive                        Pack all workspace packages
  --filter <PATTERN>                     Filter packages to pack
  --out <OUT>                            Output path for the tarball
  --pack-destination <PACK_DESTINATION>  Directory where the tarball will be saved
  --pack-gzip-level <PACK_GZIP_LEVEL>    Gzip compression level (0-9)
  --json                                 Output in JSON format
  -h, --help                             Print help

Documentation: https://viteplus.dev/guide/package-management
```

## `vp pm pack`

should pack current package

```
📦  command-pack-pnpm10@1.0.0
Tarball Contents
package.json
Tarball Details
command-pack-pnpm10-1.0.0.tgz
```

## `vpt rm -f command-pack-pnpm10-1.0.0.tgz`


## `vp pm pack --out ./dist/package.tgz`

should pack with output file

```
📦  command-pack-pnpm10@1.0.0
Tarball Contents
package.json
Tarball Details
<workspace>/dist/package.tgz
```

## `vpt rm -rf ./dist`

```
```

## `vp pm pack --pack-destination ./dist`

should pack with destination

```
📦  command-pack-pnpm10@1.0.0
Tarball Contents
package.json
Tarball Details
<workspace>/dist/command-pack-pnpm10-1.0.0.tgz
```

## `vpt rm -rf ./dist`

```
```

## `vp pm pack --json --pack-gzip-level 9`

should pack with gzip compression level

```
{
  "name": "command-pack-pnpm10",
  "version": "1.0.0",
  "filename": "command-pack-pnpm10-1.0.0.tgz",
  "files": [
    {
      "path": "package.json"
    }
  ]
}
```

## `vpt rm -f command-pack-pnpm10-1.0.0.tgz`


## `vp pm pack --json`

should pack with json output

```
{
  "name": "command-pack-pnpm10",
  "version": "1.0.0",
  "filename": "command-pack-pnpm10-1.0.0.tgz",
  "files": [
    {
      "path": "package.json"
    }
  ]
}
```

## `vpt rm -f command-pack-pnpm10-1.0.0.tgz`


## `vp pm pack -- --loglevel=warn`

should support pass through arguments

```
📦  command-pack-pnpm10@1.0.0
Tarball Contents
package.json
Tarball Details
command-pack-pnpm10-1.0.0.tgz
```

## `vpt rm -f command-pack-pnpm10-1.0.0.tgz`


## `vpt json-edit package.json packageManager pnpm@10.10.0`


## `vp pm pack --recursive`

Reject recursive pack before pnpm 10.11.0.

**Exit code:** 1

```
pnpm < 10.11.0 does not support --recursive.
```

## `vpt json-edit package.json packageManager pnpm@9.15.9`


## `vp pm pack --out package.tgz`

Reject --out before pnpm 10.0.0.

**Exit code:** 1

```
pnpm < 10.0.0 does not support --out.
```

## `vpt json-edit package.json packageManager pnpm@9.13.2`


## `vp pm pack --json`

Reject --json before pnpm 9.14.0.

**Exit code:** 1

```
pnpm < 9.14.0 does not support --json.
```

## `vpt json-edit package.json packageManager pnpm@8.2.0`


## `vp pm pack --pack-gzip-level 0`

Reject --pack-gzip-level before pnpm 8.3.0, including an explicit zero.

**Exit code:** 1

```
pnpm < 8.3.0 does not support --pack-gzip-level.
```
