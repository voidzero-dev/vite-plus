# ignore_dist

## `vp run lint`

```
$ node -e "console.log('lint')"
lint
```

## `vpt mkdir dist`

```
```

## `vp run lint`

new dist folder should not invalidate cache

```
$ node -e "console.log('lint')" ◉ cache hit, replaying
lint

---
vp run: cache hit, <duration> saved.
```
