# command_config_npm10

## `vp pm config --help`

should show help

```
VITE+ - The Unified Toolchain for the Web

Usage: vp pm config <COMMAND>

Manage package manager configuration

Commands:
  list    List all configuration
  get     Get configuration value
  set     Set configuration value
  delete  Delete configuration key

Options:
  -h, --help  Print help

Documentation: https://viteplus.dev/guide/install
```

## `vp pm config get vite-plus-pm-config-test-key --location project`

should get config value from project scope

```
test-value
```

## `vp pm config delete vite-plus-pm-config-test-key --location project`

should delete config key from project scope

```
```

## `vpt print-file .npmrc`

```
foo=bar
```
