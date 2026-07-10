# explicit_serial

Exercises the explicit `serial = true` isolation opt-in (the non-ctrl-c path):
the case takes the exclusive execution lease and runs to completion, proving
the flag parses and the isolated path works.

## `vpt print 'runs in isolation'`

```
runs in isolation
```
