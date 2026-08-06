# command_config_yarn4

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

## `vp pm config set vite-plus-pm-config-test-key test-value --location project`

should set config value in project scope

**Exit code:** 1

```
Usage Error: Couldn't find a configuration settings named "vite-plus-pm-config-test-key"

$ yarn config set [--json] [-H,--home] <name> <value>
```

## `vp pm config get vite-plus-pm-config-test-key --location project`

should get config value from project scope

**Exit code:** 1

```
Usage Error: Couldn't find a configuration settings named "vite-plus-pm-config-test-key"

$ yarn config get [--why] [--json] [--no-redacted] <name>
```

## `vp pm config delete vite-plus-pm-config-test-key --location project`

should delete config key from project scope (uses yarn config unset)

**Exit code:** 1

```
Usage Error: Couldn't find a configuration settings named "vite-plus-pm-config-test-key"

$ yarn config unset [-H,--home] <name>
```
