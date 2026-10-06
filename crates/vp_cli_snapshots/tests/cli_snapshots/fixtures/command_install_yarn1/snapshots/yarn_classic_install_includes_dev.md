# yarn_classic_install_includes_dev

## `vpt json-edit package.json devDependencies.dev-only-fixture file:./dev-dep`


## `vp install -D`

include dev dependencies despite NODE_ENV=production without excluding production dependencies

```
VITE+ - The Unified Toolchain for the Web

yarn install <version>
info No lockfile found.
[1/4] Resolving packages...
[2/4] Fetching packages...
[3/4] Linking dependencies...
[4/4] Building fresh packages...

success Saved lockfile.

Done in <duration>.
```

## `vpt stat-file node_modules/prod-only-fixture/package.json --assert file`

```
node_modules/prod-only-fixture/package.json: file
```

## `vpt stat-file node_modules/dev-only-fixture/package.json --assert file`

```
node_modules/dev-only-fixture/package.json: file
```

## `vp install --dev --frozen-lockfile`

preserve dev inclusion in frozen-lockfile mode too

```
VITE+ - The Unified Toolchain for the Web

yarn install <version>
[1/4] Resolving packages...
success Already up-to-date.

Done in <duration>.
```

## `vpt print-file package.json`

```
{
  "dependencies": {
    "prod-only-fixture": "file:prod-dep"
  },
  "devDependencies": {
    "dev-only-fixture": "file:./dev-dep"
  },
  "name": "command-install-yarn1",
  "packageManager": "yarn@1.22.22",
  "private": true,
  "version": "1.0.0"
}
```
