# command_run_dynamic_config_stdin

Loading a non-static run config through the JS resolver leaves the stdin that tasks inherit in blocking mode (#3155).

## `vp run probe`

terminal stdin

```
$ vpt check-blocking ⊘ cache disabled
stdin:blocking
```

## `vpt pipe-stdin x -- vp run probe`

piped stdin

```
$ vpt check-blocking ⊘ cache disabled
stdin:blocking
```
