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

error: Command 'not-exist-command' not found

Did you mean `vp test`?
```
