# npm_install_includes_dev

## `vpt json-edit package.json dependencies.prod-only-fixture file:./prod-dep`


## `vpt json-edit package.json devDependencies.dev-only-fixture file:./dev-dep`


## `vp install --dev --ignore-scripts`

include dev dependencies despite NODE_ENV=production without excluding production dependencies

```
VITE+ - The Unified Toolchain for the Web

added 2 packages in <duration>
```

## `vpt stat-file node_modules/prod-only-fixture/package.json --assert file`

```
node_modules/prod-only-fixture/package.json: file
```

## `vpt stat-file node_modules/dev-only-fixture/package.json --assert file`

```
node_modules/dev-only-fixture/package.json: file
```

## `vpt write-file node_modules/preserved.txt keep`


## `vp install --dev --frozen-lockfile --ignore-scripts --silent`

npm ci also includes dev dependencies and performs its native cleanup

```
```

## `vpt stat-file node_modules/preserved.txt --assert missing`

```
node_modules/preserved.txt: missing
```

## `vpt stat-file node_modules/dev-only-fixture/package.json --assert file`

```
node_modules/dev-only-fixture/package.json: file
```

## `vpt print-file package.json package-lock.json`

```
{
  "dependencies": {
    "prod-only-fixture": "file:./prod-dep"
  },
  "devDependencies": {
    "dev-only-fixture": "file:./dev-dep"
  },
  "name": "command-install-npm11",
  "packageManager": "npm@11.16.0",
  "private": true,
  "version": "1.0.0"
}
{
  "name": "command-install-npm11",
  "version": "1.0.0",
  "lockfileVersion": 3,
  "requires": true,
  "packages": {
    "": {
      "name": "command-install-npm11",
      "version": "1.0.0",
      "dependencies": {
        "prod-only-fixture": "file:./prod-dep"
      },
      "devDependencies": {
        "dev-only-fixture": "file:./dev-dep"
      }
    },
    "dev-dep": {
      "name": "dev-only-fixture",
      "version": "1.0.0",
      "dev": true
    },
    "node_modules/dev-only-fixture": {
      "resolved": "dev-dep",
      "link": true
    },
    "node_modules/prod-only-fixture": {
      "resolved": "prod-dep",
      "link": true
    },
    "prod-dep": {
      "name": "prod-only-fixture",
      "version": "1.0.0"
    }
  }
}
```
