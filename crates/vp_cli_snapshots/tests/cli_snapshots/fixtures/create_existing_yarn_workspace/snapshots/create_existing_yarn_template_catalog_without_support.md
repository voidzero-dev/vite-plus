# create_existing_yarn_template_catalog_without_support

A template's catalog reference must not add catalog settings that Yarn before 4.10 rejects.

## `vpt json-edit package.json packageManager yarn@4.9.0`


## `vpt cp vitest-template.config.ts vite.config.ts`


## `vpt write-file .yarnrc.yml 'nodeLinker: node-modules
'`


## `vp create vitest-app --no-interactive --no-agent --no-editor --no-hooks`


## `vpt print-file .yarnrc.yml`

```
nodeLinker: node-modules
```

## `yarn config get nodeLinker`

Yarn still accepts the root configuration.

```
node-modules
```
