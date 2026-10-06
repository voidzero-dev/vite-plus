# bun_update_prod_includes_optional

## `vp install --ignore-scripts`


## `vpt json-edit package.json dependencies.is-number '>=6.0.0 <=7.0.0'`


## `vpt json-edit package.json devDependencies.yocto-queue '>=0.1.0 <=1.0.0'`


## `vpt json-edit package.json optionalDependencies.isarray '>=1.0.0 <=2.0.0'`


## `vp update -P -- --ignore-scripts`

Bun updates production and optional dependencies while dev dependencies remain unchanged

```
bun update <version> (<hash>)

↑ isarray 1.0.0 → 2.0.0 (<version> available)
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

    "isarray": ["isarray@2.0.0", "", {}, "sha512-+7GtYSNjbLkjIGQy5ucmTjyY9ANbbW77DOT6nbQN4ZDo3CnST3HtjnGowbi+NF2pCPCba1rLO9qLlrFBsvDeew=="],

    "yocto-queue": ["yocto-queue@0.1.0", "http://127.0.0.1:<port>/yocto-queue/-/yocto-queue-0.1.0.tgz", {}, "sha512-rVksvsnNCdJ/ohGc6xgPwyN8eheCxsiLM8mxuE/t/mOVqJewPuO1miLpTHQiRgTKCLexL4MeAFVagts7HmNZ2Q=="],
  }
}
```
