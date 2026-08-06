# command_env_install_parallel

## `vp install -g --concurrency 1 ./parallel-pkg-a ./parallel-pkg-b`

Install multiple global packages

```
VITE+ - The Unified Toolchain for the Web

info: Installing 2 global packages with Node.js <version>
✓ Installed parallel-pkg-a 1.0.0
  Bins: parallel-a

✓ Installed parallel-pkg-b 2.0.0
  Bins: parallel-b
```

## `parallel-a`

Both binaries should be callable

```
parallel-a ok
```

## `parallel-b`

```
parallel-b ok
```

## `vp remove -g parallel-pkg-a parallel-pkg-b`

Cleanup

```
Uninstalled parallel-pkg-a
Uninstalled parallel-pkg-b
```
