# command_pack_pnpm11

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

Documentation: https://viteplus.dev/guide/install
```

## `vp pm pack`

should pack current package

```
package: command-pack-pnpm11@1.0.0
Tarball Contents
package.json
Tarball Details
command-pack-pnpm11-1.0.0.tgz
```

## `vpt rm -f command-pack-pnpm11-1.0.0.tgz`


## `vp pm pack --out ./dist/package.tgz`

should pack with output file

```
package: command-pack-pnpm11@1.0.0
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
package: command-pack-pnpm11@1.0.0
Tarball Contents
package.json
Tarball Details
<workspace>/dist/command-pack-pnpm11-1.0.0.tgz
```

## `vpt rm -rf ./dist`

```
```

## `vp pm pack --json --pack-gzip-level 9`

should pack with gzip compression level

```
{
  "name": "command-pack-pnpm11",
  "version": "1.0.0",
  "filename": "command-pack-pnpm11-1.0.0.tgz",
  "files": [
    {
      "path": "package.json"
    }
  ]
}
```

## `vpt rm -f command-pack-pnpm11-1.0.0.tgz`


## `vp pm pack --json`

should pack with json output

```
{
  "name": "command-pack-pnpm11",
  "version": "1.0.0",
  "filename": "command-pack-pnpm11-1.0.0.tgz",
  "files": [
    {
      "path": "package.json"
    }
  ]
}
```

## `vpt rm -f command-pack-pnpm11-1.0.0.tgz`


## `vp pm pack -- --loglevel=warn`

should support pass through arguments

```
package: command-pack-pnpm11@1.0.0
Tarball Contents
package.json
Tarball Details
command-pack-pnpm11-1.0.0.tgz
```

## `vpt rm -f command-pack-pnpm11-1.0.0.tgz`

