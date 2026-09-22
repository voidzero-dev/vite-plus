# install_with_packages_drops_install_only_options

## `vp install ./dep --fix-lockfile --resolution-only --lockfile-only`

diagnose install-only options before converting to add, while preserving common options

```
VITE+ - The Unified Toolchain for the Web

warn: install with package names does not support --fix-lockfile.
warn: install with package names does not support --resolution-only.

up to date, audited 3 packages in <duration>

found 0 vulnerabilities
```

## `vpt stat-file package-lock.json --assert file`

```
package-lock.json: file
```

## `vpt stat-file node_modules --assert missing`

```
node_modules: missing
```

## `vpt print-file package.json`

```
{
  "name": "install-package-options",
  "version": "1.0.0",
  "private": true,
  "packageManager": "npm@11.13.0",
  "dependencies": {
    "install-option-dep": "file:dep"
  }
}
```
