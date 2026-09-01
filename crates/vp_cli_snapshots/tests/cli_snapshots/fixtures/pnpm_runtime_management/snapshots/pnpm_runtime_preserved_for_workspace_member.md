# pnpm_runtime_preserved_for_workspace_member

A runtime required by any workspace member keeps pnpm runtime management enabled.

## `vpt write-file package.json '{"name":"pnpm-runtime-management","private":true,"packageManager":"pnpm@11.1.0","devEngines":{"runtime":{"name":"node","version":"20.18.0","onFail":"download"}}}
'`


## `vpt write-file pnpm-workspace.yaml 'packages:
  - packages/*
'`


## `vpt mkdir packages/member`

**Exit code:** 1

```
No such file or directory (os error 2)
```

*(remaining steps skipped: step failed)*
