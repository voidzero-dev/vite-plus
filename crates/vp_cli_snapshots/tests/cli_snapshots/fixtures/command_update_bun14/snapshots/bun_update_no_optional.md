# bun_update_no_optional

## `vp install --ignore-scripts`


## `vpt json-edit package.json dependencies.is-number '>=6.0.0 <=7.0.0'`


## `vpt json-edit package.json devDependencies.yocto-queue '>=0.1.0 <=1.0.0'`


## `vpt json-edit package.json optionalDependencies.isarray '>=1.0.0 <=2.0.0'`


## `vp update --no-optional -- --ignore-scripts`

production and dev dependencies advance while optional dependencies remain unchanged

```
bun update <version> (<hash>)

↑ yocto-queue 0.1.0 → 1.0.0 (<version> available)
↑ is-number 6.0.0 → 7.0.0

2 packages installed [<duration>]
```

## `vpt print-file package.json`

```
{
  "dependencies": {
    "is-number": ">=6.0.0 <=7.0.0"
  },
  "devDependencies": {
    "yocto-queue": ">=0.1.0 <=1.0.0"
  },
  "name": "command-update-bun14",
  "optionalDependencies": {
    "isarray": ">=1.0.0 <=2.0.0"
  },
  "packageManager": "bun@1.4.0",
  "private": true,
  "version": "1.0.0"
}
```

## `vpt print-file bun.lock`

```
{
  "lockfileVersion": 2,
  "configVersion": 1,
  "workspaces": {
    "": {
      "name": "command-update-bun14",
      "dependencies": {
        "is-number": ">=6.0.0 <=7.0.0",
      },
      "devDependencies": {
        "yocto-queue": ">=0.1.0 <=1.0.0",
      },
      "optionalDependencies": {
        "isarray": ">=1.0.0 <=2.0.0",
      },
    },
  },
  "packages": {
    "is-number": ["is-number@7.0.0", "", {}, "sha512-41Cifkg6e8TylSpdtTpeLVMqvSBEVzTttHvERD741+pnZ8ANv0004MRL43QKPDlK9cGvNp6NZWZUBlbGXYxxng=="],

    "isarray": ["isarray@1.0.0", "http://127.0.0.1:<port>/isarray/-/isarray-1.0.0.tgz", {}, "sha512-VLghIWNM6ELQzo7zwmcg0NmTVyWKYjvIeM83yjp0wRDTmUnrM678fQbcKBo6n2CJEF0szoG//ytg+TKla89ALQ=="],

    "yocto-queue": ["yocto-queue@1.0.0", "", {}, "sha512-9bnSc/HEW2uRy67wc+T8UwauLuPJVn28jb+GtJY16iiKWyvmYJRXVT4UamsAEGQfPohgr2q4Tq0sQbQlxTfi1g=="],
  }
}
```
