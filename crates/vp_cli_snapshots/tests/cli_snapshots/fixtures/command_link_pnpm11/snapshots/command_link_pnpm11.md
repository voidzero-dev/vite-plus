# command_link_pnpm11

## `vp link -h`

should show help message

```
VITE+ - The Unified Toolchain for the Web

Usage: vp link [PACKAGE|DIR] [ARGS]...

Link packages for local development

Arguments:
  [PACKAGE|DIR]  Package name or directory to link
  [ARGS]...      Arguments to pass to package manager

Options:
  -h, --help  Print help

Documentation: https://viteplus.dev/guide/install
```

## `vp install`

install initial dependencies

```
VITE+ - The Unified Toolchain for the Web

dependencies:
 testnpm2 1.0.1

Done in <duration> using pnpm <version>
```

## `vpt mkdir -p ../test-lib-pnpm`

create test library

```
```

## `vpt write-file ../test-lib-pnpm/package.json '{"name": "testnpm2", "version": "1.0.0"}
'`

```
```

## `vp link ../test-lib-pnpm`

should link local directory

```
Packages: -1
-

dependencies:
- testnpm2 1.0.1
 testnpm2 1.0.0 <- ../test-lib-pnpm
```

## `vpt print-file package.json pnpm-lock.yaml`

```
{
  "name": "command-link-pnpm11",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "*"
  },
  "packageManager": "pnpm@11.0.6"
}
lockfileVersion: '9.0'

settings:
  autoInstallPeers: true
  excludeLinksFromLockfile: false

overrides:
  testnpm2: link:../test-lib-pnpm

importers:

  .:
    dependencies:
      testnpm2:
        specifier: link:../test-lib-pnpm
        version: link:../test-lib-pnpm
```

## `vp ln ../test-lib-pnpm`

should work with ln alias

```
Lockfile is up to date, resolution step is skipped
```

## `vpt print-file package.json pnpm-lock.yaml`

```
{
  "name": "command-link-pnpm11",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "*"
  },
  "packageManager": "pnpm@11.0.6"
}
lockfileVersion: '9.0'

settings:
  autoInstallPeers: true
  excludeLinksFromLockfile: false

overrides:
  testnpm2: link:../test-lib-pnpm

importers:

  .:
    dependencies:
      testnpm2:
        specifier: link:../test-lib-pnpm
        version: link:../test-lib-pnpm
```

## `vp unlink ../test-lib-pnpm -- --no-frozen-lockfile`

should unlink the package (pnpm v11 requires --no-frozen-lockfile under CI=true to avoid ERR_PNPM_LOCKFILE_CONFIG_MISMATCH)

```
Already up to date
```

## `vp unlink testnpm2 -- --no-frozen-lockfile`

```
testnpm2 is linked to <workspace>/node_modules from <case>/test-lib-pnpm
Already up to date

dependencies:
- testnpm2 1.0.0
 testnpm2 1.0.1
```

## `vpt print-file package.json pnpm-lock.yaml`

```
{
  "name": "command-link-pnpm11",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "*"
  },
  "packageManager": "pnpm@11.0.6"
}
lockfileVersion: '9.0'

settings:
  autoInstallPeers: true
  excludeLinksFromLockfile: false

importers:

  .:
    dependencies:
      testnpm2:
        specifier: '*'
        version: 1.0.1

packages:

  testnpm2@1.0.1:
    resolution: {integrity: sha512-F4AQ+KmzhbOSlt7ae+X2O8IJktFZAcN6OK169TT4ny7M3e4Vje7NITZTOU31AtEk9L/Z8lrCrqinl/eY6WPuEw==}

snapshots:

  testnpm2@1.0.1: {}
```
