# core_version_guard

## `vp build`

**Exit code:** 1

```
error: Failed to resolve vite command: GenericFailure, Error: The project's `vite` alias resolves to @voidzero-dev/vite-plus-core@<version>, but this vite-plus CLI requires @voidzero-dev/vite-plus-core@<version>: the two packages are published in lockstep and other pairings are untested. A dependency bot usually causes this by updating vite-plus and the `vite` alias in separate PRs. Update the `vite` alias to npm:@voidzero-dev/vite-plus-core@<version> where it is declared (catalog, overrides, resolutions, or dependencies), or run `vp migrate` to realign it. Set VP_SKIP_CORE_VERSION_CHECK=1 to skip this check.
```

## `vp test`

**Exit code:** 1

```
error: Failed to resolve test command: GenericFailure, Error: The project's `vite` alias resolves to @voidzero-dev/vite-plus-core@<version>, but this vite-plus CLI requires @voidzero-dev/vite-plus-core@<version>: the two packages are published in lockstep and other pairings are untested. A dependency bot usually causes this by updating vite-plus and the `vite` alias in separate PRs. Update the `vite` alias to npm:@voidzero-dev/vite-plus-core@<version> where it is declared (catalog, overrides, resolutions, or dependencies), or run `vp migrate` to realign it. Set VP_SKIP_CORE_VERSION_CHECK=1 to skip this check.
```

## `vp build app`

the guard checks the positional root, where vite is real Vite

```
note: `vp build app` sets Vite's root without changing the working directory. To run as if started there, use `vp -C app build`.
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
