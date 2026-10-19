# yarn_option_version_guards

## `vp install --fix-lockfile`

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

yarn < 4.0.0 does not support --fix-lockfile.
```

## `vp add testnpm2 --ignore-scripts`

Yarn 2 supports skip-builds for install, but not for add.

**Exit code:** 1

```
yarn >= 2 && < 3 does not support --ignore-scripts.
```
