# command_install_auto_create_package_json

## `vpt stat-file package.json --assert-not file`

verify no package.json exists

```
package.json: missing
```

## `vp install --silent`

should auto-create package.json and install

```
```

## `vpt print-file package.json`

```
{
  "type": "module",
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `vp add testnpm2 -D`

should add package to auto-created package.json

```
✓ Lockfile passes supply-chain policies (verified <duration> ago)

devDependencies:
 testnpm2 1.0.1

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json`

```
{
  "type": "module",
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  },
  "devDependencies": {
    "testnpm2": "^1.0.1"
  }
}
```
