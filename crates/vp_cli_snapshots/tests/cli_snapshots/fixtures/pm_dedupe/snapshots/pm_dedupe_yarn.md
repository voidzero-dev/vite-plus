# pm_dedupe_yarn

## `vp dedupe -- --silent`

Yarn Classic falls back to install because install already deduplicates dependencies

```
warn: Yarn Classic dedupes during install, falling back to yarn install
```

## `vp dedupe --check -- --silent`

reject unsupported --check without falling back to install

**Exit code:** 1

```
yarn < 2 does not support --check.
```

## `vpt print-file package.json`

verify Yarn Classic manifest is unchanged

```
{
  "name": "pm-dedupe-yarn",
  "version": "1.0.0",
  "private": true,
  "license": "MIT",
  "packageManager": "yarn@1.22.22"
}
```
