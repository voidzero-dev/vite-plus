# cache_scripts_default

## `vp run hello`

cache should be disabled by default for package.json scripts

```
$ node hello.mjs ⊘ cache disabled
hello from script
```

## `vp run hello`

second run should also show cache disabled

```
$ node hello.mjs ⊘ cache disabled
hello from script
```
