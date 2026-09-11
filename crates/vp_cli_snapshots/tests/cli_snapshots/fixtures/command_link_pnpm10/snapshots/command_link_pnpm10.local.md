# command_link_pnpm10

## `vp link -h`

should show help message

```
Link packages for local development

Usage: vp link [PACKAGE|DIR] [ARGS]...

Arguments:
  [PACKAGE|DIR]  Package name or directory to link
  [ARGS]...      Arguments to pass to package manager

Options:
  -h, --help  Print help
```

## `vp install`

install initial dependencies

```

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
  "name": "command-link-pnpm10",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "*"
  },
  "packageManager": "pnpm@10.19.0"
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
  "name": "command-link-pnpm10",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "*"
  },
  "packageManager": "pnpm@10.19.0"
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

## `vp unlink ../test-lib-pnpm`

should unlink the package

```
Nothing to unlink
```

## `vp unlink testnpm2`

```
Nothing to unlink
```

## `vpt print-file package.json pnpm-lock.yaml`

```
{
  "name": "command-link-pnpm10",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "*"
  },
  "packageManager": "pnpm@10.19.0"
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
