# pm_licenses_unsupported

## `vp pm licenses --help`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp pm licenses <COMMAND>

Inspect licenses of installed dependencies

Commands:
  list  List licenses of installed dependencies [alias: ls]

Options:
  -h, --help  Print help

Documentation: https://viteplus.dev/package-management/
```

## `vp pm licenses list --help`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp pm licenses list [OPTIONS] [-- <PASS_THROUGH_ARGS>...]

List licenses of installed dependencies

Arguments:
  [PASS_THROUGH_ARGS]...  Additional arguments

Options:
  --json      Output in the package manager's native JSON format
  -h, --help  Print help

Documentation: https://viteplus.dev/package-management/
```

## `vp pm licenses list`

npm warns and exits successfully without installing a scanner

```
warn: npm does not have a 'licenses' command.
```

## `vpt json-edit package.json packageManager yarn@4.18.0`


## `vp pm licenses list --json`

Yarn Modern still warns when JSON output is requested

```
warn: yarn@2+ does not have a 'licenses' command.
```

## `vpt json-edit package.json packageManager pnpm@7.16.0`


## `vp pm licenses list`

older pnpm versions warn and exit successfully

```
warn: pnpm licenses requires pnpm >= 7.17.0. Skipping.
```

## `vpt json-edit package.json packageManager bun@1.3.11`


## `vp pm licenses list`

older Bun versions warn and exit successfully

```
warn: bun pm licenses requires bun >= 1.4.0. Skipping.
```

## `vpt stat-file node_modules --assert missing`

```
node_modules: missing
```
