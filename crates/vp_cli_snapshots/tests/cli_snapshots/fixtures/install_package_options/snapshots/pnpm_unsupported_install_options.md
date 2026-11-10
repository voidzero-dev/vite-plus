# pnpm_unsupported_install_options

## `vpt json-edit package.json packageManager pnpm@11.24.0`


## `vp install ./dep --lockfile-only --frozen-lockfile`

reject frozen-lockfile before pnpm add can modify the project

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

pnpm does not support --frozen-lockfile.
```

## `vpt stat-file pnpm-lock.yaml --assert missing`

```
pnpm-lock.yaml: missing
```

## `vpt stat-file node_modules --assert missing`

```
node_modules: missing
```
