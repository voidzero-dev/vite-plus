# create_existing_yarn_aligned_vitest_catalog

Creating a member completes aligned Vitest entries in the named toolchain catalog without changing the default catalog.

## `vpt cp vitest-template.config.ts vite.config.ts`


## `vpt write-file .yarnrc.yml 'nodeLinker: node-modules
catalog:
  '\''@vitest/utils'\'': '\''^5.0.0'\''
catalogs:
  tools:
    vite-plus: '\''*'\''
'`


## `vp install`


## `vp create vitest-app --no-interactive --no-agent --no-editor --no-hooks`


## `vpt print-file apps/web/package.json`

```
{
  "name": "web",
  "private": true,
  "devDependencies": {
    "@vitest/utils": "catalog:tools",
    "vite-plus": "catalog:tools"
  }
}
```

## `vpt print-file .yarnrc.yml`

```
nodeLinker: node-modules
catalog:
  '@vitest/utils': '^5.0.0'
catalogs:
  tools:
    vite-plus: '*'
    '@vitest/utils': <version>
```

## `vp install --frozen-lockfile`

