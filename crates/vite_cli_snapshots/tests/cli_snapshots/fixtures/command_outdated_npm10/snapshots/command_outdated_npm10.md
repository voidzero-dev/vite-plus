# command_outdated_npm10

## `vp install`

should install packages first

```
VITE+ - The Unified Toolchain for the Web

added 4 packages, and audited 5 packages in <duration>

found 0 vulnerabilities
```

## `vp outdated testnpm2`

should outdated package

**Exit code:** 1

```
Package   Current  Wanted  Latest  Location               Depended by
testnpm2    1.0.0   1.0.0   1.0.1  node_modules/testnpm2  workspace
```

## `vp outdated test-vite*`

should outdated with glob pattern not working on npm

```
```

## `vp outdated --format json`

should support json output

**Exit code:** 1

```
{
  "test-vite-plus-other-optional": {
    "current": "1.0.0",
    "wanted": "1.0.0",
    "latest": "1.1.0",
    "dependent": "workspace",
    "location": "<workspace>/node_modules/test-vite-plus-other-optional"
  },
  "test-vite-plus-top-package": {
    "current": "1.0.0",
    "wanted": "1.0.0",
    "latest": "1.1.0",
    "dependent": "workspace",
    "location": "<workspace>/node_modules/test-vite-plus-top-package"
  },
  "testnpm2": {
    "current": "1.0.0",
    "wanted": "1.0.0",
    "latest": "1.0.1",
    "dependent": "workspace",
    "location": "<workspace>/node_modules/testnpm2"
  }
}
```

## `vp outdated --format list`

should support list output

**Exit code:** 1

```
<workspace>/node_modules/test-vite-plus-other-optional:test-vite-plus-other-optional@1.0.0:test-vite-plus-other-optional@1.0.0:test-vite-plus-other-optional@1.1.0:workspace
<workspace>/node_modules/test-vite-plus-top-package:test-vite-plus-top-package@1.0.0:test-vite-plus-top-package@1.0.0:test-vite-plus-top-package@1.1.0:workspace
<workspace>/node_modules/testnpm2:testnpm2@1.0.0:testnpm2@1.0.0:testnpm2@1.0.1:workspace
```

## `vp outdated --format table`

should support table output

**Exit code:** 1

```
Package                        Current  Wanted  Latest  Location                                    Depended by
test-vite-plus-other-optional    1.0.0   1.0.0   1.1.0  node_modules/test-vite-plus-other-optional  workspace
test-vite-plus-top-package       1.0.0   1.0.0   1.1.0  node_modules/test-vite-plus-top-package     workspace
testnpm2                         1.0.0   1.0.0   1.0.1  node_modules/testnpm2                       workspace
```

## `vp outdated testnpm2 --long`

should support --long

**Exit code:** 1

```
Package   Current  Wanted  Latest  Location               Depended by  Package Type  Homepage
testnpm2    1.0.0   1.0.0   1.0.1  node_modules/testnpm2  workspace    dependencies
```

## `vp outdated -r`

should support recursive output

**Exit code:** 1

```
Package                        Current  Wanted  Latest  Location                                    Depended by
test-vite-plus-other-optional    1.0.0   1.0.0   1.1.0  node_modules/test-vite-plus-other-optional  workspace
test-vite-plus-top-package       1.0.0   1.0.0   1.1.0  node_modules/test-vite-plus-top-package     workspace
testnpm2                         1.0.0   1.0.0   1.0.1  node_modules/testnpm2                       workspace
```

## `vp outdated -P`

should support prod output

**Exit code:** 1

```
warn: --prod/--dev not supported by npm
Package                        Current  Wanted  Latest  Location                                    Depended by
test-vite-plus-other-optional    1.0.0   1.0.0   1.1.0  node_modules/test-vite-plus-other-optional  workspace
test-vite-plus-top-package       1.0.0   1.0.0   1.1.0  node_modules/test-vite-plus-top-package     workspace
testnpm2                         1.0.0   1.0.0   1.0.1  node_modules/testnpm2                       workspace
```

## `vp outdated -D`

should support dev output

**Exit code:** 1

```
warn: --prod/--dev not supported by npm
Package                        Current  Wanted  Latest  Location                                    Depended by
test-vite-plus-other-optional    1.0.0   1.0.0   1.1.0  node_modules/test-vite-plus-other-optional  workspace
test-vite-plus-top-package       1.0.0   1.0.0   1.1.0  node_modules/test-vite-plus-top-package     workspace
testnpm2                         1.0.0   1.0.0   1.0.1  node_modules/testnpm2                       workspace
```

## `vp outdated --no-optional`

should support no-optional output

**Exit code:** 1

```
warn: --no-optional not supported by npm
Package                        Current  Wanted  Latest  Location                                    Depended by
test-vite-plus-other-optional    1.0.0   1.0.0   1.1.0  node_modules/test-vite-plus-other-optional  workspace
test-vite-plus-top-package       1.0.0   1.0.0   1.1.0  node_modules/test-vite-plus-top-package     workspace
testnpm2                         1.0.0   1.0.0   1.0.1  node_modules/testnpm2                       workspace
```

## `vp outdated --compatible`

should compatible output nothing

**Exit code:** 1

```
warn: --compatible not supported by npm
Package                        Current  Wanted  Latest  Location                                    Depended by
test-vite-plus-other-optional    1.0.0   1.0.0   1.1.0  node_modules/test-vite-plus-other-optional  workspace
test-vite-plus-top-package       1.0.0   1.0.0   1.1.0  node_modules/test-vite-plus-top-package     workspace
testnpm2                         1.0.0   1.0.0   1.0.1  node_modules/testnpm2                       workspace
```

## `vpt json-edit package.json optionalDependencies.test-vite-plus-other-optional '"^1.0.0"'`

should support compatible output with optional dependencies


## `vp outdated --compatible`

**Exit code:** 1

```
warn: --compatible not supported by npm
Package                        Current  Wanted  Latest  Location                                    Depended by
test-vite-plus-other-optional    1.0.0   1.1.0   1.1.0  node_modules/test-vite-plus-other-optional  workspace
test-vite-plus-top-package       1.0.0   1.0.0   1.1.0  node_modules/test-vite-plus-top-package     workspace
testnpm2                         1.0.0   1.0.0   1.0.1  node_modules/testnpm2                       workspace
```

## `vp outdated --sort-by name`

should support sort-by output

**Exit code:** 1

```
warn: --sort-by not supported by npm
Package                        Current  Wanted  Latest  Location                                    Depended by
test-vite-plus-other-optional    1.0.0   1.1.0   1.1.0  node_modules/test-vite-plus-other-optional  workspace
test-vite-plus-top-package       1.0.0   1.0.0   1.1.0  node_modules/test-vite-plus-top-package     workspace
testnpm2                         1.0.0   1.0.0   1.0.1  node_modules/testnpm2                       workspace
```
