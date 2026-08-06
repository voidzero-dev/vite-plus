# cache_clean

## `vp run hello`

create the cache for 'echo hello'

```
$ node -e "process.stdout.write('hello')"
hello
```

## `vp run hello`

hit the cache

```
$ node -e "process.stdout.write('hello')" ◉ cache hit, replaying
hello
---
vp run: cache hit, <duration> saved.
```

## `vp cache clean`

clean the cache

```
```

## `vp run hello`

cache miss after clean

```
$ node -e "process.stdout.write('hello')"
hello
```

## `cd subfolder && vp cache clean`

cache can be located and cleaned from subfolder

```
```

## `vp run hello`

cache miss after clean

```
$ node -e "process.stdout.write('hello')"
hello
```
