# bun_recursive_update_respects_filter

## `vpt write-file package.json '{"name":"root","private":true,"packageManager":"bun@1.4.0","workspaces":["packages/*"],"dependencies":{"is-number":"6.0.0"}}'`


## `vpt write-file packages/app/package.json '{"name":"app","version":"1.0.0","dependencies":{"is-number":"6.0.0"}}'`


## `vpt mkdir packages/utils`


## `vpt write-file packages/utils/package.json '{"name":"utils","version":"1.0.0","dependencies":{"is-number":"6.0.0"}}'`


## `vp install --ignore-scripts`


## `cd packages/app && vp update --recursive --filter utils is-number@7.0.0 -- --ignore-scripts`

an explicit filter limits recursive updates to the selected workspace

```
bun update <version> (<hash>)

↑ is-number 6.0.0 → 7.0.0

1 package installed [<duration>]
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json`

```
{"name":"root","private":true,"packageManager":"bun@1.4.0","workspaces":["packages/*"],"dependencies":{"is-number":"6.0.0"}}{"name":"app","version":"1.0.0","dependencies":{"is-number":"6.0.0"}}{
  "name": "utils",
  "version": "1.0.0",
  "dependencies": {
    "is-number": "7.0.0"
  }
}
```

## `vpt print-file bun.lock`

```
{
  "lockfileVersion": 2,
  "configVersion": 1,
  "workspaces": {
    "": {
      "name": "root",
      "dependencies": {
        "is-number": "6.0.0",
      },
    },
    "packages/app": {
      "name": "app",
      "version": "1.0.0",
      "dependencies": {
        "is-number": "6.0.0",
      },
    },
    "packages/utils": {
      "name": "utils",
      "version": "1.0.0",
      "dependencies": {
        "is-number": "7.0.0",
      },
    },
  },
  "packages": {
    "app": ["app@workspace:packages/app"],

    "is-number": ["is-number@6.0.0", "http://127.0.0.1:<port>/is-number/-/is-number-6.0.0.tgz", {}, "sha512-Wu1VHeILBK8KAWJUAiSZQX94GmOE45Rg6/538fKwiloUu21KncEkYGPqob2oSZ5mUT73vLGrHQjKw3KMPwfDzg=="],

    "utils": ["utils@workspace:packages/utils"],

    "utils/is-number": ["is-number@7.0.0", "", {}, "sha512-41Cifkg6e8TylSpdtTpeLVMqvSBEVzTttHvERD741+pnZ8ANv0004MRL43QKPDlK9cGvNp6NZWZUBlbGXYxxng=="],
  }
}
```
