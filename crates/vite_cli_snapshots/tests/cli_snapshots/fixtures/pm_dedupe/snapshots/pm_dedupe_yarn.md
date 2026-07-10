# pm_dedupe_yarn

## `vp dedupe -- --silent`

Yarn Classic falls back to install because install already deduplicates dependencies

```
warn: Yarn Classic dedupes during install, falling back to yarn install
```

## `vpt print-file package.json`

verify Yarn Classic completed

```
{
  "name": "pm-dedupe-yarn",
  "version": "1.0.0",
  "private": true,
  "license": "MIT",
  "packageManager": "yarn@1.22.22"
}
```
