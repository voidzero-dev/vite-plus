# node_version_pin_reports_created_then_updated

## `vp env pin 22.13.0 --target node-version --no-install`

a new .node-version is reported as created

```
VITE+ - The Unified Toolchain for the Web

✓ Pinned Node.js version to 22.13.0
  Created .node-version in <workspace>
note: Version will be downloaded on first use.
```

## `vp env pin 24.11.0 --no-install --force`

overwriting the existing .node-version is reported as updated

```
VITE+ - The Unified Toolchain for the Web

✓ Pinned Node.js version to 24.11.0
  Updated .node-version in <workspace>
note: Version will be downloaded on first use.
```

## `vpt print-file .node-version`

```
24.11.0
```
