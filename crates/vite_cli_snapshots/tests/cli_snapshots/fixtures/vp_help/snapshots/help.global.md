# help

Top-level help output, one snapshot per flavor. The parity matrix keeps the two command surfaces honest.

## `vp help`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp [COMMAND]

Start:
  create      Create a new project from a template
  migrate     Migrate an existing project to Vite+
  config      Configure hooks and agent integration
  staged      Run linters on staged files
  install, i  Install all dependencies, or add packages if package names are provided
  env         Manage Node.js versions

Develop:
  dev          Run the development server
  check        Run format, lint, and type checks
  lint         Lint code
  fmt, format  Format code
  test         Run tests

Execute:
  run    Run tasks (also available as standalone `vpr`)
  exec   Execute a command from local node_modules/.bin
  node   Run a Node.js script (shorthand for `env exec node`)
  dlx    Execute a package binary without installing it as a dependency
  cache  Manage the task cache

Build:
  build    Build for production
  pack     Build library
  preview  Preview production build

Manage Dependencies:
  add                        Add packages to dependencies
  remove, rm, un, uninstall  Remove packages from dependencies
  update, up                 Update packages to their latest versions
  dedupe                     Deduplicate dependencies by removing older versions
  outdated                   Check for outdated packages
  list, ls                   List installed packages
  why, explain               Show why a package is installed
  info, view, show           View package information from the registry
  link, ln                   Link packages for local development
  unlink                     Unlink packages
  rebuild                    Rebuild native modules
  pm                         Forward a command to the package manager

Maintain:
  upgrade  Update vp itself to the latest version
  implode  Remove vp and all related data

Documentation: https://viteplus.dev/guide/

Options:
  -V, --version  Print version
  -h, --help     Print help
```
