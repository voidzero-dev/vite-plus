# env_install_interrupt

## `vp install -g ./long-time-install-package`

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
✓ Installed @scope/long-time-install-package 0.0.0
  Bins: long-time-install-package
```

## `long-time-install-package`

```
long-time-install-package
```

## `node test-reinstall-interrupt.js`

Reinstall but interrupt


## `long-time-install-package`

Original package should be still runnable

```
long-time-install-package
```

## `node check-stale-packages.js --expect-stale`

Interrupted reinstall should leave stale package

```
interrupted stale package exists
```

## `vp install -g ./long-time-install-package`

Successful reinstall should clean stale packages

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
✓ Installed @scope/long-time-install-package 0.0.0
  Bins: long-time-install-package
```

## `node check-stale-packages.js`

```
interrupted stale package removed
```
