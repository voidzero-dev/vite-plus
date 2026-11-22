# update_without_latest

## `vp install -g is-number@npm:testnpm2@^1`


## `vp outdated -g is-number@6 --format list`

An explicit spec queries is-number, which has no latest tag

**Exit code:** 1

```
warn: npm resolution failed for is-number@6: No latest dist-tag for is-number; skipping
```

## `vp update -g is-number@6`

Updating only needs the wanted version

```
info: Updating 1 global package with Node.js <version>
 Updated is-number to 6.0.0
```

## `vp list -g`

```
Package           Node version   Binaries
---               ---            ---
is-number@6.0.0   <version>
```
