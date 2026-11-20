# create_existing_yarn_without_catalog_support

Yarn before 4.10 receives concrete versions instead of unsupported catalog references.

## `vpt json-edit package.json packageManager yarn@4.9.0`


## `vpt json-edit package.json resolutions.typescript 5.9.3`

Yarn 4.9's built-in TypeScript patch predates TypeScript 7.


## `vp create vite:application --directory apps/web --no-interactive --no-agent --no-editor --no-hooks`


## `vpt stat-file yarn.lock --assert file`

```
yarn.lock: file
```

## `vpt print-file apps/web/package.json`

```
{
  "name": "web",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vp dev",
    "build": "tsc && vp build",
    "preview": "vp preview"
  },
  "devDependencies": {
    "typescript": "~7.0.2",
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
    "vite-plus": "<version>"
  }
}
```

## `vp install --frozen-lockfile`


## `cd apps/web && vp run build`

