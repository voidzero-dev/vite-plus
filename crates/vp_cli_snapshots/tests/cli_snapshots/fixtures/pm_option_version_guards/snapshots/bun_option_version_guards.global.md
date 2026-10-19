# bun_option_version_guards

## `vp update --prod`

**Exit code:** 1

```
bun < 1.4 does not support --prod.
```

## `vp update --no-optional`

**Exit code:** 1

```
bun < 1.4 does not support --no-optional.
```

## `vp update --recursive`

**Exit code:** 1

```
bun < 1.4 does not support --recursive without --interactive.
```

## `vp update --filter app`

Older Bun supports filters only in interactive update.

**Exit code:** 1

```
bun < 1.4 does not support --filter without --interactive.
```
