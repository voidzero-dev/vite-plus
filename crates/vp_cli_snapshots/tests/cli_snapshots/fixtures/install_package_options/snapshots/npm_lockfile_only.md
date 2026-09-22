# npm_lockfile_only

## `vp install ./dep --lockfile-only`

adding a package preserves lockfile-only

```
VITE+ - The Unified Toolchain for the Web

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
