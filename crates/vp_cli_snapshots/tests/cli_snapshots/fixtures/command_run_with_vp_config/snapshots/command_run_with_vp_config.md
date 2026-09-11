# command_run_with_vp_config

## `vp run foo`

should run vp config command

```
$ vp config ⊘ cache disabled
.git can't be found
```

## `vp run bar`

should throw error

**Exit code:** 2

```
$ vp not-exist-command ⊘ cache disabled

[1m[31merror:[39m[0m Command '[94mnot-exist-command[39m' not found

Did you mean [94m`vp test`[39m?
```
