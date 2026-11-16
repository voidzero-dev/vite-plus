# create_yarn1_npx_fallback

Yarn 1 has no dlx command, so create runs the template through npx.

## `vp create vite:application --directory app --no-interactive --no-git --no-agent --no-editor --no-hooks`


## `vpt stat-file app/package.json --assert file`

the application was scaffolded

```
app/package.json: file
```

## `vpt grep-file app/package.json '"version": "1.22.22"'`

the project keeps the Yarn 1 pin

```
app/package.json: found "\"version\": \"1.22.22\""
```
