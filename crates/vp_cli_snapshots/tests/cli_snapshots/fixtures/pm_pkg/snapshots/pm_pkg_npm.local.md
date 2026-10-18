# pm_pkg_npm

## `vp pm pkg get name`

get output has no Vite+ header when piped

```
"pm-pkg-npm"
```

## `vp pm pkg set 'scripts.build=vite build' 'description=managed by vp'`

```
```

## `vp pm pkg set private=true --json`

```
```

## `vp pm pkg get scripts.build description private`

```
{
  "scripts.build": "vite build",
  "description": "managed by vp",
  "private": true
}
```

## `vp pm pkg delete scripts.build description`

```
```

## `vpt json-edit package.json scripts '"invalid"'`


## `vp pm pkg fix`

```
```

## `vp pm pkg get`

```
{
  "name": "pm-pkg-npm",
  "packageManager": "npm@11.11.1",
  "private": true,
  "version": "1.0.0"
}
```
