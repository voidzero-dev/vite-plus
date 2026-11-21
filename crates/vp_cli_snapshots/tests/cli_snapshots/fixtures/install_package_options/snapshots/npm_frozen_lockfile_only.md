# npm_frozen_lockfile_only

## `vp install ./dep`


## `vp install --frozen-lockfile --lockfile-only`

an in-sync lockfile passes without reinstalling node_modules

```
VITE+ - The Unified Toolchain for the Web

up to date in <duration>
```

## `vpt stat-file node_modules/install-option-dep --assert-not missing`

```
node_modules/install-option-dep: symlink
```

## `vpt json-edit package.json dependencies.install-option-dep file:dep-v2`


## `vp install --frozen-lockfile --lockfile-only`

a lockfile out of sync with package.json fails

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

npm error code EUSAGE
npm error
npm error `npm ci` can only install packages when your package.json and package-lock.json or npm-shrinkwrap.json are in sync. Please update your lock file with `npm install` before continuing.
npm error
npm error Invalid: lock file's install-option-dep@1.0.0 does not satisfy install-option-dep@2.0.0
npm error Missing: install-option-dep@2.0.0 from lock file
npm error
npm error Clean install a project
npm error
npm error Usage:
npm error npm ci
npm error
npm error Options:
npm error [--install-strategy <hoisted|nested|shallow|linked>] [--legacy-bundling]
npm error [--global-style] [--omit <dev|optional|peer> [--omit <dev|optional|peer> ...]]
npm error [--include <prod|dev|optional|peer> [--include <prod|dev|optional|peer> ...]]
npm error [--strict-peer-deps] [--foreground-scripts] [--ignore-scripts]
npm error [--allow-git <all|none|root>] [--no-audit] [--no-bin-links] [--no-fund]
npm error [--dry-run]
npm error [-w|--workspace <workspace-name> [-w|--workspace <workspace-name> ...]]
npm error [--workspaces] [--include-workspace-root] [--install-links]
npm error
npm error   --install-strategy
npm error     Sets the strategy for installing packages in node_modules.
npm error
npm error   --legacy-bundling
npm error     Instead of hoisting package installs in `node_modules`, install packages
npm error
npm error   --global-style
npm error     Only install direct dependencies in the top level `node_modules`,
npm error
npm error   --omit
npm error     Dependency types to omit from the installation tree on disk.
npm error
npm error   --include
npm error     Option that allows for defining which types of dependencies to install.
npm error
npm error   --strict-peer-deps
npm error     If set to `true`, and `--legacy-peer-deps` is not set, then _any_
npm error
npm error   --foreground-scripts
npm error     Run all build scripts (ie, `preinstall`, `install`, and
npm error
npm error   --ignore-scripts
npm error     If true, npm does not run scripts specified in package.json files.
npm error
npm error   --allow-git
npm error     Limits the ability for npm to fetch dependencies from git references.
npm error
npm error   --audit
npm error     When "true" submit audit reports alongside the current npm command to the
npm error
npm error   --bin-links
npm error     Tells npm to create symlinks (or `.cmd` shims on Windows) for package
npm error
npm error   --fund
npm error     When "true" displays the message at the end of each `npm install`
npm error
npm error   --dry-run
npm error     Indicates that you don't want npm to make any changes and that it should
npm error
npm error   -w|--workspace
npm error     Enable running a command in the context of the configured workspaces of the
npm error
npm error   --workspaces
npm error     Set to true to run the command in the context of **all** configured
npm error
npm error   --include-workspace-root
npm error     Include the workspace root when workspaces are enabled for a command.
npm error
npm error   --install-links
npm error     When set file: protocol dependencies will be packed and installed as
npm error
npm error
npm error aliases: clean-install, ic, install-clean, isntall-clean
npm error
npm error Run "npm help ci" for more info
npm error A complete log of this run can be found in: <home>/.npm/_logs/<timestamp>-debug-0.log
```

## `vpt stat-file node_modules/install-option-dep --assert-not missing`

```
node_modules/install-option-dep: symlink
```

## `vpt print-file package-lock.json`

```
{
  "name": "install-package-options",
  "version": "1.0.0",
  "lockfileVersion": 3,
  "requires": true,
  "packages": {
    "": {
      "name": "install-package-options",
      "version": "1.0.0",
      "dependencies": {
        "install-option-dep": "file:dep"
      }
    },
    "dep": {
      "name": "install-option-dep",
      "version": "1.0.0"
    },
    "node_modules/install-option-dep": {
      "resolved": "dep",
      "link": true
    }
  }
}
```
