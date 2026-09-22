# pnpm_unsupported_install_options

## `vpt json-edit package.json packageManager pnpm@11.24.0`


## `vp install ./dep --lockfile-only --frozen-lockfile`

warn for frozen-lockfile that pnpm add cannot accept, but preserve lockfile-only

```
VITE+ - The Unified Toolchain for the Web

warn: pnpm does not support --frozen-lockfile.

dependencies:
 install-option-dep link:dep

Done in <duration> using pnpm <version>
```

## `vpt stat-file pnpm-lock.yaml --assert file`

```
pnpm-lock.yaml: file
```

## `vpt stat-file node_modules --assert missing`

```
node_modules: missing
```
