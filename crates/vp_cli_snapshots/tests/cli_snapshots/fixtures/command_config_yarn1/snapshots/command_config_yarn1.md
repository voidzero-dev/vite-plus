# command_config_yarn1

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

reject project writes instead of changing the home config

**Exit code:** 1

```
Yarn Classic does not support --location project.
```

## `vp pm config get vite-plus-pm-config-test-key --location project`

reject unsupported project location

**Exit code:** 1

```
Yarn Classic does not support --location project.
```

## `vp pm config delete vite-plus-pm-config-test-key --location project`

reject project deletion instead of deleting from the home config

**Exit code:** 1

```
Yarn Classic does not support --location project.
```
