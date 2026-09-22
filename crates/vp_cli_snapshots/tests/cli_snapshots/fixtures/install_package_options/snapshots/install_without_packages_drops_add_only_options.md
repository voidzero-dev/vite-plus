# install_without_packages_drops_add_only_options

## `vp install --save-exact --save-peer --save-optional --save-catalog --lockfile-only`

diagnose add-only options without package names instead of reporting manager support

```
VITE+ - The Unified Toolchain for the Web

warn: install without package names does not support --save-exact.
warn: install without package names does not support --save-peer.
warn: install without package names does not support --save-optional.
warn: install without package names does not support --save-catalog.

up to date, audited 1 package in <duration>

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
  "packageManager": "npm@11.13.0"
}
```
