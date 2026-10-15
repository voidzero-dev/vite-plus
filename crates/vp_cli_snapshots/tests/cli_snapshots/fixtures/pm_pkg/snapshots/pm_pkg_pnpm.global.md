# pm_pkg_pnpm

## `vp pm pkg get name`

pnpm preserves its native unquoted string output

```
pm-pkg-pnpm
```

## `vp pm pkg get name --json`

```
"pm-pkg-pnpm"
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
  "name": "pm-pkg-pnpm",
  "packageManager": "pnpm@11.3.0",
  "private": true,
  "version": "1.0.0"
}
```
