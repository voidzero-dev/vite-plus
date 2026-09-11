# exit_code

## `vp run script1`

script1 run, create the cache and should be success

```
$ echo 'success' ⊘ cache disabled
success
```

## `vp run script1`

script1 should hit the updated cache

```
$ echo 'success' ⊘ cache disabled
success
```

## `vp run script2`

script2 should be failure and not cache

**Exit code:** 1

```
$ node failure.js
failure
```

## `vp run script2`

script2 should be failure and not cache

**Exit code:** 1

```
$ node failure.js
failure
```
