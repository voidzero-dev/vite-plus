# core_version_guard

## `vp build`

**Exit code:** 1

```
error: Your `vite` alias uses @voidzero-dev/vite-plus-core@<version>.
This Vite+ CLI requires @voidzero-dev/vite-plus-core@<version>.

Choose a fix:
- Update the `vite` alias to npm:@voidzero-dev/vite-plus-core@<version>.
- Run `vp migrate`.

To skip this check, set VP_SKIP_CORE_VERSION_CHECK=1.
```

## `vp test`

**Exit code:** 1

```
error: Your `vite` alias uses @voidzero-dev/vite-plus-core@<version>.
This Vite+ CLI requires @voidzero-dev/vite-plus-core@<version>.

Choose a fix:
- Update the `vite` alias to npm:@voidzero-dev/vite-plus-core@<version>.
- Run `vp migrate`.

To skip this check, set VP_SKIP_CORE_VERSION_CHECK=1.
```

## `vp build app`

the guard checks the positional root, where vite is real Vite

```
✓ 2 modules transformed.
computing gzip size...
app/dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```

## `VP_SKIP_CORE_VERSION_CHECK=1 vp build`

VP_SKIP_CORE_VERSION_CHECK=1 skips the guard

```
✓ 2 modules transformed.
computing gzip size...
dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
