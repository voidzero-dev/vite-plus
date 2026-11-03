# bun_remove_workspace_scope

## `vpt write-file package.json '{"name":"root","private":true,"packageManager":"bun@1.4.0","workspaces":["packages/*"],"dependencies":{"testnpm2":"1.0.0"}}'`


## `vpt write-file packages/app/package.json '{"name":"app","version":"1.0.0","dependencies":{"testnpm2":"1.0.0"}}'`


## `vpt mkdir packages/utils`


## `vpt write-file packages/utils/package.json '{"name":"utils","version":"1.0.0","dependencies":{"testnpm2":"1.0.0"}}'`


## `vp install --ignore-scripts`


## `cd packages/app && vp remove --filter app --filter utils testnpm2 -- --ignore-scripts`

preserve native filtering without recursive removal

```
bun remove <version> (<hash>)

- testnpm2
[<duration>] done
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json bun.lock`

```
{"name":"root","private":true,"packageManager":"bun@1.4.0","workspaces":["packages/*"],"dependencies":{"testnpm2":"1.0.0"}}{
  "name": "app",
  "version": "1.0.0"
}{
  "name": "utils",
  "version": "1.0.0"
}{
  "lockfileVersion": 2,
  "configVersion": 1,
  "workspaces": {
    "": {
      "name": "root",
      "dependencies": {
        "testnpm2": "1.0.0",
      },
    },
    "packages/app": {
      "name": "app",
      "version": "1.0.0",
    },
    "packages/utils": {
      "name": "utils",
      "version": "1.0.0",
    },
  },
  "packages": {
    "app": ["app@workspace:packages/app"],

    "testnpm2": ["testnpm2@1.0.0", "http://127.0.0.1:<port>/testnpm2/-/testnpm2-1.0.0.tgz", {}, "sha512-8gdtqxKad+83Iog2v514VsHsSk/R+we9j5/9zX9tB+QC2ubvB06zJ08k0PSl5uzviXByEMiWm7EzSbBAh2GZ/w=="],

    "utils": ["utils@workspace:packages/utils"],
  }
}
```

## `node -e 'require('\''node:assert/strict'\'').equal(require('\''testnpm2/package.json'\'').version, '\''1.0.0'\'')'`


## `cd packages/app && vp remove --filter root testnpm2 -- --ignore-scripts`

an explicit root name also works from a child directory

```
bun remove <version> (<hash>)

- testnpm2
1 package removed [<duration>]
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json`

```
{
  "name": "root",
  "packageManager": "bun@1.4.0",
  "private": true,
  "workspaces": ["packages/*"]
}{
  "name": "app",
  "version": "1.0.0"
}{
  "name": "utils",
  "version": "1.0.0"
}
```

## `vpt stat-file node_modules/testnpm2 --assert missing`

```
node_modules/testnpm2: missing
```
