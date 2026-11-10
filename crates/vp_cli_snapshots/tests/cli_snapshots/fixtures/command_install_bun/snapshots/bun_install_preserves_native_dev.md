# bun_install_preserves_native_dev

## `vp install -D`

Bun's native dev option does not exclude production dependencies

```
VITE+ - The Unified Toolchain for the Web

bun install <version> (<hash>)

 prod-only-fixture@prod-dep

1 package installed [<duration>]
```

## `vpt stat-file node_modules/prod-only-fixture/package.json --assert file`

```
node_modules/prod-only-fixture/package.json: file
```

## `vp install --dev --frozen-lockfile`

preserve the dev option in frozen-lockfile mode too

```
VITE+ - The Unified Toolchain for the Web

bun install <version> (<hash>)

 prod-only-fixture@prod-dep

1 package installed [<duration>]
```

## `vpt print-file package.json`

```
{
  "name": "command-install-bun",
  "version": "1.0.0",
  "private": true,
  "dependencies": {
    "prod-only-fixture": "file:prod-dep"
  },
  "packageManager": "bun@1.4.0"
}
```
