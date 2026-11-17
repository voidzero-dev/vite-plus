# create_existing_yarn_named_default_catalog

Yarn's named default catalog must remain catalog:default, not catalog:.

## `vp create vite:application --directory apps/seed --no-interactive --no-agent --no-editor --no-hooks`


## `vpt replace-file-content .yarnrc.yml 'catalog:
' 'catalogs:
  default:
'`


## `vpt replace-file-content .yarnrc.yml '  vite:' '    vite:'`


## `vpt replace-file-content .yarnrc.yml '  vite-plus:' '    vite-plus:'`


## `vpt json-edit apps/seed/package.json devDependencies.vite catalog:default`


## `vpt json-edit apps/seed/package.json devDependencies.vite-plus catalog:default`


## `vp install`

Verify that the existing workspace installs before adding another app.


## `vp create vite:application --directory apps/web --no-interactive --no-agent --no-editor --no-hooks`


## `vpt print-file .yarnrc.yml`

```
nodeLinker: node-modules
catalogs:
  default:
    vite: npm:@voidzero-dev/vite-plus-core@<version>
    vite-plus: <version>
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
    "vite": "catalog:default",
    "vite-plus": "catalog:default"
  }
}
```

## `vp install --frozen-lockfile`


## `cd apps/web && vp run build`

