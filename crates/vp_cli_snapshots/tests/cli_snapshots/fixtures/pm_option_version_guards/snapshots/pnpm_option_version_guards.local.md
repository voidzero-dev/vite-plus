# pnpm_option_version_guards

## `vp add testnpm2 --allow-build testnpm2`

```

dependencies:
 testnpm2 1.0.1

Done in <duration> using pnpm <version>
```

## `vp add testnpm2 --save-catalog-name web`

```
✓ Lockfile passes supply-chain policies (verified <duration> ago)
Already up to date

Done in <duration> using pnpm <version>
```

## `vp install testnpm2 --save-catalog`

The install alias must use the add support checks.

```
[WARN] Replaced "testnpm2" ("catalog:web") with "testnpm2" from a different source.
✓ Lockfile passes supply-chain policies (verified <duration> ago)
Lockfile is up to date, resolution step is skipped

Done in <duration> using pnpm <version>
```

## `vp pm pack --recursive`

```
package: pm-option-version-guards@1.0.0
Tarball Contents
package.json
pnpm-workspace.yaml
Tarball Details
pm-option-version-guards-1.0.0.tgz
```

## `vp pm list --find-by finder`

**Exit code:** 1

```
Error: ERR_PNPM_FINDER_NOT_FOUND

  x No finder with name finder is found
```

## `vp why --find-by finder`

A finder replaces package names, but still requires a supported pnpm version.

**Exit code:** 1

```
Error: ERR_PNPM_FINDER_NOT_FOUND

  x No finder with name finder is found
```
