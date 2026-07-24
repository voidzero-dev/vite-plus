# pm_patch_bun

## `vp pm patch placeholder -- --version`

Bun receives the patch command

**Exit code:** 1

```
bun patch <version> (1e86cebd)
No packages! Deleted empty lockfile

[<duration>] done

error: package placeholder not found
```

## `vp pm patch-commit placeholder -- --version`

Bun receives patch commit through the --commit flag

**Exit code:** 1

```
bun patch <version> (1e86cebd)
error: Cannot find lockfile. Install packages with `bun install` before patching them.
```
