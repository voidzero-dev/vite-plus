# install_package_manager_noninteractive

CI and piped commands use the selector's pnpm default without prompting or pinning the manager.

## `CI=true vp install --lockfile-only`

```
VITE+ - The Unified Toolchain for the Web

Done in <duration> using pnpm <version>
```

## `vpt stat-file pnpm-lock.yaml --assert file`

```
pnpm-lock.yaml: file
```

## `vpt rm pnpm-lock.yaml`

```
```

## `vp install --lockfile-only`

```

Done in <duration> using pnpm <version>
```

## `vpt stat-file pnpm-lock.yaml --assert file`

```
pnpm-lock.yaml: file
```

## `vpt print-file package.json`

```
{"name":"install-package-manager-selection","private":true}
```
