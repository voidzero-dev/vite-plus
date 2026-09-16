# windows_case_sensitive_shims

## `node setup.cjs`


## `vp run probe`

Nested task planning resolves the lowercase shim with uppercase PATHEXT.

```
~/packages/app$ astro --version ⊘ cache disabled
local shim --version
```

## `cd packages/app && vp exec astro --version`

```
local shim --version
```

## `vpt write-file node_modules/.bin/astro.CMD "@echo wrong workspace shim %*"`


## `vp run probe`

The earlier package-local lowercase shim wins over a later uppercase shim.

```
~/packages/app$ astro --version ⊘ cache disabled
local shim --version
```

## `cd packages/app && vp exec astro --version`

```
local shim --version
```

## `cd packages/app && vp run cached`

```
~/packages/app$ astro cached
local shim cached
```

## `cd packages/app && vp run cached`

Resolving the program keeps the task's cache behavior.

```
~/packages/app$ astro cached ◉ cache hit, replaying
local shim cached

---
vp run: cache hit, <duration> saved.
```
