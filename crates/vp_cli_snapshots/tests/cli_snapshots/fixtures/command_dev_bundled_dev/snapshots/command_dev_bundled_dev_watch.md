# command_dev_bundled_dev_watch

Bundled dev forwards polling and debounce options to the native dev engine.

## `npm install --ignore-scripts --legacy-peer-deps --no-audit --no-fund`


## `node verify-watch.mjs`

```
HTTP status: 200
Requested watch options: {"usePolling":true,"interval":1234,"compareContentsForPolling":true,"useDebounce":true,"debounceDuration":250,"debounceTickRate":50}
Native watch options: [{"skipWrite":true,"usePolling":true,"pollInterval":1234,"useDebounce":true,"debounceDuration":250,"compareContentsForPolling":true,"debounceTickRate":50}]
```
