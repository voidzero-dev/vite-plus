# create_from_bun_monorepo_subdir

## `cd apps/website && vp create --no-interactive vite:generator`

from workspace subdir with object-form workspaces


## `vpt stat-file tools/vite-plus-generator/package.json --assert file`

created at tools/vite-plus-generator

```
tools/vite-plus-generator/package.json: file
```

## `vpt stat-file apps/website/tools/vite-plus-generator/package.json --assert-not file`

not created inside apps/website/

```
apps/website/tools/vite-plus-generator/package.json: missing
```

## `cd apps && vp create --no-interactive vite:application`

from workspace parent dir


## `vpt stat-file apps/vite-plus-application/package.json --assert file`

created at apps/vite-plus-application

```
apps/vite-plus-application/package.json: file
```

## `cd scripts/helper && vp create --no-interactive vite:library`

from non-workspace dir


## `vpt stat-file packages/vite-plus-library/package.json --assert file`

created at packages/vite-plus-library

```
packages/vite-plus-library/package.json: file
```

## `vpt print-file package.json`

verify workspaces object form preserved after workspace update

```
{
  "name": "test-bun-monorepo",
  "version": "0.0.0",
  "private": true,
  "workspaces": {
    "packages": [
      "apps/*",
      "packages/*",
      "tools/*"
    ],
    "catalog": {
      "vite": "npm:@voidzero-dev/vite-plus-core@latest",
      "vitest": "^4.0.0",
      "vite-plus": "latest"
    }
  },
  "packageManager": "bun@1.3.11"
}
```
