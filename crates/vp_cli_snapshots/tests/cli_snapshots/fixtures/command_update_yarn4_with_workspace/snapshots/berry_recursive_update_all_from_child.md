# berry_recursive_update_all_from_child

## `vpt write-file package.json '{"name":"root","private":true,"packageManager":"yarn@4.10.3","workspaces":["packages/*"],"dependencies":{"is-number":"6.0.0"}}'`


## `vpt write-file packages/app/package.json '{"name":"app","version":"1.0.0","dependencies":{"is-number":"6.0.0"}}'`


## `vpt write-file packages/utils/package.json '{"name":"utils","version":"1.0.0","dependencies":{"is-number":"6.0.0"}}'`


## `vpt write-file .yarnrc.yml 'nodeLinker: node-modules
'`


## `vp install`


## `cd packages/app && vp update --recursive --latest`

omitting package names still updates all workspace dependencies

```
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0085: │ + is-number@npm:7.0.0::__archiveUrl=https%3A%2F%2Fregistry.npmjs.org%2Fis-number%2F-%2Fis-number-7.0.0.tgz
➤ YN0085: │ - is-number@npm:6.0.0::__archiveUrl=https%3A%2F%2Fregistry.npmjs.org%2Fis-number%2F-%2Fis-number-6.0.0.tgz
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0013: │ A package was added to the project (+ <size> KiB).
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration>
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json`

```
{
  "name": "root",
  "private": true,
  "packageManager": "yarn@4.10.3",
  "workspaces": [
    "packages/*"
  ],
  "dependencies": {
    "is-number": "^7.0.0"
  }
}
{
  "name": "app",
  "version": "1.0.0",
  "dependencies": {
    "is-number": "^7.0.0"
  }
}
{
  "name": "utils",
  "version": "1.0.0",
  "dependencies": {
    "is-number": "^7.0.0"
  }
}
```
