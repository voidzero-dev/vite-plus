# configured_reporter

## `vpt write-file .npmrc 'reporter=silent
loglevel=warn
json=true
parseable=true
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`


## `vpt print-file postinstall-config.jsonl .npmrc`

Both installs preserve the project's reporting settings in postinstall and on disk.

```
{"reporter":"silent","loglevel":"warn","json":"true","parseable":"true"}
{"reporter":"silent","loglevel":"warn","json":"true","parseable":"true"}
reporter=silent
loglevel=warn
json=true
parseable=true
```
