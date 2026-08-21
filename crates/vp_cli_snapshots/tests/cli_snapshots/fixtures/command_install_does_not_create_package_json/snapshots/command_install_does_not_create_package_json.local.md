# command_install_does_not_create_package_json

Package-manager commands never create a manifest implicitly.

## `vpt stat-file package.json --assert-not file`

verify no package.json exists

```
package.json: missing
```

## `vp install --silent`

install should require package.json

**Exit code:** 1

```
error: Package not found in workspace: `<workspace>`
```

## `vpt stat-file package.json --assert-not file`

install should not create package.json

```
package.json: missing
```

## `vp add testnpm2 -D`

add should require package.json

**Exit code:** 1

```
error: Package not found in workspace: `<workspace>`
```

## `vpt stat-file package.json --assert-not file`

add should not create package.json

```
package.json: missing
```
