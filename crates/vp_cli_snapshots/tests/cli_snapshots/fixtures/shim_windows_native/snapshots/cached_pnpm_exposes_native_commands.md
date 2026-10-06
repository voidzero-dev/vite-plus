# cached_pnpm_exposes_native_commands

## `node prepare.cjs pnpm`


## `pnpm probe.cjs nested pnpm 'value with spaces' path\file`

```
pnpm: native executable, arguments and environment preserved
pnpm: native executable, arguments and environment preserved
pnpm: nested script uses the native executable
```

## `pnpx probe.cjs args pnpx 'value with spaces' path\file`

```
pnpx: native executable, arguments and environment preserved
```

## `vpt pipe-stdin 'piped input' -- pnpm probe.cjs stdin pnpm`

```
pnpm: piped stdin preserved
```
