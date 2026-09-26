# command_env_install_parallel

## `vp install -g --concurrency 1 ./parallel-pkg-a ./parallel-pkg-b`

Install multiple global packages

```
VITE+ - The Unified Toolchain for the Web

\x1b[94;1minfo: Installing 2 global packages with Node.js <version>
\x1b[32m+ Installed \x1b[1mparallel-pkg-a \x1b[1m1.0.0
  Bins: \x1b[1mparallel-a

\x1b[32m+ Installed \x1b[1mparallel-pkg-b \x1b[1m2.0.0
  Bins: \x1b[1mparallel-b
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

Installation and removal use plus/minus markers and the same package styling

```
\x1b[31m- Uninstalled \x1b[1mparallel-pkg-a
\x1b[31m- Uninstalled \x1b[1mparallel-pkg-b
```
