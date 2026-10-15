# pm_pkg_bun

## `vp pm pkg get name`

```
"pm-pkg-bun"
```

## `vp pm pkg get name --json`

```
"pm-pkg-bun"
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

## `vpt json-edit package.json name '"PM-PKG-BUN"'`


## `vp pm pkg fix`

```
```

## `vp pm pkg get`

```
{
  "name": "pm-pkg-bun",
  "packageManager": "bun@1.2.19",
  "private": true,
  "scripts": {},
  "version": "1.0.0"
}
```
