# npm_option_version_guards

## `vp pm publish --dry-run`

Reject preview before an npm version that would publish despite dry-run. The fixture is also private for safety.

**Exit code:** 1

```
npm < 5.10.0 does not support --dry-run.
```

## `vp pm publish --provenance`

**Exit code:** 1

```
npm < 9.5.0 does not support --provenance.
```

## `vp pm pack --filter app`

**Exit code:** 1

```
npm < 7.9.0 does not support --filter.
```

## `vp pm pack --pack-destination dist`

**Exit code:** 1

```
npm < 7.18.0 does not support --pack-destination.
```

## `vp install --filter app`

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

npm < 7.14.0 does not support --filter.
```

## `vp install --workspace-root`

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

npm < 8.1.0 does not support --workspace-root.
```
