# run_rewrites_managed_pnpm

## `node prepare.cjs pnpm`


## `vp run probe`

Task planning also rewrites managed shims outside the workspace.

```
$ pnpm --flag "value with spaces" 'path/file' ⊘ cache disabled
launcher=ps1
arguments, environment, cwd, and Node selection preserved
```

## `vp run probe`

Piped task stdin retains the batch launcher.

```
$ pnpm --flag "value with spaces" 'path/file' ⊘ cache disabled
launcher=cmd
arguments, environment, cwd, and Node selection preserved
```

## `SHIM_TEST_EXIT_CODE=7 vp run probe`

**Exit code:** 7

```
$ pnpm --flag "value with spaces" 'path/file' ⊘ cache disabled
launcher=ps1
arguments, environment, cwd, and Node selection preserved
```

## `node prepare.cjs pnpm alias`


## `vpt rm $VP_HOME/package_manager/pnpm/10.18.0/pnpm/bin/pnpx.ps1`


## `vp run probe`

A missing PowerShell sibling retains the batch launcher.

```
$ pnpx --flag "value with spaces" 'path/file' ⊘ cache disabled
launcher=cmd
arguments, environment, cwd, and Node selection preserved
```
