# command_install_bug_31

## `vp install --no-frozen-lockfile --silent`

install dependencies work

```
```

## `vpt mkdir -p packages/sub-project`

create sub project and package.json

```
```

## `vpt write-file packages/sub-project/package.json '{"name": "sub-project", "dependencies": { "testnpm2": "1.0.0" }}
'`

```
```

## `vp install --no-frozen-lockfile --silent`

install again should work and without cache

```
```

## `vpt list-dir packages/sub-project/node_modules/testnpm2/package.json`

check testnpm2 is installed

```
packages/sub-project/node_modules/testnpm2/package.json
```

## `vpt mkdir -p others/other`

create non workspace project

```
```

## `vpt write-file others/other/package.json '{"name": "other", "dependencies": { "testnpm2": "1.0.0" }}
'`

```
```

## `vp install --no-frozen-lockfile --silent`

should install cache hit

```
```

## `vpt stat-file others/other/node_modules/testnpm2 --assert-not dir`

the directory must not exist

```
others/other/node_modules/testnpm2: missing
```

## `vpt rm -rf packages/sub-project`

remove sub project

```
```

## `vp install --no-frozen-lockfile --silent`

should install again without cache

```
```
