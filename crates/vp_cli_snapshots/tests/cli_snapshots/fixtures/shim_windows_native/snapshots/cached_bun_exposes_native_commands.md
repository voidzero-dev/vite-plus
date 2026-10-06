# cached_bun_exposes_native_commands

## `node prepare.cjs bun`


## `bun probe.cjs nested bun 'value with spaces' path\file`

An existing Bun cache gains .exe commands before they enter a package script's PATH.

```
bun: native executable, arguments and environment preserved
bun: native executable, arguments and environment preserved
bun: nested script uses the native executable
```

## `bunx probe.cjs args bunx 'value with spaces' path\file`

```
bunx: native executable, arguments and environment preserved
```

## `vpt pipe-stdin 'piped input' -- bun probe.cjs stdin bun`

```
bun: piped stdin preserved
```

## `SHIM_TEST_EXIT_CODE=7 bun probe.cjs args bun 'value with spaces' path\file`

**Exit code:** 7

```
bun: native executable, arguments and environment preserved
```
